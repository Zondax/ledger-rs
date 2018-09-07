/*******************************************************************************
*   (c) 2018 ZondaX GmbH
*
*  Licensed under the Apache License, Version 2.0 (the "License");
*  you may not use this file except in compliance with the License.
*  You may obtain a copy of the License at
*
*      http://www.apache.org/licenses/LICENSE-2.0
*
*  Unless required by applicable law or agreed to in writing, software
*  distributed under the License is distributed on an "AS IS" BASIS,
*  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
*  See the License for the specific language governing permissions and
*  limitations under the License.
********************************************************************************/
#[macro_use]
extern crate quick_error;
extern crate hidapi;
extern crate byteorder;

#[cfg(target_os = "linux")]
#[macro_use]
extern crate nix;

#[cfg(target_os = "linux")]
extern crate libc;

#[cfg(target_os = "linux")]
use std::{ffi::CStr, mem};
use std::{
    ffi::CString,
    io::Cursor,
};

use byteorder::{BigEndian, ReadBytesExt};
use hidapi::HidDevice;

const LEDGER_VID: u16 = 0x2c97;
const LEDGER_USAGE_PAGE: u16 = 0xFFA0;
const LEDGER_CHANNEL: u16 = 0x0101;
const LEDGER_PACKET_SIZE: u8 = 64;

const LEDGER_TIMEOUT: i32 = 10000000;

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        DeviceNotFound{
            description("Could not find a ledger device")
        }
        // TODO: Extend error messages to handle APDU error codes
        Comm(descr: &'static str) {
            description(descr)
            display("Communication Error: {}", descr)
        }
        Io ( err: std::io::Error ) {
            from()
            description("io error")
            display("I/O error: {}", err)
            cause(err)
        }
        Hid ( err: hidapi::HidError ) {
            from()
            description("hid error")
            display("hid error: {}", err)
            // cause(err)
        }
        Ioctl ( err: nix::Error ) {
            from()
            description("ioctl error")
            display("ioctl error: {}", err)
            cause(err)
        }
        Unexpected ( err: std::str::Utf8Error ) {
            from()
            description("unexpected error")
            display("unexpected error: {}", err)
            cause(err)
        }
    }
}

const HID_MAX_DESCRIPTOR_SIZE: usize = 4096;

#[repr(C)]
pub struct HidrawReportDescriptor {
    size: u32,
    value: [u8; HID_MAX_DESCRIPTOR_SIZE],
}

