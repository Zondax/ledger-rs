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
use core::convert::{TryInto};

use encdec::{EncDec};
use snafu::prelude::*;

#[cfg(test)]
mod tests;

pub mod apdus;

mod error;
pub use error::{ApduError, ApduErrorCode};

/// Re-export Encode and Decode traits for consumers / implementers
pub use encdec::{Encode, Decode};

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

    /// Will attempt to interpret the error code as an [ApduErrorCode],
    /// returning the code as is otherwise
    pub fn error_code(&self) -> Result<ApduErrorCode, u16> {
        self.retcode.try_into().map_err(|_| self.retcode)
    }

    /// Returns the raw return code
    #[inline(always)]
    pub fn retcode(&self) -> u16 {
        self.retcode
    }
}

/// [`ApduBase`] marker trait for APDUs
pub trait ApduBase<'a>: Send + PartialEq + Debug + Encode<Error=ApduError> + Decode<'a, Output=Self, Error=ApduError> {}

impl <'a, T: Send + PartialEq + Debug + Encode<Error=ApduError> + Decode<'a, Output=Self, Error=ApduError>> ApduBase<'a> for T {}

/// [`ApduCmd`] implemented for APDU commands / requests
pub trait ApduCmd<'a>: ApduBase<'a> {
    /// Fetch APDU header for encoding
    fn header(&self) -> ApduHeader;
}


/// [`ApduStatic`] helper for static APDU command definitions
pub trait ApduStatic {
    /// Class ID for APDU commands
    const CLA: u8;

    /// Instruction ID for APDU commands
    const INS: u8;

    /// Fetch P1 value (defaults to `0`)
    fn p1(&self) -> u8 {
        0
    }

    /// Fetch P2 value (defaults to `0`)
    fn p2(&self) -> u8 {
        0
    }
}

/// Blanked [`ApduCmd`] implementation for [`ApduStatic`] types
impl <'a, T: ApduBase<'a> + ApduStatic> ApduCmd<'a> for T {
    fn header(&self) -> ApduHeader {
        ApduHeader{
            cla: T::CLA,
            ins: T::INS,
            p1: self.p1(),
            p2: self.p2(),
            len: self.encode_len().unwrap() as u8,
        }
    }
}
    

/// Length of encoded [`ApduHeader`]
pub const APDU_HDR_LEN: usize = 5;

/// APDU Header
pub struct ApduHeader {
    /// Class
    pub cla: u8,
    /// Instruction
    pub ins: u8,
    /// Parameter one
    pub p1: u8,
    /// Parameter two
    pub p2: u8,
    /// Encoded data length
    pub len: u8,
}


impl ApduHeader {
    /// Encode header to the provided buffer
    pub fn encode(&self, buff: &mut [u8]) {
        buff[0] = self.cla;
        buff[1] = self.ins;
        buff[2] = self.p1;
        buff[3] = self.p2;
        buff[4] = self.len;
    }

    /// Decode APDU header from the provided buffer
    pub fn decode(&self, buff: &[u8]) -> Self {
        Self{
            cla: buff[0],
            ins: buff[1],
            p1: buff[2],
            p2: buff[3],
            len: buff[4],
        }
    }
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;

    /// Helper for APDU encode / decode tests
    pub fn encode_decode_apdu<'a, A: ApduBase<'a>>(buff: &'a mut [u8], apdu: &A) {
        let n = apdu.encode(buff);

        assert_eq!(n, apdu.encode_len());

        let decoded = A::decode(&buff[..n]).unwrap();

        assert_eq!(apdu, &decoded);
    }
}
