
use crate::{ApduCmd, ApduBase, ApduEmpty, ApduError};

/// Application info APDU command
#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct AppInfoGet {}

impl <'a> ApduCmd<'a> for AppInfoGet {
    /// Application Info command APDU is class `0xb0`
    const CLA: u8 = 0xb0;

    /// Application Info GET APDU is instruction `0x00`
    const INS: u8 = 0x01;
}

/// [`AppInfoGet`] APDU command has no body
impl ApduEmpty for AppInfoGet {}


/// Application information APDU response
#[derive(Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all="camelCase"))]
pub struct AppInfo<'a> {
    /// Application name
    pub name: &'a str,
    /// Application version
    pub version: &'a str,
    /// Application flags
    pub flags: AppFlags,
}

bitflags::bitflags! {
    /// Application info flags
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct AppFlags: u8 {
        /// Recovery mode
        const RECOVERY = 0x01;
        /// Signed application
        const SIGNED = 0x02;
        /// User onboarded
        const ONBOARDED = 0x04;
        /// PIN validated
        const PIN_VALIDATED = 0xF0;
    }
}

const APP_VERSION_FMT: u8 = 1;

impl <'a> AppInfo<'a> {
    /// Create a new application version APDU
    pub fn new(name: &'a str, version: &'a str, flags: AppFlags) -> Self {
        Self{ name, version, flags }
    }
}

impl <'a>ApduBase<'a> for AppInfo<'a> {
    /// Encode an app version APDU into the provided buffer
    fn encode(&self, buff: &mut [u8]) -> usize {
        // TODO: check buffer length is viable

        let mut index = 0;
        buff[0] = APP_VERSION_FMT;
        index += 1;

        buff[index] = self.name.len() as u8;
        buff[index + 1..][..self.name.len()].copy_from_slice(self.name.as_bytes());
        index += 1 + self.name.len();

        buff[index] = self.version.len() as u8;
        buff[index + 1..][..self.version.len()].copy_from_slice(self.version.as_bytes());
        index += 1 + self.version.len();

        buff[index] = 1;
        buff[index + 1] = self.flags.bits();
        index += 2;

        return index;
            
    }

    /// Decode an app version APDU from the provided buffer
    fn decode(buff: &'a [u8]) -> Result<Self, ApduError> {
        let mut index = 0;
        let buff = buff.as_ref();

        // Check app version format
        if buff[index] != APP_VERSION_FMT {
            return Err(ApduError::InvalidVersion(buff[index]));
        }
        index += 1;

        // Fetch name string
        let name_len = buff[index] as usize;
        let name = core::str::from_utf8(&buff[index + 1..][..name_len])
            .map_err(|_| ApduError::Utf8 )?;
        index += 1 + name_len;

        // Fetch version string
        let version_len = buff[index] as usize;
        let version = core::str::from_utf8(&buff[index + 1..][..version_len])
            .map_err(|_| ApduError::Utf8 )?;
        index += 1 + version_len;

        // Fetch flags
        let _flags_len = buff[index];
        let flags = AppFlags::from_bits_truncate(buff[index + 1]);

        Ok(Self{ name, version, flags })
    }
}

#[cfg(test)]
mod test {    
    use crate::test::encode_decode_apdu;
    use super::*;

    #[test]
    fn app_info_get_apdu() {
        let apdu = AppInfoGet::default();

        let mut buff = [0u8; 128];
        encode_decode_apdu(&mut buff, &apdu);
    }

    #[test]
    fn app_info_apdu() {
        let name = "TEST NAME";
        let version = "TEST VERSION";

        let apdu = AppInfo::new(name, version, AppFlags::ONBOARDED);

        let mut buff = [0u8; 128];
        encode_decode_apdu(&mut buff, &apdu);
    }
}