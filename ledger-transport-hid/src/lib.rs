/*******************************************************************************
*   (c) 2022 Zondax AG
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
pub use errors::LedgerHIDError;

use byteorder::{BigEndian, ReadBytesExt};
use hidapi::{DeviceInfo, HidApi, HidDevice};
use log::info;

use std::{io::Cursor, ops::Deref, sync::Mutex};

pub use hidapi;
use ledger_transport::{async_trait, APDUAnswer, APDUCommand, Exchange};

const LEDGER_VID: u16 = 0x2c97;
const LEDGER_USAGE_PAGE: u16 = 0xFFA0;
const LEDGER_CHANNEL: u16 = 0x0101;
// for Windows compatability, we prepend the buffer with a 0x00
// so the actual buffer is 64 bytes
const LEDGER_PACKET_WRITE_SIZE: u8 = 65;
const LEDGER_PACKET_READ_SIZE: u8 = 64;
const LEDGER_TIMEOUT: i32 = 10_000_000;

pub struct TransportNativeHID {
    device: Mutex<HidDevice>,
}

impl TransportNativeHID {
    fn is_ledger(dev: &DeviceInfo) -> bool {
        dev.vendor_id() == LEDGER_VID && dev.usage_page() == LEDGER_USAGE_PAGE
    }

    /// Get a list of ledger devices available
    pub fn list_ledgers(api: &HidApi) -> impl Iterator<Item = &DeviceInfo> {
        api.device_list().filter(|dev| Self::is_ledger(dev))
    }

    /// Create a new HID transport, connecting to the first ledger found
    /// # Warning
    /// Opening the same device concurrently will lead to device lock after the first handle is closed
    /// see [issue](https://github.com/ruabmbua/hidapi-rs/issues/81)
    pub fn new(api: &HidApi) -> Result<Self, LedgerHIDError> {
        let first_ledger = Self::list_ledgers(api)
            .next()
            .ok_or(LedgerHIDError::DeviceNotFound)?;

        Self::open_device(api, first_ledger)
    }

    /// Open a specific ledger device
    ///
    /// # Note
    /// No checks are made to ensure the device is a ledger device
    ///
    /// # Warning
    /// Opening the same device concurrently will lead to device lock after the first handle is closed
    /// see [issue](https://github.com/ruabmbua/hidapi-rs/issues/81)
    pub fn open_device(api: &HidApi, device: &DeviceInfo) -> Result<Self, LedgerHIDError> {
        let device = device.open_device(api)?;
        let _ = device.set_blocking_mode(true);

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
        in_data.extend_from_slice(apdu_command);

        let mut buffer = vec![0u8; LEDGER_PACKET_WRITE_SIZE as usize];
        // Windows platform requires 0x00 prefix and Linux/Mac tolerate this as well
        buffer[0] = 0x00;
        buffer[1] = ((channel >> 8) & 0xFF) as u8; // channel big endian
        buffer[2] = (channel & 0xFF) as u8; // channel big endian
        buffer[3] = 0x05u8;

        for (sequence_idx, chunk) in in_data
            .chunks((LEDGER_PACKET_WRITE_SIZE - 6) as usize)
            .enumerate()
        {
            buffer[4] = ((sequence_idx >> 8) & 0xFF) as u8; // sequence_idx big endian
            buffer[5] = (sequence_idx & 0xFF) as u8; // sequence_idx big endian
            buffer[6..6 + chunk.len()].copy_from_slice(chunk);

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
        channel: u16,
        apdu_answer: &mut Vec<u8>,
    ) -> Result<usize, LedgerHIDError> {
        let mut buffer = vec![0u8; LEDGER_PACKET_READ_SIZE as usize];
        let mut sequence_idx = 0u16;
        let mut expected_apdu_len = 0usize;

        loop {
            let res = device.read_timeout(&mut buffer, LEDGER_TIMEOUT)?;

            if (sequence_idx == 0 && res < 7) || res < 5 {
                return Err(LedgerHIDError::Comm("Read error. Incomplete header"));
            }

            let mut rdr = Cursor::new(&buffer);

            let rcv_channel = rdr.read_u16::<BigEndian>()?;
            let rcv_tag = rdr.read_u8()?;
            let rcv_seq_idx = rdr.read_u16::<BigEndian>()?;

            if rcv_channel != channel {
                return Err(LedgerHIDError::Comm("Invalid channel"));
            }
            if rcv_tag != 0x05u8 {
                return Err(LedgerHIDError::Comm("Invalid tag"));
            }

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

        let mut ledgers = TransportNativeHID::list_ledgers(&api);

        let a_ledger = ledgers.next().expect("could not find any ledger device");
        info!("{:?}", a_ledger.path());
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

    #[test]
    #[serial]
    #[ignore] //see https://github.com/ruabmbua/hidapi-rs/issues/81
    fn open_same_device_twice() {
        use ledger_zondax_generic::{App, AppExt};
        struct Dummy;
        impl App for Dummy {
            const CLA: u8 = 0;
        }

        init_logging();

        let api = hidapi();
        let ledger = TransportNativeHID::list_ledgers(&api)
            .next()
            .expect("could not get a device");

        let t1 = TransportNativeHID::open_device(api, ledger).expect("Could not open device");
        let t2 = TransportNativeHID::open_device(api, ledger).expect("Could not open device");

        // use device info command that works in the dashboard
        let (r1, r2) = futures::executor::block_on(futures::future::join(
            Dummy::get_device_info(&t1),
            Dummy::get_device_info(&t2),
        ));

        let (r1, r2) = (
            r1.expect("error during exchange (t1)"),
            r2.expect("error during exchange (t2)"),
        );

        info!("r1: {:x?}", r1);
        info!("r2: {:x?}", r2);

        assert_eq!(r1, r2);
    }
}
