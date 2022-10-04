use core::convert::{TryFrom};

use snafu::Snafu;
use strum::{Display, EnumString};

/// APDU encode / decode errors
#[derive(Copy, Clone, PartialEq, Debug, Display, EnumString)]
#[cfg_attr(feature = "thiserror", derive(thiserror::Error))]
pub enum ApduError {
    /// Invalid version / format identifier
    InvalidVersion(u8),
    /// Invalid UTF8 in string component
    Utf8,
    /// Invalid object length
    InvalidLength,
    /// Invalid object encoding
    InvalidEncoding,
    /// Error response from device (may be an [`ApduErrorCode`])
    ErrorCode(u16),
}


#[derive(Copy, Clone, Debug, Snafu, PartialEq, Eq, EnumString)]
#[repr(u16)]
/// Common known APDU error codes
pub enum ApduErrorCode {
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

impl From<ApduErrorCode> for u16 {
    fn from(code: ApduErrorCode) -> Self {
        code as u16
    }
}

impl TryFrom<u16> for ApduErrorCode {
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

impl From<encdec::Error> for ApduError {
    fn from(e: encdec::Error) -> Self {
        match e {
            encdec::Error::Length => ApduError::InvalidLength,
            encdec::Error::Utf8 => ApduError::Utf8,
        }
    }
}