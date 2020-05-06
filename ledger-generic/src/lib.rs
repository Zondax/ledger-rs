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
use std::convert::Into;

#[derive(Debug)]
pub struct ApduCommand {
    pub cla: u8,
    pub ins: u8,
    pub p1: u8,
    pub p2: u8,
    pub length: u8,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct ApduAnswer {
    pub data: Vec<u8>,
    pub retcode: u16,
}

impl ApduCommand {
    pub fn serialize(&self) -> Vec<u8> {
        let mut v = vec![self.cla, self.ins, self.p1, self.p2, self.length];
        v.extend(&self.data);
        v
    }
}

impl ApduAnswer {
    pub fn from_answer(answer: Vec<u8>) -> ApduAnswer {
        let apdu_retcode =
            (u16::from(answer[answer.len() - 2]) << 8) + u16::from(answer[answer.len() - 1]);
        let apdu_data = &answer[..answer.len() - 2];

        return ApduAnswer {
            data: apdu_data.to_vec(),
            retcode: apdu_retcode,
        }
    }
}

#[derive(Copy, Clone)]
pub enum APDUErrorCodes {
    NoError = 0x9000,
}

impl Into<u16> for APDUErrorCodes {
    fn into(self) -> u16 {
        self as u16
    }
}
