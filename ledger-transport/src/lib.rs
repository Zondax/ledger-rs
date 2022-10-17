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
//! Generic APDU transport library for Ledger Nano S/X apps

#![deny(trivial_casts, trivial_numeric_casts)]
#![deny(unused_import_braces, unused_qualifications)]
#![deny(missing_docs)]

use std::error::Error;
use core::fmt::Debug;

pub use async_trait::async_trait;
use byteorder::{NetworkEndian, ByteOrder};
use ledger_apdu::ApduError;
pub use ledger_apdu::{ApduCmd, ApduBase, ApduHeader, ApduErrorCode, APDUAnswer, APDUCommand, APDU_HDR_LEN};

/// Use to talk to the ledger device
#[async_trait]
pub trait Exchange: Send {
    /// Error defined by Transport used
    type Error: Error + Debug;

    /// Send a command with the given transport and retrieve an answer or a transport error.
    /// 
    /// The provided buffer is used for TX and RX operations to allow use with reference containing objects
    async fn exchange<'a, 'c, ANS: ApduBase<'a>>(
        &self,
        command: impl ApduCmd<'c>,
        buff: &'a mut [u8],
    ) -> Result<ANS, Self::Error>;


    /// Helper to encode an APDU command to buffer w/ headers for use
    /// when implementing [`exhange`].
    /// 
    /// This may be overwritten where headers must be extended (eg. for the speculos / TCP transport where messages are prefixed by their length)
    fn apdu_encode<'a, CMD: ApduCmd<'a>>(apdu: &CMD, buff: &mut [u8]) -> Result<usize, ApduError> {
        // Generate APDU header
        let hdr = apdu.header();

        // Encode the header
        hdr.encode(&mut buff[..]);

        // Encode the command
        let l = apdu.encode(&mut buff[APDU_HDR_LEN..])?;

        // Return encoded length
        Ok(l + APDU_HDR_LEN)
    }

    /// Helper to decode APDU responses and return codes, 
    /// for use when implementing [`exchange`].
    /// 
    /// This may be overwritten where headers must be extended (eg. for the speculos / TCP transport where messages are prefixed by their length)
    fn apdu_decode<'a, ANS: ApduBase<'a>>(buff: &'a [u8]) -> Result<ANS, ApduError> {

        // Fetch response code
        let retcode = NetworkEndian::read_u16(&buff[buff.len()-2..][..2]);

        // TODO: is there any case in which a message should be decode in spite of
        // a failing response code?
        if retcode != ApduErrorCode::NoError as u16 {
            return Err(ApduError::ErrorCode(retcode));
        }

        // Decode response message
        let (answer, _) = ANS::decode(&buff[..buff.len()-2])?;

        // Return answer
        Ok(answer)
    }

}

#[async_trait]
impl <T: Exchange + Send + Sync> Exchange for &T {
    type Error = <T as Exchange>::Error;

    async fn exchange<'a, 'c, ANS: ApduBase<'a>>(
        &self,
        command: impl ApduCmd<'c>,
        buff: &'a mut [u8],
    ) -> Result<ANS, Self::Error> {
        <T as Exchange>::exchange(self, command, buff).await
    }
}