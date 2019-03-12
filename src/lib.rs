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
#[macro_use]
extern crate cfg_if;
#[macro_use]
extern crate lazy_static;

extern crate byteorder;
extern crate hidapi;

cfg_if! {
    if #[cfg(target_os = "linux")] {
        #[macro_use]
        extern crate nix;
        extern crate libc;
        use std::{ffi::CStr, mem};
    } else {
        // Mock the type in other target_os
        mod nix {
            quick_error! {
                #[derive(Debug)]
                pub enum Error {
                }
            }
        }
    }
}

use std::{ffi::CString, io::Cursor};

use byteorder::{BigEndian, ReadBytesExt};
use hidapi::HidDevice;
use std::cell::RefCell;
use std::sync::{Arc, Mutex, Weak};

const LEDGER_VID: u16 = 0x2c97;
const LEDGER_USAGE_PAGE: u16 = 0xFFA0;
const LEDGER_CHANNEL: u16 = 0x0101;
const LEDGER_PACKET_SIZE: u8 = 64;

const LEDGER_TIMEOUT: i32 = 10_000_000;

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Ioctl ( err: nix::Error ) {
            from()
            description("ioctl error")
            display("ioctl error: {}", err)
            cause(err)
        }
        DeviceNotFound{
            description("Could not find a ledger device")
        }
        Comm(descr: &'static str) {
            description(descr)
            display("Communication Error: {}", descr)
        }
        Apdu(descr: &'static str) {
            description(descr)
            display("APDU: {}", descr)
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
        Unexpected ( err: std::str::Utf8Error ) {
            from()
            description("unexpected error")
            display("unexpected error: {}", err)
            cause(err)
        }
    }
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

pub struct HidApiWrapper {
    _api: RefCell<Weak<Mutex<hidapi::HidApi>>>,
}

#[allow(dead_code)]
pub struct LedgerApp {
    api_mutex: Arc<Mutex<hidapi::HidApi>>,
    device: HidDevice,
    device_mutex: Mutex<i32>,
    logging: bool,
}

unsafe impl Send for HidApiWrapper {}

lazy_static! {
    static ref HIDAPIWRAPPER: Arc<Mutex<HidApiWrapper>> =
        Arc::new(Mutex::new(HidApiWrapper::new()));
}

impl HidApiWrapper {
    fn new() -> Self {
        HidApiWrapper {
            _api: RefCell::new(Weak::new()),
        }
    }

    fn get(&self) -> Result<Arc<Mutex<hidapi::HidApi>>, Error> {
        let tmp = self._api.borrow().upgrade();
        if tmp.is_some() {
            let api_mutex = tmp.unwrap();
            return Ok(api_mutex);
        }

        let hidapi = hidapi::HidApi::new()?;
        let tmp = Arc::new(Mutex::new(hidapi));
        self._api.replace(Arc::downgrade(&tmp));
        Ok(tmp)
    }
}

impl ApduCommand {
    fn serialize(&self) -> Vec<u8> {
        let mut v = vec![self.cla, self.ins, self.p1, self.p2, self.length];
        v.extend(&self.data);
        v
    }
}

pub fn map_apdu_error(retcode: u16) -> Error {
    match retcode {
        0x6400 => {
            Error::Apdu("[APDU_CODE_EXECUTION_ERROR] No information given (NV-Ram not changed)")
        }
        0x6700 => Error::Apdu("[APDU_CODE_WRONG_LENGTH] Wrong length"),
        0x6982 => Error::Apdu("[APDU_CODE_EMPTY_BUFFER]"),
        0x6983 => Error::Apdu("[APDU_CODE_OUTPUT_BUFFER_TOO_SMALL]"),
        0x6984 => Error::Apdu("[APDU_CODE_DATA_INVALID] data reversibly blocked (invalidated)"),
        0x6985 => {
            Error::Apdu("[APDU_CODE_CONDITIONS_NOT_SATISFIED] Conditions of use not satisfied")
        }
        0x6986 => {
            Error::Apdu("[APDU_CODE_COMMAND_NOT_ALLOWED] Command not allowed (no current EF)")
        }
        0x6A80 => {
            Error::Apdu("[APDU_CODE_BAD_KEY_HANDLE] The parameters in the data field are incorrect")
        }
        0x6B00 => Error::Apdu("[APDU_CODE_INVALIDP1P2] Wrong parameter(s) P1-P2"),
        0x6D00 => {
            Error::Apdu("[APDU_CODE_INS_NOT_SUPPORTED] Instruction code not supported or invalid")
        }
        0x6E00 => Error::Apdu("[APDU_CODE_CLA_NOT_SUPPORTED] Class not supported"),
        0x6F00 => Error::Apdu("[APDU_CODE_UNKNOWN]"),
        0x6F01 => Error::Apdu("[APDU_CODE_SIGN_VERIFY_ERROR]"),
        _ => Error::Apdu("[APDU_ERROR] Unknown"),
    }
}