#[derive(Debug)]
pub struct ApduCommand {
    pub cla: u8,
    pub ins: u8,
    pub p1: u8,
    pub p2: u8,
    pub length: u8,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct ApduAnswer {
    pub data: Vec<u8>,
    pub retcode: u16,
}

impl ApduCommand {
    fn serialize(&self) -> Vec<u8>
    {
        let mut v = vec![self.cla, self.ins, self.p1, self.p2, self.length];
        v.extend(&self.data);
        v
    }
}

#[cfg(not(target_os = "linux"))]
fn find_ledger_device_path() -> Result<CString, LedgerError>
{
    extern crate hidapi;

    let api = hidapi::HidApi::new()?;
    for device in api.devices() {
        if device.vendor_id == LEDGER_VID && device.usage_page == LEDGER_USAGE_PAGE {
            return Ok(device.path.clone());
        }
    }

    Err(Box::from("Could not find Ledger Nano S device"))
}


#[cfg(target_os = "linux")]
fn find_ledger_device_path() -> Result<CString, Error>
{
    extern crate hidapi;

    let api = hidapi::HidApi::new()?;
    for device in api.devices() {
        if device.vendor_id == LEDGER_VID {
            let usage_page = get_usage_page(&device.path)?;
            if usage_page == LEDGER_USAGE_PAGE {
                return Ok(device.path.clone());
            }
        }
    }
    Err(Error::DeviceNotFound)
}

#[cfg(target_os = "linux")]
fn get_usage_page(device_path: &CStr) -> Result<u16, Error>
{
    // #define HIDIOCGRDESCSIZE	_IOR('H', 0x01, int)
    // #define HIDIOCGRDESC		_IOR('H', 0x02, struct HidrawReportDescriptor)
    ioctl_read!(hid_read_descr_size, b'H', 0x01, libc::c_int);
    ioctl_read!(hid_read_descr, b'H', 0x02, HidrawReportDescriptor);

    use std::os::unix::{fs::OpenOptionsExt, io::AsRawFd};
    use std::fs::OpenOptions;

    let file_name = device_path.to_str()?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(file_name)?;

    let mut desc_size = 0;

    unsafe {
        let fd = file.as_raw_fd();
        let mut desc_raw: HidrawReportDescriptor = mem::uninitialized();

        hid_read_descr_size(fd, &mut desc_size)?;
        desc_raw.size = desc_size as u32;
        hid_read_descr(fd, &mut desc_raw)?;

        let data = &desc_raw.value[..desc_raw.size as usize];

        let mut data_len;
        let mut key_size;
        let mut i = 0 as usize;

        while i < desc_size as usize {
            let key = data[i];
            let key_cmd = key & 0xFC;

            if key & 0xF0 == 0xF0 {
                data_len = 0;
                key_size = 3;
                if i + 1 < desc_size as usize {
                    data_len = data[i + 1];
                }
            } else {
                key_size = 1;
                data_len = key & 0x03;
                if data_len == 3 {
                    data_len = 4;
                }
            }

            if key_cmd == 0x04 {
                // println!("{:02x?} {:02x?} {:02x?}", data, data_len, i);
                let usage_page = match data_len {
                    1 => data[i + 1] as u16,
                    2 => (data[i + 2] as u16 * 256 + data[i + 1] as u16),
                    _ => 0 as u16
                };

                return Ok(usage_page);
            }

            i += (data_len + key_size) as usize;
        }
    }
    Ok(0)
}

fn write_apdu(device: &HidDevice, channel: u16, apdu_command: &[u8]) -> Result<i32, Error>
{
    let command_length = apdu_command.len() as usize;
    let mut in_data = Vec::with_capacity(command_length + 2);
    in_data.push(((command_length >> 8) & 0xFF) as u8);
    in_data.push(((command_length >> 0) & 0xFF) as u8);
    in_data.extend_from_slice(&apdu_command);

    let mut buffer = vec![0u8; LEDGER_PACKET_SIZE as usize];
    buffer[0] = ((channel >> 8) & 0xFF) as u8;         // channel big endian
    buffer[1] = ((channel >> 0) & 0xFF) as u8;         // channel big endian
    buffer[2] = 0x05u8;

    let mut sequence_idx = 0u16;

    for chunk in in_data.chunks((LEDGER_PACKET_SIZE - 5) as usize)
        {
            buffer[3] = ((sequence_idx >> 8) & 0xFF) as u8;         // sequence_idx big endian
            buffer[4] = ((sequence_idx >> 0) & 0xFF) as u8;         // sequence_idx big endian
            buffer[5..5 + chunk.len()].copy_from_slice(chunk);

            let result = device.write(&buffer);

            match result
                {
                    Ok(size) => if size < buffer.len() {
//                        println!("{:#?}", size);
//                        println!("{:#?}", buffer.len());
                        return Err(Error::Comm("USB write error. Could not send whole message"));
                    },
                    Err(x) => return Err(Error::Hid(x))
                }

            sequence_idx += 1;
        }
    Ok(1)
}

fn read_apdu(device: &HidDevice, _channel: u16, apdu_answer: &mut Vec<u8>) -> Result<usize, Error>
{
    let mut buffer = vec![0u8; LEDGER_PACKET_SIZE as usize];
    let mut sequence_idx = 0u16;
    let mut expected_apdu_len = 0usize;

    loop {
        let res = device.read_timeout(&mut buffer, LEDGER_TIMEOUT)?;

        if (sequence_idx == 0 && res < 7) || res < 5 {
            return Err(Error::Comm("Read error. Incomplete header"));
        }

        let mut rdr = Cursor::new(&buffer);

        let _rcv_channel = rdr.read_u16::<BigEndian>()?;
        let _rcv_tag = rdr.read_u8()?;
        let rcv_seq_idx = rdr.read_u16::<BigEndian>()?;

        // TODO: Check why windows returns a different channel/tag
//        if rcv_channel != channel {
//            return Err(Box::from(format!("Invalid channel: {}!={}", rcv_channel, channel )));
//        }
//        if rcv_tag != 0x05u8 {
//            return Err(Box::from("Invalid tag"));
//        }

        if rcv_seq_idx != sequence_idx {
            return Err(Error::Comm("Invalid sequence idx"));
        }

        if rcv_seq_idx == 0 {
            expected_apdu_len = rdr.read_u16::<BigEndian>()? as usize;
        }

        let available: usize = buffer.len() - rdr.position() as usize;
        let missing: usize = expected_apdu_len - apdu_answer.len();
        let end_p = rdr.position() as usize + std::cmp::min(available, missing);

        apdu_answer.extend_from_slice(&buffer[rdr.position() as usize..end_p]);

        if apdu_answer.len() >= expected_apdu_len {
            return Ok(apdu_answer.len());
        }

        sequence_idx += 1;
    }
}

pub fn exchange(command: ApduCommand) -> Result<ApduAnswer, Error>
{
    extern crate hidapi;

    ////////////////////////////////
    ////////////////////////////////
    ////////////////////////////////
    // TODO: keep device in memory so reconnections are not necessary
    // Get device path
    let device_path = find_ledger_device_path()?;
    // println!("Device Path :\t{:?}", device_path);

    // Open device
    let api = hidapi::HidApi::new()?;
    let device = api.open_path(&device_path)?;
    ////////////////////////////////
    ////////////////////////////////
    ////////////////////////////////

    println!(">> {:X?}", &command.serialize());
    write_apdu(&device, LEDGER_CHANNEL,&command.serialize())?;

    let mut answer: Vec<u8> = Vec::with_capacity(256);
    let res = read_apdu(&device, LEDGER_CHANNEL, &mut answer)?;

    println!("<< {:X?}", answer);

    if res < 2 {
        return Err(Error::Comm("response was too short"));
    }

    let apdu_retcode = ((answer[answer.len() - 2] as u16) << 8) + answer[answer.len() - 1] as u16;
    let apdu_data = &answer[..answer.len() - 2];

    // TODO: match the return code and send back a cleaner enum

    Ok(ApduAnswer
        {
            data: apdu_data.to_vec(),
            retcode: apdu_retcode,
        }
    )
}

#[cfg(test)]
mod integration_tests {
    #[test]
    fn list_all_devices() {
        extern crate hidapi;

        let api = hidapi::HidApi::new()?;
        for device_info in api.devices() {
            println!("{:#?} - {:#x}/{:#x} {:#} {:#}",
                     device_info.path,
                     device_info.vendor_id,
                     device_info.usage_page,
                     device_info.manufacturer_string.clone().unwrap_or_default(),
                     device_info.product_string.clone().unwrap_or_default()
            );
        }
    }

    #[test]
    fn test_ledger_device_path() {
        use find_ledger_device_path;
        let ledger_path = find_ledger_device_path().unwrap();
        println!("{:?}", ledger_path);
    }

    #[test]
    fn test_serialize() {
        use ApduCommand;

        let data = vec![0, 0, 0, 1, 0, 0, 0, 1];

        let command = ApduCommand {
            cla: 0x56,
            ins: 0x01,
            p1: 0x00,
            p2: 0x00,
            length: data.len() as u8,
            data: data,
        };

        let serialized_command = command.serialize();

        let expected = vec![86, 1, 0, 0, 8, 0, 0, 0, 1, 0, 0, 0, 1];

        assert_eq!(serialized_command, expected)
    }

    #[test]
    fn test_exchange() {
        use exchange;
        use ApduCommand;

        let command = ApduCommand {
            cla: 0x56,
            ins: 0x00,
            p1: 0x00,
            p2: 0x00,
            length: 0,
            data: Vec::new(),
        };
        let result = exchange(command).unwrap();
        println!("{:?}", result);
    }
}
