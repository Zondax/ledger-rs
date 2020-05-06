/*******************************************************************************
*   (c) 2020 ZondaX GmbH
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
use byteorder::{LittleEndian, WriteBytesExt};

pub mod errors;

use crate::errors::BIP44PathError;

const HARDENED_BIT: u32 = 1 << 31;

#[derive(Debug)]
pub struct BIP44Path(pub [u32; 5]);

/// Bip44Path
///
/// Implementation of the BIP44 standard for derivation path.
///
impl BIP44Path {
    pub fn from_slice(path: &[u32]) -> Result<BIP44Path, BIP44PathError> {
        let mut path_array: [u32; 5] = Default::default();
        if path.len() != 5 {
            return Err(BIP44PathError::InvalidLength);
        };

        path_array.copy_from_slice(path);

        Ok(BIP44Path(path_array))
    }

    pub fn from_string(path: &str) -> Result<BIP44Path, BIP44PathError> {
        let mut path = path.split('/');

        if path.next() != Some("m") {
            return Err(BIP44PathError::MissingPrefix);
        };

        let result = path
            .map(|index| {
                let (index_to_parse, mask) = if index.ends_with('\'') {
                    // Remove the last character and harden index
                    (&index[..index.len() - 1], HARDENED_BIT)
                } else {
                    (index, 0)
                };

                // FIX ME
                let child_index = index_to_parse.parse::<u32>()?;

                Ok(child_index | mask)
            })
            .collect::<Result<Vec<u32>, std::num::ParseIntError>>()?;

        let bip44_path = BIP44Path::from_slice(&result)?;

        Ok(bip44_path)
    }

    pub fn serialize(&self) -> Vec<u8>  {
        let mut m = Vec::new();
        m.write_u32::<LittleEndian>(self.0[0]).unwrap();
        m.write_u32::<LittleEndian>(self.0[1]).unwrap();
        m.write_u32::<LittleEndian>(self.0[2]).unwrap();
        m.write_u32::<LittleEndian>(self.0[3]).unwrap();
        m.write_u32::<LittleEndian>(self.0[4]).unwrap();
        m
    }

    pub fn is_testnet(&self) -> bool {
        return self.0[1] == (1 | HARDENED_BIT);
    }
}

#[cfg(test)]
mod tests {
    use crate::BIP44Path;
    use crate::errors::BIP44PathError;
    use byteorder::{LittleEndian, WriteBytesExt};

    const HARDENED_BIT: u32 = 1 << 31;

    #[test]
    fn create_derive_path() {
        let path_string = "m/44'/461'/0/0/0";

        let result = BIP44Path::from_string(path_string).unwrap();

        assert_eq!(result.0[0], (44 | HARDENED_BIT));
        assert_eq!(result.0[1], (461 | HARDENED_BIT));
        assert_eq!(result.0[2], 0);
        assert_eq!(result.0[3], 0);
        assert_eq!(result.0[4], 0);
    }

    #[test]
    fn serialize_path() {
        let path_string = "m/44'/461'/0/0/0";

        let bip44_path = BIP44Path::from_string(path_string).unwrap();

        let path_serialized = bip44_path.serialize();

        let mut expected_result = Vec::new();
        expected_result.write_u32::<LittleEndian>(44 | HARDENED_BIT).unwrap();
        expected_result.write_u32::<LittleEndian>(461 | HARDENED_BIT).unwrap();
        expected_result.write_u32::<LittleEndian>(0).unwrap();
        expected_result.write_u32::<LittleEndian>(0).unwrap();
        expected_result.write_u32::<LittleEndian>(0).unwrap();

        assert_eq!(path_serialized, expected_result)
    }

    #[test]
    fn error_missing_prefix() {
        let path_string = "44/44'/461'/0/0/0";

        let result_err = BIP44Path::from_string(path_string).unwrap_err();

        assert_eq!(result_err, BIP44PathError::MissingPrefix);
    }

    #[test]
    fn error_invalid_length() {
        let path_string = "m/44'/461'/0/0";

        let result_err = BIP44Path::from_string(path_string).unwrap_err();

        assert_eq!(result_err, BIP44PathError::InvalidLength);
    }

    #[test]
    fn error_invalid_integer() {
        let path_string = "m/44'/461'/a/0/0";

        let result_err = BIP44Path::from_string(path_string);

        println!("{:?}", result_err);

        assert!(result_err.is_err());
    }
}
