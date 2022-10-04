//! Shared APDU definitions

use core::fmt::Debug;

use encdec::{Encode, Decode};

mod app_info;
pub use app_info::*;

mod version;
pub use version::*;

mod device_info;
pub use device_info::*;

use crate::{ApduCmd, ApduHeader, ApduError};

/// Empty APDU for exchanges where no response data is expected
#[derive(Clone, PartialEq, Default, Debug, Encode, Decode)]
#[encdec(error="ApduError")]
pub struct Empty;


/// Generic APDU passes through slices for manual construction of messages
#[derive(Clone, PartialEq, Debug)]
pub struct Generic<'a>{
    /// APDU application class
    pub cla: u8,
    /// APDU instruction
    pub ins: u8,
    /// p1 value if set
    pub p1: u8,
    /// p2 value if set
    pub p2: u8,
    /// Slice of data to send
    pub data: &'a [u8],
}

impl <'a> Generic<'a> {
    /// Create a new generic APDU
    pub fn new(cla: u8, ins: u8, p1: u8, p2: u8, data: &'a [u8]) -> Self {
        Self{ cla, ins, p1, p2, data }
    }
}

impl <'a> Encode for Generic<'a> {
    type Error = ApduError;
    
    fn encode(&self, buff: &mut [u8]) -> Result<usize, ApduError> {
        if buff.len() < self.data.len() {
            return Err(ApduError::InvalidLength);
        }

        buff[..self.data.len()].copy_from_slice(&self.data);

        Ok(self.data.len())
    }

    fn encode_len(&self) -> Result<usize, ApduError> {
        Ok(self.data.len())
    }
}


impl <'a> Decode<'a> for Generic<'a> {
    type Output = Self;
    type Error = ApduError;

    fn decode(buff: &'a [u8]) -> Result<(Self::Output, usize), Self::Error> {
        Ok((Self{data: buff, cla: 0, ins: 0, p1: 0, p2: 0}, 0))
    }
}

impl <'a> ApduCmd<'a> for Generic<'a> {
    fn header(&self) -> ApduHeader {
        ApduHeader { 
            cla: self.cla, 
            ins: self.ins, 
            p1: self.p1, 
            p2: self.p2, 
            len: self.encode_len().unwrap() as u8,
        }
    }
}
