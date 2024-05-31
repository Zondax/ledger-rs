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
//! Support library for Ledger Nano S/X apps developed by Zondax
//!
//! Contains common commands

#![deny(warnings, trivial_casts)]
#![deny(unused_import_braces, unused_qualifications)]
#![deny(missing_docs)]

mod errors;
use std::str;

use async_trait::async_trait;
pub use errors::*;
use ledger_transport::{APDUAnswer, APDUCommand, APDUErrorCode, Exchange};
use serde::{Deserialize, Serialize};

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

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
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

/// Defines what we can consider an "App"
pub trait App {
    /// App's APDU CLA
    const CLA: u8;
}

#[async_trait]
/// Common commands for any given APP
///
/// This trait is automatically implemented for any type that implements [App]
pub trait AppExt<E>: App
where
    E: Exchange + Send + Sync,
    E::Error: std::error::Error,
{
    /// Handles the error response from APDU exchange.
    ///
    /// # Arguments
    /// * `response` - The response from the APDU exchange.
    ///
    /// # Returns
    /// A result indicating success or containing a specific ledger application error.
    fn handle_response_error(response: &APDUAnswer<E::AnswerType>) -> Result<(), LedgerAppError<E::Error>> {
        match response.error_code() {
            Ok(APDUErrorCode::NoError) => Ok(()),
            Ok(err) => Err(LedgerAppError::AppSpecific(err as _, err.description())),
            Err(err) => Err(LedgerAppError::Unknown(err)),
        }
    }

    /// Handles the error response from APDU exchange.
    ///
    /// # Arguments
    /// * `response` - The response from the APDU exchange.
    ///
    /// # Returns
    /// A result indicating success or containing a specific ledger application error.
    fn handle_response_error_signature(response: &APDUAnswer<E::AnswerType>) -> Result<(), LedgerAppError<E::Error>> {
        match response.error_code() {
            Ok(APDUErrorCode::NoError) if response.data().is_empty() => Err(LedgerAppError::NoSignature),
            Ok(APDUErrorCode::NoError) => Ok(()),
            Ok(err) => Err(LedgerAppError::AppSpecific(err as _, err.description())),
            Err(err) => Err(LedgerAppError::AppSpecific(err, "[APDU_ERROR] Unknown".to_string())),
        }
    }

    /// Retrieve the device info
    ///
    /// Works only in the dashboard
    async fn get_device_info(transport: &E) -> Result<DeviceInfo, LedgerAppError<E::Error>> {
        let command = APDUCommand { cla: CLA_DEVICE_INFO, ins: INS_DEVICE_INFO, p1: 0x00, p2: 0x00, data: Vec::new() };

        let response = transport.exchange(&command).await?;
        match response.error_code() {
            Ok(APDUErrorCode::NoError) => {},
            Ok(err) => return Err(LedgerAppError::Unknown(err as _)),
            Err(err) => return Err(LedgerAppError::Unknown(err)),
        }

        let response_data = response.data();

        let target_id_slice = &response_data[0 .. 4];
        let mut idx = 4;
        let se_version_len: usize = response_data[idx] as usize;
        idx += 1;
        let se_version_bytes = &response_data[idx .. idx + se_version_len];

        idx += se_version_len;

        let flags_len: usize = response_data[idx] as usize;
        idx += 1;
        let flag = &response_data[idx .. idx + flags_len];
        idx += flags_len;

        let mcu_version_len: usize = response_data[idx] as usize;
        idx += 1;
        let mut tmp = &response_data[idx .. idx + mcu_version_len];
        if tmp[mcu_version_len - 1] == 0 {
            tmp = &response_data[idx .. idx + mcu_version_len - 1];
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
    ///
    /// Works only in app (TOOD: dashboard support)
    async fn get_app_info(transport: &E) -> Result<AppInfo, LedgerAppError<E::Error>> {
        let command = APDUCommand { cla: CLA_APP_INFO, ins: INS_APP_INFO, p1: 0x00, p2: 0x00, data: Vec::new() };

        let response = transport.exchange(&command).await?;
        match response.error_code() {
            Ok(APDUErrorCode::NoError) => {},
            Ok(err) => return Err(LedgerAppError::AppSpecific(err as _, err.description())),
            Err(err) => return Err(LedgerAppError::Unknown(err as _)),
        }

        let response_data = response.data();

        if response_data[0] != 1 {
            return Err(LedgerAppError::InvalidFormatID);
        }

        let app_name_len: usize = response_data[1] as usize;
        let app_name_bytes = &response_data[2 .. app_name_len];

        let mut idx = 2 + app_name_len;
        let app_version_len: usize = response_data[idx] as usize;
        idx += 1;
        let app_version_bytes = &response_data[idx .. idx + app_version_len];

        idx += app_version_len;

        let app_flags_len = response_data[idx];
        idx += 1;
        let flags_value = response_data[idx];

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
    async fn get_version(transport: &E) -> Result<Version, LedgerAppError<E::Error>> {
        let command = APDUCommand { cla: Self::CLA, ins: INS_GET_VERSION, p1: 0x00, p2: 0x00, data: Vec::new() };

        let response = transport.exchange(&command).await?;
        match response.error_code() {
            Ok(APDUErrorCode::NoError) => {},
            Ok(err) => return Err(LedgerAppError::Unknown(err as _)),
            Err(err) => return Err(LedgerAppError::Unknown(err)),
        }

        let response_data = response.data();

        let version = match response_data.len() {
            // single byte version numbers
            4 => Version {
                mode: response_data[0],
                major: response_data[1] as u16,
                minor: response_data[2] as u16,
                patch: response_data[3] as u16,
                locked: false,
                target_id: [0, 0, 0, 0],
            },
            // double byte version numbers
            7 => Version {
                mode: response_data[0],
                major: response_data[1] as u16 * 256 + response_data[2] as u16,
                minor: response_data[3] as u16 * 256 + response_data[4] as u16,
                patch: response_data[5] as u16 * 256 + response_data[6] as u16,
                locked: false,
                target_id: [0, 0, 0, 0],
            },
            // double byte version numbers + lock + target id
            9 => Version {
                mode: response_data[0],
                major: response_data[1] as u16,
                minor: response_data[2] as u16,
                patch: response_data[3] as u16,
                locked: response_data[4] != 0,
                target_id: [response_data[5], response_data[6], response_data[7], response_data[8]],
            },
            // double byte version numbers + lock + target id
            12 => Version {
                mode: response_data[0],
                major: response_data[1] as u16 * 256 + response_data[2] as u16,
                minor: response_data[3] as u16 * 256 + response_data[4] as u16,
                patch: response_data[5] as u16 * 256 + response_data[6] as u16,
                locked: response_data[7] != 0,
                target_id: [response_data[8], response_data[9], response_data[10], response_data[11]],
            },
            _ => return Err(LedgerAppError::InvalidVersion),
        };
        Ok(version)
    }

    /// Stream a long request in chunks, with an option to set P2 for all chunks.
    /// This method combines the functionality of sending chunks with or without P2 set for all chunks.
    ///
    /// # Arguments
    /// * `transport` - The transport layer used for communication.
    /// * `command` - The initial APDU command.
    /// * `message` - The message to be sent in chunks.
    /// * `set_p2_for_all` - A boolean to determine if P2 should be set for all chunks.
    ///
    /// # Returns
    /// A result containing the final APDU answer or a ledger application error.
    async fn send_chunks<I: std::ops::Deref<Target = [u8]> + Send + Sync>(
        transport: &E,
        command: APDUCommand<I>,
        message: &[u8],
    ) -> Result<APDUAnswer<E::AnswerType>, LedgerAppError<E::Error>> {
        let chunks = message.chunks(USER_MESSAGE_CHUNK_SIZE);
        match chunks.len() {
            0 => return Err(LedgerAppError::InvalidEmptyMessage),
            n if n > 255 => return Err(LedgerAppError::InvalidMessageSize),
            _ => (),
        }

        if command.p1 != ChunkPayloadType::Init as u8 {
            return Err(LedgerAppError::InvalidChunkPayloadType);
        }

        let mut response = transport.exchange(&command).await?;
        Self::handle_response_error(&response)?;

        // Send message chunks
        let last_chunk_index = chunks.len() - 1;
        for (packet_idx, chunk) in chunks.enumerate() {
            let mut p1 = ChunkPayloadType::Add as u8;
            if packet_idx == last_chunk_index {
                p1 = ChunkPayloadType::Last as u8;
            }

            let command = APDUCommand { cla: command.cla, ins: command.ins, p1, p2: command.p2, data: chunk.to_vec() };

            response = transport.exchange(&command).await?;
            Self::handle_response_error(&response)?;
        }

        Ok(response)
    }
}

impl<T, E> AppExt<E> for T
where
    T: App,
    E: Exchange + Send + Sync,
    E::Error: std::error::Error,
{
}
