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
use super::*;

const SERIALIZED_APDU: &[u8] = &[0xFF, 0x00, 0, 0, 3, 0x42, 0x42, 0x42];
const APDU_RESPONSE: &[u8] = &[0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x90];

#[test]
#[cfg(feature = "std")]
fn apdu_command_vec() {
    let data = std::vec![SERIALIZED_APDU[5]; 3];

    let command = APDUCommand {
        cla: 0xFF,
        ins: 0x00,
        p1: 0,
        p2: 0,
        data,
    };

    assert_eq!(SERIALIZED_APDU, &command.serialize()[..])
}

#[test]
fn apdu_command_slice() {
    let data = &SERIALIZED_APDU[5..];

    let _ = APDUCommand {
        cla: 0xFF,
        ins: 0x00,
        p1: 0,
        p2: 0,
        data,
    };
}

#[test]
fn apdu_answer_success() {
    let answer = APDUAnswer::from_answer(APDU_RESPONSE).expect("valid answer length >= 2");

    let code = answer.error_code().expect("valid error code");
    assert_eq!(code, APDUErrorCode::NoError);

    assert_eq!(answer.apdu_data(), &APDU_RESPONSE[..4]);
}

#[test]
fn apdu_answer_vec() {
    let answer = APDUAnswer::from_answer(APDU_RESPONSE.to_vec()).expect("valid answer length >= 2");

    let code = answer.error_code().expect("valid error code");
    assert_eq!(code, APDUErrorCode::NoError);

    assert_eq!(answer.apdu_data(), &APDU_RESPONSE[..4]);
}

#[test]
fn apdu_answer_error() {
    let answer = APDUAnswer::from_answer(&[0x00, 0x64][..]).expect("valid answer length >= 2");

    let code = answer.error_code().expect("valid error code");
    assert_eq!(code, APDUErrorCode::ExecutionError);

    assert_eq!(answer.apdu_data(), &[]);
}

#[test]
fn apdu_answer_unknown() {
    let answer = APDUAnswer::from_answer(&APDU_RESPONSE[..4]).expect("valid answer length >= 2");

    let code = answer.error_code().expect_err("invalid error code");
    assert_eq!(code, 0xEFBE);

    assert_eq!(answer.apdu_data(), &[0xDE, 0xAD]);
}

#[test]
fn apdu_answer_too_short() {
    let answer = APDUAnswer::from_answer(&[][..]).expect_err("empty answer");

    assert_eq!(answer, APDUAnswerError::TooShort);
}
