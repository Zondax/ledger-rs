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
//! This crate contains a couple of utilities to talk via the APDU protocol to Ledger devices

#![no_std]
#![deny(missing_docs)]

extern crate no_std_compat as std;


use core::{ops::Deref, fmt::Debug};
use core::convert::{TryFrom, TryInto};

use snafu::prelude::*;

#[cfg(test)]
mod tests;

mod apdus;

mod error;
pub use error::{ApduError, APDUErrorCode};

#[derive(Debug, Clone)]
/// An APDU command
pub struct APDUCommand<B> {
    ///APDU Class
    ///
    /// An incorrect APDU Class will prevent you from communicating with the device
    pub cla: u8,
    ///APDU Instruction
    pub ins: u8,
    ///First parameter of instruction
    pub p1: u8,
    ///Second parameter of instruction
    pub p2: u8,
    ///Payload of the instruction, can be empty
    pub data: B,
}

#[cfg(feature = "std")]
impl<B> APDUCommand<B>
where
    B: Deref<Target = [u8]>,
{
    /// Serialize this [APDUCommand] to be sent to the device
    pub fn serialize(&self) -> std::vec::Vec<u8> {
        let mut v = std::vec![self.cla, self.ins, self.p1, self.p2, self.data.len() as u8];
        v.extend(self.data.iter());
        v
    }
}

#[derive(Debug)]
/// An APDU answer, whole last 2 bytes are interpreted as `retcode`
pub struct APDUAnswer<B> {
    data: B,
    retcode: u16,
}

#[derive(Debug, Snafu, PartialEq, Eq)]
/// Error interpreting bytes as an APDU answer
pub enum APDUAnswerError {
    #[snafu(display("answer too short (< 2 bytes)"))]
    /// Passed APDU answer was less than the minimum 2 bytes required for the return code
    TooShort,
}

impl<B> APDUAnswer<B>
where
    B: std::ops::Deref<Target = [u8]>,
{
    /// Attempt to interpret the given slice as an APDU answer
    pub fn from_answer(answer: B) -> Result<Self, APDUAnswerError> {
        ensure!(answer.len() >= 2, TooShortSnafu);
        let retcode = arrayref::array_ref!(answer, answer.len() - 2, 2);
        let retcode = u16::from_be_bytes(*retcode);

        Ok(APDUAnswer {
            data: answer,
            retcode,
        })
    }

    /// Will return the answer's payload
    #[inline(always)]
    pub fn apdu_data(&self) -> &[u8] {
        &self.data[..self.data.len() - 2]
    }

    /// Will return the answer's payload
    #[inline(always)]
    pub fn data(&self) -> &[u8] {
        self.apdu_data()
    }

    /// Will attempt to interpret the error code as an [APDUErrorCode],
    /// returning the code as is otherwise
    pub fn error_code(&self) -> Result<APDUErrorCode, u16> {
        self.retcode.try_into().map_err(|_| self.retcode)
    }

    /// Returns the raw return code
    #[inline(always)]
    pub fn retcode(&self) -> u16 {
        self.retcode
    }
}

/// [`ApduBase`] provides encode/decode methods for APDUs
pub trait ApduBase<'a>: Send + PartialEq + Debug + Sized {
    /// Encode an APDU to the provided buffer, returning the length of the encoded data.
    fn encode(&self, buff: &mut [u8]) -> usize;

    /// Decode an APDU from the provided buffer, returning the decoded object
    fn decode(buff: &'a [u8]) -> Result<Self, ApduError>;
}

/// [`ApduCmd`] implemented for APDU commands / requests
pub trait ApduCmd<'a>: ApduBase<'a> {
    /// Class ID for APDU commands
    const CLA: u8;

    /// Instruction ID for APDU commands
    const INS: u8;

    /// Fetch P1 value
    fn p1(&self) -> u8 {
        0
    }

    /// Fetch P2 value
    fn p2(&self) -> u8 {
        0
    }
}

/// Marker trait for empty APDUs (automatically implements [`ApduBase`])
pub trait ApduEmpty: Send + PartialEq + Debug + Default {}

/// Default [`ApduBase`] impl for [`ApduEmpty`] APDUs
impl <'a, T: ApduEmpty> ApduBase<'a> for T {
    /// Encode APDU into the provided buffer
    fn encode(&self, _buff: &mut [u8]) -> usize {
        0
    }

    /// Decode APDU from the provided buffer
    fn decode(_buff: &'a [u8]) -> Result<Self, ApduError> {
        Ok(Default::default())
    }
}


#[cfg(test)]
pub(crate) mod test {
    use super::*;

    /// Helper for APDU encode / decode tests
    pub fn encode_decode_apdu<'a, A: ApduBase<'a>>(buff: &'a mut [u8], apdu: &A) {
        let n = apdu.encode(buff);

        let decoded = A::decode(&buff[..n]).unwrap();

        assert_eq!(apdu, &decoded);
    }
}
