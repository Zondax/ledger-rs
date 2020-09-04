/*******************************************************************************
*   (c) 2020 Zondax GmbH
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
//! Support library for Ledger Nano S/X apps developed by Zondax

#![deny(warnings, trivial_casts, trivial_numeric_casts)]
#![deny(unused_import_braces, unused_qualifications)]
#![deny(missing_docs)]

use crate::LedgerAppError;
use ledger_apdu::{map_apdu_error_description, APDUAnswer, APDUCommand, APDUErrorCodes};
use ledger_transport::errors::TransportError;
use ledger_transport::{APDUTransport, Exchange};
use serde::{Deserialize, Serialize};
use std::str;

const INS_GET_VERSION: u8 = 0x00;
const CLA_APP_INFO: u8 = 0xb0;
const INS_APP_INFO: u8 = 0x01;
const CLA_DEVICE_INFO: u8 = 0xe0;
const INS_DEVICE_INFO: u8 = 0x01;
const USER_MESSAGE_CHUNK_SIZE: usize = 250;

/// Chunk payload type
pub enum ChunkPayloadType {
    /// First chunk
    Init = 0x00,
    /// Append chunk
    Add = 0x01,
    /// Last chunk
    Last = 0x02,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
/// App Version
pub struct Version {
    /// Application Mode
    #[serde(rename(serialize = "testMode"))]
    pub mode: u8,
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

#[derive(Clone, Debug, Deserialize, Serialize)]
/// App Information
pub struct AppInfo {
    /// Name of the application
    #[serde(rename(serialize = "appName"))]
    pub app_name: String,
    /// App version
    #[serde(rename(serialize = "appVersion"))]
    pub app_version: String,
    /// Flag length
    #[serde(rename(serialize = "flagLen"))]
    pub flag_len: u8,
    /// Flag value
    #[serde(rename(serialize = "flagsValue"))]
    pub flags_value: u8,
    /// Flag Recovery
    #[serde(rename(serialize = "flagsRecovery"))]
    pub flag_recovery: bool,
    /// Flag Signed MCU code
    #[serde(rename(serialize = "flagsSignedMCUCode"))]
    pub flag_signed_mcu_code: bool,
    /// Flag Onboarded
    #[serde(rename(serialize = "flagsOnboarded"))]
    pub flag_onboarded: bool,
    /// Flag Pin Validated
    #[serde(rename(serialize = "flagsPINValidated"))]
    pub flag_pin_validated: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
/// App Device Info
pub struct DeviceInfo {
    /// Target ID
    #[serde(rename(serialize = "targetId"))]
    pub target_id: [u8; 4],
    /// Secure Element Version
    #[serde(rename(serialize = "seVersion"))]
    pub se_version: String,
    /// Device Flag
    pub flag: Vec<u8>,
    /// MCU Version
    #[serde(rename(serialize = "mcuVersion"))]
    pub mcu_version: String,
}

/// Retrieve the device info
pub async fn get_device_info<T: Exchange>(
    apdu_transport: &APDUTransport<T>,
) -> Result<DeviceInfo, LedgerAppError> {
    let command = APDUCommand {
        cla: CLA_DEVICE_INFO,
        ins: INS_DEVICE_INFO,
        p1: 0x00,
        p2: 0x00,
        data: Vec::new(),
    };

    let response = apdu_transport.exchange(&command).await?;
    if response.retcode != APDUErrorCodes::NoError as u16 {
        return Err(LedgerAppError::TransportError(
            TransportError::APDUExchangeError,
        ));
    }

    let target_id_slice = &response.data[0..4];
    let mut idx = 4;
    let se_version_len: usize = response.data[idx] as usize;
    idx += 1;
    let se_version_bytes = &response.data[idx..idx + se_version_len];

    idx += se_version_len;

    let flags_len: usize = response.data[idx] as usize;
    idx += 1;
    let flag = &response.data[idx..idx + flags_len];
    idx += flags_len;

    let mcu_version_len: usize = response.data[idx] as usize;
    idx += 1;
    let mut tmp = &response.data[idx..idx + mcu_version_len];
    if tmp[mcu_version_len - 1] == 0 {
        tmp = &response.data[idx..idx + mcu_version_len - 1];
    }

    let mut target_id = [Default::default(); 4];
    target_id.copy_from_slice(target_id_slice);

    let se_version = str::from_utf8(se_version_bytes).map_err(|_e| LedgerAppError::Utf8)?;
    let mcu_version = str::from_utf8(tmp).map_err(|_e| LedgerAppError::Utf8)?;

    let device_info = DeviceInfo {
        target_id,
        se_version: se_version.to_string(),
        flag: flag.to_vec(),
        mcu_version: mcu_version.to_string(),
    };

    Ok(device_info)
}

/// Retrieve the app info
pub async fn get_app_info<T: Exchange>(
    apdu_transport: &APDUTransport<T>,
) -> Result<AppInfo, LedgerAppError> {
    let command = APDUCommand {
        cla: CLA_APP_INFO,
        ins: INS_APP_INFO,
        p1: 0x00,
        p2: 0x00,
        data: Vec::new(),
    };

    let response = apdu_transport.exchange(&command).await?;
    if response.retcode != APDUErrorCodes::NoError as u16 {
        return Err(LedgerAppError::AppSpecific(
            response.retcode,
            map_apdu_error_description(response.retcode).to_string(),
        ));
    }

    if response.data[0] != 1 {
        return Err(LedgerAppError::InvalidFormatID);
    }

    let app_name_len: usize = response.data[1] as usize;
    let app_name_bytes = &response.data[2..app_name_len];

    let mut idx = 2 + app_name_len;
    let app_version_len: usize = response.data[idx] as usize;
    idx += 1;
    let app_version_bytes = &response.data[idx..idx + app_version_len];

    idx += app_version_len;

    let app_flags_len = response.data[idx];
    idx += 1;
    let flags_value = response.data[idx];

    let app_name = str::from_utf8(app_name_bytes).map_err(|_e| LedgerAppError::Utf8)?;
    let app_version = str::from_utf8(app_version_bytes).map_err(|_e| LedgerAppError::Utf8)?;

    let app_info = AppInfo {
        app_name: app_name.to_string(),
        app_version: app_version.to_string(),
        flag_len: app_flags_len,
        flags_value,
        flag_recovery: (flags_value & 1) != 0,
        flag_signed_mcu_code: (flags_value & 2) != 0,
        flag_onboarded: (flags_value & 4) != 0,
        flag_pin_validated: (flags_value & 128) != 0,
    };

    Ok(app_info)
}

/// Retrieve the app version
pub async fn get_version<T: Exchange>(
    cla: u8,
    apdu_transport: &APDUTransport<T>,
) -> Result<Version, LedgerAppError> {
    let command = APDUCommand {
        cla,
        ins: INS_GET_VERSION,
        p1: 0x00,
        p2: 0x00,
        data: Vec::new(),
    };

    let response = apdu_transport.exchange(&command).await?;
    if response.retcode != APDUErrorCodes::NoError as u16 {
        return Err(LedgerAppError::InvalidVersion);
    }

    let version = match response.data.len() {
        // single byte version numbers
        4 => Version {
            mode: response.data[0],
            major: response.data[1] as u16,
            minor: response.data[2] as u16,
            patch: response.data[3] as u16,
            locked: false,
            target_id: [0, 0, 0, 0],
        },
        // double byte version numbers
        7 => Version {
            mode: response.data[0],
            major: response.data[1] as u16 * 256 + response.data[2] as u16,
            minor: response.data[3] as u16 * 256 + response.data[4] as u16,
            patch: response.data[5] as u16 * 256 + response.data[6] as u16,
            locked: false,
            target_id: [0, 0, 0, 0],
        },
        // double byte version numbers + lock + target id
        9 => Version {
            mode: response.data[0],
            major: response.data[1] as u16,
            minor: response.data[2] as u16,
            patch: response.data[3] as u16,
            locked: response.data[4] != 0,
            target_id: [
                response.data[5],
                response.data[6],
                response.data[7],
                response.data[8],
            ],
        },
        // double byte version numbers + lock + target id
        12 => Version {
            mode: response.data[0],
            major: response.data[1] as u16 * 256 + response.data[2] as u16,
            minor: response.data[3] as u16 * 256 + response.data[4] as u16,
            patch: response.data[5] as u16 * 256 + response.data[6] as u16,
            locked: response.data[7] != 0,
            target_id: [
                response.data[8],
                response.data[9],
                response.data[10],
                response.data[11],
            ],
        },
        _ => return Err(LedgerAppError::InvalidVersion),
    };
    Ok(version)
}

/// Stream a long request in chunks
pub async fn send_chunks<T: Exchange>(
    apdu_transport: &APDUTransport<T>,
    start_command: &APDUCommand,
    message: &[u8],
) -> Result<APDUAnswer, LedgerAppError> {
    let chunks = message.chunks(USER_MESSAGE_CHUNK_SIZE);
    match chunks.len() {
        0 => return Err(LedgerAppError::InvalidEmptyMessage),
        n if n > 255 => return Err(LedgerAppError::InvalidMessageSize),
        _ => (),
    }

    if start_command.p1 != ChunkPayloadType::Init as u8 {
        return Err(LedgerAppError::InvalidChunkPayloadType);
    }

    let mut response = apdu_transport.exchange(start_command).await?;
    if response.retcode != 0x9000 {
        return Err(LedgerAppError::AppSpecific(
            response.retcode,
            map_apdu_error_description(response.retcode).to_string(),
        ));
    }

    // Send message chunks
    let last_chunk_index = chunks.len() - 1;
    for (packet_idx, chunk) in chunks.enumerate() {
        let mut p1 = ChunkPayloadType::Add as u8;
        if packet_idx == last_chunk_index {
            p1 = ChunkPayloadType::Last as u8
        }

        let command = APDUCommand {
            cla: start_command.cla,
            ins: start_command.ins,
            p1,
            p2: 0,
            data: chunk.to_vec(),
        };

        response = apdu_transport.exchange(&command).await?;
        if response.retcode != 0x9000 {
            return Err(LedgerAppError::AppSpecific(
                response.retcode,
                map_apdu_error_description(response.retcode).to_string(),
            ));
        }
    }

    Ok(response)
}
