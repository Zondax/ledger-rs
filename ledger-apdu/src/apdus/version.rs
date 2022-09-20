
use num_enum::{IntoPrimitive, TryFromPrimitive};
use byteorder::{ByteOrder, NetworkEndian};
use encdec::{Encode, Decode};

use crate::{ApduStatic, ApduBase, ApduError};

/// Version APDU command
#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct VersionGet<const CLA: u8 = 0x00> {}

impl <'a, const CLA: u8> ApduStatic for VersionGet<CLA> {
    /// Version command class defined by application
    const CLA: u8 = CLA;

    /// Application Version GET APDU is instruction 0
    const INS: u8 = 0x00;
}

impl <const CLA: u8> Encode for VersionGet<CLA> {
    type Error = ApduError;

    fn encode_len(&self) -> Result<usize, Self::Error> {
        Ok(0)
    }

    fn encode(&self, _buff: &mut [u8]) -> Result<usize, Self::Error> {
        Ok(0)
    }
}

impl <'a, const CLA: u8> Decode<'a> for VersionGet<CLA> {
    type Error = ApduError;

    type Output = Self;

    fn decode(_buff: &'a [u8]) -> Result<(Self::Output, usize), Self::Error> {
        Ok((Self{}, 0))
    }
}

/// Application information APDU response
#[derive(Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all="camelCase"))]
pub struct Version {
    /// Application Mode
    #[cfg_attr(features = "serde", serde(rename(serialize = "testMode")))]
    pub mode: VersionMode,
    /// Version Major
    pub major: u16,
    /// Version Minor
    pub minor: u16,
    /// Version Patch
    pub patch: u16,
    /// Device is locked
    pub locked: bool,
    /// Target ID
    pub target_id: [u8; 4],
}


impl Version {
    /// Create a new application version APDU
    pub fn new(mode: VersionMode, major: u16, minor: u16, patch: u16, locked: bool, target_id: [u8; 4]) -> Self {
        Self{ mode, major, minor, patch, locked, target_id }
    }
}

/// Version encoding mode
#[derive(Copy, Clone, PartialEq, Debug, IntoPrimitive, TryFromPrimitive)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all="camelCase"))]
#[repr(u8)]
pub enum VersionMode {
    /// Single byte version numbers
    SingleByte = 0x04,
    /// Two byte version numbers
    DoubleByte = 0x07,
    /// Single byte version numbers with flags and target_id
    SingleBytePlus = 0x09,
    /// Two byte version numbers with flags and target_id
    DoubleBytePlus = 0x0c,
}

impl Encode for Version {
    type Error = ApduError;

    /// Encode an app version APDU into the provided buffer
    fn encode(&self, buff: &mut [u8]) -> Result<usize, ApduError> {
        // TODO: check buffer length is viable

        let mut index = 0;
        
        // Write mode
        buff[index] = self.mode.into();
        index += 1;

        // Write version numbers
        match self.mode {
            VersionMode::SingleByte | VersionMode::SingleBytePlus => {
                buff[index + 0] = self.major as u8;
                buff[index + 1] = self.minor as u8;
                buff[index + 2] = self.patch as u8;
                index += 3;
            },
            VersionMode::DoubleByte | VersionMode::DoubleBytePlus => {
                NetworkEndian::write_u16(&mut buff[index + 0..], self.major);
                NetworkEndian::write_u16(&mut buff[index + 2..], self.minor);
                NetworkEndian::write_u16(&mut buff[index + 4..], self.patch);
                index += 6;
            },
        };

        // Write flags
        match self.mode {
            VersionMode::SingleBytePlus | VersionMode::DoubleBytePlus => {
                if self.locked {
                    buff[index] = 1;
                } else {
                    buff[index] = 0;
                }
                
                buff[index+1..][..4].copy_from_slice(&self.target_id);
                index += 5;
            },
            _ => (),
        };


        Ok(index)
            
    }

    /// Compute APDU encoded length
    fn encode_len(&self) -> Result<usize, ApduError> {
        let mut len = 1;

        len += match self.mode {
            VersionMode::SingleByte | VersionMode::SingleBytePlus => 3,
            VersionMode::DoubleByte | VersionMode::DoubleBytePlus => 6,
        };

        len += match self.mode {
            VersionMode::SingleBytePlus | VersionMode::DoubleBytePlus => 5,
            _ => 0,
        };

        Ok(len)
    }
}

impl <'a> Decode<'a> for Version {
    type Output = Self;
    type Error = ApduError;
    
    /// Decode an app version APDU from the provided buffer
    fn decode(buff: &'a [u8]) -> Result<(Self::Output, usize), Self::Error> {
        let mut index = 0;

        // Parse out mode
        let mode = match VersionMode::try_from(buff[index]) {
            Ok(v) => v,
            Err(_) => return Err(ApduError::InvalidVersion(buff[index])),
        };
        index += 1;

        // Parse out version numbers
        let (major, minor, patch) = match mode {
            VersionMode::SingleByte | VersionMode::SingleBytePlus => {
                let (ma, mi, p) = (
                    buff[index + 0] as u16,
                    buff[index + 1] as u16,
                    buff[index + 2] as u16,
                );
                index += 3;
                (ma, mi, p)
            },
            VersionMode::DoubleByte | VersionMode::DoubleBytePlus => {
                let (ma, mi, p) = (
                    NetworkEndian::read_u16(&buff[index + 0..]),
                    NetworkEndian::read_u16(&buff[index + 2..]),
                    NetworkEndian::read_u16(&buff[index + 4..]),
                );
                index += 6;
                (ma, mi, p)
            },
        };

        // Parse out flags
        let (locked, target_id) = match mode {
            VersionMode::SingleBytePlus | VersionMode::DoubleBytePlus => {
                let locked = buff[index] != 0;
                let mut target_id = [0u8; 4];
                target_id.copy_from_slice(&buff[index+1..][..4]);

                index += 5;
                (locked, target_id)
            },
            VersionMode::SingleByte | VersionMode::DoubleByte => (false, [0u8; 4]),
        };

        Ok((Self{ mode, major, minor, patch, locked, target_id }, index))
    }

}

#[cfg(test)]
mod test {
    use crate::test::encode_decode_apdu;
    use super::*;

    #[test]
    fn version_get_apdu() {
        let apdu = VersionGet::<0x12>::default();

        let mut buff = [0u8; 128];
        encode_decode_apdu(&mut buff, &apdu);
    }

    #[test]
    fn version_apdu() {
        // Test each mode
        let tests = &[
            Version::new(VersionMode::SingleByte, 10, 11, 12, false, [0x00; 4]),
            Version::new(VersionMode::SingleBytePlus, 10, 11, 12, false, [0xaa; 4]),
            Version::new(VersionMode::DoubleByte, 1010, 1011, 1012, false, [0x00; 4]),
            Version::new(VersionMode::DoubleBytePlus, 1010, 1011, 1012, false, [0xaa; 4]),
        ];

        for t in tests {
            let mut buff = [0u8; 128];
            encode_decode_apdu(&mut buff, t);
        }
    }
}
