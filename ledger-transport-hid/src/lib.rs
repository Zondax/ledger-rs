/*******************************************************************************
*   (c) 2022 Zondax GmbH
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
mod errors;
use errors::LedgerHIDError;

use byteorder::{BigEndian, ReadBytesExt};
use cfg_if::cfg_if;
use hidapi::{HidApi, HidDevice};
use log::info;

use std::{ffi::CStr, io::Cursor, ops::Deref, sync::Mutex};

pub use hidapi;
use ledger_transport::{async_trait, APDUAnswer, APDUCommand, Exchange};

cfg_if! {
if #[cfg(target_os = "linux")] {
    use nix::{self, ioctl_read};
    use std::mem;
} else {
    // Mock the type in other target_os
    mod nix {
        #[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
        pub enum Error {}
    }
}}

const LEDGER_VID: u16 = 0x2c97;
const LEDGER_USAGE_PAGE: u16 = 0xFFA0;
const LEDGER_CHANNEL: u16 = 0x0101;
const LEDGER_PACKET_SIZE: u8 = 64;
const LEDGER_TIMEOUT: i32 = 10_000_000;

pub struct TransportNativeHID {
    device: Mutex<HidDevice>,
}

impl TransportNativeHID {
    #[cfg(not(target_os = "linux"))]
    fn find_ledger_device_path(api: &HidApi) -> Result<&CStr, LedgerHIDError> {
        for device in api.device_list() {
            if device.vendor_id() == LEDGER_VID && device.usage_page() == LEDGER_USAGE_PAGE {
                return Ok(device.path());
            }
        }
        Err(LedgerHIDError::DeviceNotFound)
    }

    #[cfg(target_os = "linux")]
    fn find_ledger_device_path(api: &HidApi) -> Result<&CStr, LedgerHIDError> {
        for device in api.device_list() {
            if device.vendor_id() == LEDGER_VID {
                let usage_page = get_usage_page(&device.path())?;
                if usage_page == LEDGER_USAGE_PAGE {
                    return Ok(device.path());
                }
            }
        }
        Err(LedgerHIDError::DeviceNotFound)
    }

    pub fn new(api: &HidApi) -> Result<Self, LedgerHIDError> {
        let device_path = TransportNativeHID::find_ledger_device_path(&api)?;
        let device = api.open_path(&device_path)?;

        let ledger = TransportNativeHID {
            device: Mutex::new(device),
        };

        Ok(ledger)
    }

    fn write_apdu(
        device: &HidDevice,
        channel: u16,
        apdu_command: &[u8],
    ) -> Result<i32, LedgerHIDError> {
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

            info!("[{:3}] << {:}", buffer.len(), hex::encode(&buffer));

            let result = device.write(&buffer);

            match result {
                Ok(size) => {
                    if size < buffer.len() {
                        return Err(LedgerHIDError::Comm(
                            "USB write error. Could not send whole message",
                        ));
                    }
                }
                Err(x) => return Err(LedgerHIDError::Hid(x)),
            }
        }
        Ok(1)
    }

    fn read_apdu(
        device: &HidDevice,
        _channel: u16,
        apdu_answer: &mut Vec<u8>,
    ) -> Result<usize, LedgerHIDError> {
        let mut buffer = vec![0u8; LEDGER_PACKET_SIZE as usize];
        let mut sequence_idx = 0u16;
        let mut expected_apdu_len = 0usize;

        loop {
            let res = device.read_timeout(&mut buffer, LEDGER_TIMEOUT)?;

            if (sequence_idx == 0 && res < 7) || res < 5 {
                return Err(LedgerHIDError::Comm("Read error. Incomplete header"));
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
                return Err(LedgerHIDError::Comm("Invalid sequence idx"));
            }

            if rcv_seq_idx == 0 {
                expected_apdu_len = rdr.read_u16::<BigEndian>()? as usize;
            }

            let available: usize = buffer.len() - rdr.position() as usize;
            let missing: usize = expected_apdu_len - apdu_answer.len();
            let end_p = rdr.position() as usize + std::cmp::min(available, missing);

            let new_chunk = &buffer[rdr.position() as usize..end_p];

            info!("[{:3}] << {:}", new_chunk.len(), hex::encode(&new_chunk));

            apdu_answer.extend_from_slice(new_chunk);

            if apdu_answer.len() >= expected_apdu_len {
                return Ok(apdu_answer.len());
            }

            sequence_idx += 1;
        }
    }

    pub fn exchange<I: Deref<Target = [u8]>>(
        &self,
        command: &APDUCommand<I>,
    ) -> Result<APDUAnswer<Vec<u8>>, LedgerHIDError> {
        let device = self.device.lock().expect("HID device poisoned");

        Self::write_apdu(&device, LEDGER_CHANNEL, &command.serialize())?;

        let mut answer: Vec<u8> = Vec::with_capacity(256);
        Self::read_apdu(&device, LEDGER_CHANNEL, &mut answer)?;

        APDUAnswer::from_answer(answer).map_err(|_| LedgerHIDError::Comm("response was too short"))
    }
}

#[async_trait]
impl Exchange for TransportNativeHID {
    type Error = LedgerHIDError;
    type AnswerType = Vec<u8>;

    async fn exchange<I>(
        &self,
        command: &APDUCommand<I>,
    ) -> Result<APDUAnswer<Self::AnswerType>, Self::Error>
    where
        I: Deref<Target = [u8]> + Send + Sync,
    {
        self.exchange(command)
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

    fn get_usage_page(device_path: &CStr) -> Result<u16, LedgerHIDError>
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

            hid_read_descr_size(fd, &mut desc_size)?;
            let mut desc_raw_uninit = mem::MaybeUninit::<HidrawReportDescriptor>::new(HidrawReportDescriptor {
                size: desc_size as u32,
                value: [0u8; 4096]
            });
            hid_read_descr(fd, desc_raw_uninit.as_mut_ptr())?;
            let desc_raw = desc_raw_uninit.assume_init();

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
    use crate::{APDUCommand, TransportNativeHID};
    use hidapi::HidApi;
    use log::info;
    use once_cell::sync::Lazy;
    use serial_test::serial;

    fn init_logging() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    fn hidapi() -> &'static HidApi {
        static HIDAPI: Lazy<HidApi> = Lazy::new(|| HidApi::new().expect("unable to get HIDAPI"));

        &HIDAPI
    }

    #[test]
    #[serial]
    fn list_all_devices() {
        init_logging();
        let api = hidapi();

        for device_info in api.device_list() {
            info!(
                "{:#?} - {:#x}/{:#x}/{:#x}/{:#x} {:#} {:#}",
                device_info.path(),
                device_info.vendor_id(),
                device_info.product_id(),
                device_info.usage_page(),
                device_info.interface_number(),
                device_info.manufacturer_string().unwrap_or_default(),
                device_info.product_string().unwrap_or_default()
            );
        }
    }

    #[test]
    #[serial]
    fn ledger_device_path() {
        init_logging();
        let api = hidapi();

        // TODO: Extend to discover two devices
        let ledger_path =
            TransportNativeHID::find_ledger_device_path(&api).expect("Could not find a device");
        info!("{:?}", ledger_path);
    }

    #[test]
    #[serial]
    fn serialize() {
        let data = vec![0, 0, 0, 1, 0, 0, 0, 1];

        let command = APDUCommand {
            cla: 0x56,
            ins: 0x01,
            p1: 0x00,
            p2: 0x00,
            data,
        };

        let serialized_command = command.serialize();

        let expected = vec![86, 1, 0, 0, 8, 0, 0, 0, 1, 0, 0, 0, 1];

        assert_eq!(serialized_command, expected)
    }

    #[test]
    #[serial]
    fn exchange() {
        use ledger_zondax_generic::{App, AppExt};
        struct Dummy;
        impl App for Dummy {
            const CLA: u8 = 0;
        }

        init_logging();

        let ledger = TransportNativeHID::new(hidapi()).expect("Could not get a device");

        // use device info command that works in the dashboard
        let result = futures::executor::block_on(Dummy::get_device_info(&ledger))
            .expect("Error during exchange");
        info!("{:x?}", result);
    }
}