impl LedgerApp {
    #[cfg(not(target_os = "linux"))]
    fn find_ledger_device_path(api: &hidapi::HidApi) -> Result<CString, Error> {
        for device in api.devices() {
            if device.vendor_id == LEDGER_VID && device.usage_page == LEDGER_USAGE_PAGE {
                return Ok(device.path.clone());
            }
        }
        Err(Error::DeviceNotFound)
    }

    #[cfg(target_os = "linux")]
    fn find_ledger_device_path(api: &hidapi::HidApi) -> Result<CString, Error> {
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

    pub fn new() -> Result<Self, Error> {
        let apiwrapper = HIDAPIWRAPPER.lock().expect("Could not lock api wrapper");
        let api_mutex = apiwrapper.get().expect("Error getting api_mutex");
        let api = api_mutex.lock().expect("Could not lock");

        let device_path = LedgerApp::find_ledger_device_path(&api)?;
        let device = api.open_path(&device_path)?;

        let ledger = LedgerApp {
            device,
            device_mutex: Mutex::new(0),
            api_mutex: api_mutex.clone(),
            logging: false,
        };

        Ok(ledger)
    }

    pub fn logging(&self) -> bool {
        self.logging
    }

    pub fn set_logging(&mut self, val: bool) {
        self.logging = val;
    }

    fn write_apdu(&self, channel: u16, apdu_command: &[u8]) -> Result<i32, Error> {
        let command_length = apdu_command.len() as usize;
        let mut in_data = Vec::with_capacity(command_length + 2);
        in_data.push(((command_length >> 8) & 0xFF) as u8);
        in_data.push((command_length & 0xFF) as u8);
        in_data.extend_from_slice(&apdu_command);

        let mut buffer = vec![0u8; LEDGER_PACKET_SIZE as usize];
        buffer[0] = ((channel >> 8) & 0xFF) as u8; // channel big endian
        buffer[1] = (channel & 0xFF) as u8; // channel big endian
        buffer[2] = 0x05u8;

        for (sequence_idx, chunk) in in_data
            .chunks((LEDGER_PACKET_SIZE - 5) as usize)
            .enumerate()
        {
            buffer[3] = ((sequence_idx >> 8) & 0xFF) as u8; // sequence_idx big endian
            buffer[4] = (sequence_idx & 0xFF) as u8; // sequence_idx big endian
            buffer[5..5 + chunk.len()].copy_from_slice(chunk);

            if self.logging {
                println!("[{:3}] << {:}", buffer.len(), hex::encode(&buffer));
            }

            let result = self.device.write(&buffer);

            match result {
                Ok(size) => {
                    if size < buffer.len() {
                        return Err(Error::Comm("USB write error. Could not send whole message"));
                    }
                }
                Err(x) => return Err(Error::Hid(x)),
            }
        }
        Ok(1)
    }

    fn read_apdu(&self, _channel: u16, apdu_answer: &mut Vec<u8>) -> Result<usize, Error> {
        let mut buffer = vec![0u8; LEDGER_PACKET_SIZE as usize];
        let mut sequence_idx = 0u16;
        let mut expected_apdu_len = 0usize;

        loop {
            let res = self.device.read_timeout(&mut buffer, LEDGER_TIMEOUT)?;

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

            let new_chunk = &buffer[rdr.position() as usize..end_p];

            if self.logging {
                println!("[{:3}] << {:}", new_chunk.len(), hex::encode(&new_chunk));
            }

            apdu_answer.extend_from_slice(new_chunk);

            if apdu_answer.len() >= expected_apdu_len {
                return Ok(apdu_answer.len());
            }

            sequence_idx += 1;
        }
    }

    pub fn exchange(&self, command: ApduCommand) -> Result<ApduAnswer, Error> {
        extern crate hidapi;

        let _guard = self.device_mutex.lock().unwrap();

        self.write_apdu(LEDGER_CHANNEL, &command.serialize())?;

        let mut answer: Vec<u8> = Vec::with_capacity(256);
        let res = self.read_apdu(LEDGER_CHANNEL, &mut answer)?;

        if res < 2 {
            return Err(Error::Comm("response was too short"));
        }

        let apdu_retcode =
            (u16::from(answer[answer.len() - 2]) << 8) + u16::from(answer[answer.len() - 1]);
        let apdu_data = &answer[..answer.len() - 2];

        if apdu_retcode != 0x9000 {
            return Err(map_apdu_error(apdu_retcode));
        }

        Ok(ApduAnswer {
            data: apdu_data.to_vec(),
            retcode: apdu_retcode,
        })
    }

    pub fn close() {
        extern crate hidapi;
    }
}

cfg_if! {
if #[cfg(target_os = "linux")] {
    const HID_MAX_DESCRIPTOR_SIZE: usize = 4096;

    #[repr(C)]
    pub struct HidrawReportDescriptor {
        size: u32,
        value: [u8; HID_MAX_DESCRIPTOR_SIZE],
    }

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
                    let usage_page = match data_len {
                        1 => u16::from(data[i + 1]),
                        2 => (u16::from(data[i + 2] )* 256 + u16::from(data[i + 1])),
                        _ => 0 as u16
                    };

                    return Ok(usage_page);
                }

