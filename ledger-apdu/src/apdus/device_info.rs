
use encdec::{Encode, Decode};

use crate::{ApduStatic, ApduBase, ApduError};

/// Device info APDU command
#[derive(Copy, Clone, PartialEq, Debug, Default, Encode, Decode)]
#[encdec(error="ApduError")]
pub struct DeviceInfoGet {}

impl ApduStatic for DeviceInfoGet {
    /// Device Info command APDU is class `0xe0`
    const CLA: u8 = 0xe0;

    /// Device Info GET APDU is instruction `0x01`
    const INS: u8 = 0x01;
}

/// Device information APDU response
#[derive(Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all="camelCase"))]
pub struct DeviceInfo<'a> {
    /// Target ID
    #[cfg_attr(features = "serde", serde(rename(serialize = "targetId")))]
    pub target_id: [u8; 4],

    /// Secure Element Version
    #[cfg_attr(features = "serde", serde(rename(serialize = "seVersion")))]
    pub se_version: &'a str,
    
    /// Device Flag(s)
    pub flag: &'a[u8],
    
    /// MCU Version
    #[cfg_attr(features = "serde", serde(rename(serialize = "mcuVersion")))]
    pub mcu_version: &'a str,
}


impl <'a> DeviceInfo<'a> {
    /// Create a new device info APDU
    pub fn new(target_id: [u8; 4], se_version: &'a str, mcu_version: &'a str, flag: &'a[u8]) -> Self {
        Self{ target_id, se_version, mcu_version, flag }
    }
}

impl <'a>Encode for DeviceInfo<'a> {
    type Error = ApduError;

    /// Encode an device info APDU into the provided buffer
    fn encode(&self, buff: &mut [u8]) -> Result<usize, ApduError> {
        // TODO: check buffer length is viable

        let mut index = 0;

        // Write target ID
        buff[index..][..4].copy_from_slice(&self.target_id);
        index += 4;

        // Write SE version
        buff[index] = self.se_version.len() as u8;
        buff[index + 1..][..self.se_version.len()].copy_from_slice(self.se_version.as_bytes());
        index += 1 + self.se_version.len();

        // Write flags
        buff[index] = self.flag.len() as u8;
        buff[index + 1..][..self.flag.len()].copy_from_slice(self.flag);
        index += 1 + self.flag.len();

        // Write MCU version
        buff[index] = self.mcu_version.len() as u8;
        buff[index + 1..][..self.mcu_version.len()].copy_from_slice(self.mcu_version.as_bytes());
        index += 1 + self.mcu_version.len();
        

        Ok(index)
            
    }

    /// Compute APDU encoded length
    fn encode_len(&self) -> Result<usize, ApduError> {
        let mut len = 4;

        len += 1 + self.se_version.len();
        len += 1 + self.flag.len();
        len += 1 + self.mcu_version.len();

        Ok(len)
    }
}


impl <'a>Decode<'a> for DeviceInfo<'a> {
    type Output = Self;
    type Error = ApduError;

    /// Decode an device info APDU from the provided buffer
    fn decode(buff: &'a [u8]) -> Result<(Self, usize), ApduError> {
        let mut index = 0;
        let buff = buff.as_ref();

        // Fetch target id
        let mut target_id = [0u8; 4];
        target_id.copy_from_slice(&buff[..4]);
        index += 4;

        // Fetch secure element version
        let se_version_len = buff[index] as usize;
        let se_version = core::str::from_utf8(&buff[index + 1..][..se_version_len])
            .map_err(|_| ApduError::Utf8 )?;
        index += 1 + se_version_len;

        // Fetch flags
        let flags_len = buff[index] as usize;
        let flag = &buff[index + 1..][..flags_len];
        index += 1 + flags_len;

        // Fetch mcu version
        let mcu_version_len = buff[index] as usize;
        let mcu_version = core::str::from_utf8(&buff[index + 1..][..mcu_version_len])
            .map_err(|_| ApduError::Utf8 )?;
        index += 1 + mcu_version_len;

        Ok((Self{ target_id, se_version, flag, mcu_version }, index))
    }
}



#[cfg(test)]
mod test {    
    use crate::test::encode_decode_apdu;
    use super::*;

    #[test]
    fn device_info_get_apdu() {
        let apdu = DeviceInfoGet::default();

        let mut buff = [0u8; 128];
        encode_decode_apdu(&mut buff, &apdu);
    }

    #[test]
    fn device_info_apdu() {
        let se = "SOME SE";
        let mcu = "SOME MCU";
        let flags = [12u8];
        let target = [0xab; 4];

        let apdu = DeviceInfo::new(target, se, mcu, &flags);

        let mut buff = [0u8; 128];
        encode_decode_apdu(&mut buff, &apdu);
    }
}
