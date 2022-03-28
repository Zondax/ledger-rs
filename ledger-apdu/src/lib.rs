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
//! This crate contains a couple of utilities to talk via the APDU protocol to Ledger devices

#![no_std]
#![deny(missing_docs)]

extern crate no_std_compat as std;
use std::convert::{TryFrom, TryInto};

use snafu::prelude::*;

#[cfg(test)]
mod tests;

#[derive(Debug)]
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
    B: core::ops::Deref<Target = [u8]>,
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
        let retcode = u16::from_le_bytes(*retcode);

        Ok(APDUAnswer {
            data: answer,
            retcode,
        })
    }

    /// Will return the answer's payload
    pub fn apdu_data(&self) -> &[u8] {
        &self.data[..self.data.len() - 2]
    }

    /// Will attempt to interpret the error code as an [APDUErrorCode],
    /// returning the code as is otherwise
    pub fn error_code(&self) -> Result<APDUErrorCode, u16> {
        self.retcode.try_into().map_err(|_| self.retcode)
    }
}

#[derive(Copy, Clone, Debug, Snafu, PartialEq, Eq)]
#[repr(u16)]
/// Common known APDU error codes
pub enum APDUErrorCode {
    ///success
    NoError = 0x9000,
    ///error during apdu execution
    ExecutionError = 0x6400,
    ///apdu command wrong length
    WrongLength = 0x6700,
    ///empty apdu buffer
    EmptyBuffer = 0x6982,
    ///apdu buffer too small
    OutputBufferTooSmall = 0x6983,
    ///apdu parameters invalid
    DataInvalid = 0x6984,
    ///apdu preconditions not satisfied
    ConditionsNotSatisfied = 0x6985,
    ///apdu command not allowed
    CommandNotAllowed = 0x6986,
    ///apdu data field incorrect (bad key)
    BadKeyHandle = 0x6A80,
    ///apdu p1 or p2 incorrect
    InvalidP1P2 = 0x6B00,
    ///apdu instruction not supported or invalid
    InsNotSupported = 0x6D00,
    ///apdu class not supported or invalid
    ClaNotSupported = 0x6E00,
    ///unknown apdu error
    Unknown = 0x6F00,
    ///apdu sign verify error
    SignVerifyError = 0x6F01,
}

impl From<APDUErrorCode> for u16 {
    fn from(code: APDUErrorCode) -> Self {
        code as u16
    }
}

impl TryFrom<u16> for APDUErrorCode {
    type Error = ();

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        let this = match value {
            0x9000 => Self::NoError,
            0x6400 => Self::ExecutionError,
            0x6700 => Self::WrongLength,
            0x6982 => Self::EmptyBuffer,
            0x6983 => Self::OutputBufferTooSmall,
            0x6984 => Self::DataInvalid,
            0x6985 => Self::ConditionsNotSatisfied,
            0x6986 => Self::CommandNotAllowed,
            0x6A80 => Self::BadKeyHandle,
            0x6B00 => Self::InvalidP1P2,
            0x6D00 => Self::InsNotSupported,
            0x6E00 => Self::ClaNotSupported,
            0x6F00 => Self::Unknown,
            0x6F01 => Self::SignVerifyError,
            _ => return Err(()),
        };

        Ok(this)
    }
}