                i += (data_len + key_size) as usize;
            }
        }
        Ok(0)
    }
}}

#[cfg(test)]
mod integration_tests {
    #[test]
    fn list_all_devices() {
        use HIDAPIWRAPPER;

        let apiwrapper = HIDAPIWRAPPER.lock().expect("Could not lock api wrapper");
        let api_mutex = apiwrapper.get().expect("Error getting api_mutex");
        let api = api_mutex.lock().expect("Could not lock");

        for device_info in api.devices() {
            println!(
                "{:#?} - {:#x}/{:#x}/{:#x}/{:#x} {:#} {:#}",
                device_info.path,
                device_info.vendor_id,
                device_info.product_id,
                device_info.usage_page,
                device_info.interface_number,
                device_info.manufacturer_string.clone().unwrap_or_default(),
                device_info.product_string.clone().unwrap_or_default()
            );
        }
    }

    #[test]
    fn ledger_device_path() {
        use LedgerApp;

        use HIDAPIWRAPPER;

        let apiwrapper = HIDAPIWRAPPER.lock().expect("Could not lock api wrapper");
        let api_mutex = apiwrapper.get().expect("Error getting api_mutex");
        let api = api_mutex.lock().expect("Could not lock");

        // TODO: Extend to discover two devices
        let ledger_path = LedgerApp::find_ledger_device_path(&api).unwrap();
        println!("{:?}", ledger_path);
    }

    #[test]
    fn serialize() {
        use ApduCommand;

        let data = vec![0, 0, 0, 1, 0, 0, 0, 1];

        let command = ApduCommand {
            cla: 0x56,
            ins: 0x01,
            p1: 0x00,
            p2: 0x00,
            length: data.len() as u8,
            data,
        };

        let serialized_command = command.serialize();

        let expected = vec![86, 1, 0, 0, 8, 0, 0, 0, 1, 0, 0, 0, 1];

        assert_eq!(serialized_command, expected)
    }

    #[test]
    fn exchange() {
        use ApduCommand;
        use LedgerApp;

        let mut ledger = LedgerApp::new().unwrap();
        ledger.set_logging(true);

        let command = ApduCommand {
            cla: 0x56,
            ins: 0x00,
            p1: 0x00,
            p2: 0x00,
            length: 0,
            data: Vec::new(),
        };

        let result = ledger.exchange(command).unwrap();
        println!("{:?}", result);
    }
}
