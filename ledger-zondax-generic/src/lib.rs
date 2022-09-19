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
pub use errors::*;

use async_trait::async_trait;

use ledger_apdu::{
    apdus::{
        DeviceInfo, DeviceInfoGet,
        AppInfo, AppInfoGet,
        Generic, Empty,
    }, ApduCmd, ApduBase,
};
use ledger_transport::Exchange;

/// Maximum size of a user message
pub const USER_MESSAGE_CHUNK_SIZE: usize = 250;

/// Maximum size of an encode APDU (based on u8 len field)
pub const APDU_LEN_MAX: usize = 256;

/// Chunk payload type
pub enum ChunkPayloadType {
    /// First chunk
    Init = 0x00,
    /// Append chunk
    Add = 0x01,
    /// Last chunk
    Last = 0x02,
}

/// Defines what we can consider an "App" with a specified "CLA"
pub trait App {
    /// Application APDU class
    const CLA: u8;
}

#[async_trait]
/// Common commands for any given APP
///
/// This trait is automatically implemented for any type that implements [App]
pub trait AppExt<E>
where
    E: Exchange + Send + Sync,
    E::Error: std::error::Error,
{
    /// Retrieve the device info
    ///
    /// Works only when the device is on the dashboard
    async fn get_device_info<'a>(transport: &E, buff: &'a mut[u8]) -> Result<DeviceInfo<'a>, LedgerAppError<E::Error>> {
        transport.exchange::<DeviceInfo>(DeviceInfoGet{}, buff).await
            .map_err(LedgerAppError::TransportError)
    }

    /// Retrieve the app info
    ///
    /// Works only in app (TODO: dashboard support)
    async fn get_app_info<'a>(transport: &E, buff: &'a mut[u8]) -> Result<AppInfo<'a>, LedgerAppError<E::Error>> {
        transport.exchange::<AppInfo>(AppInfoGet{}, buff).await
            .map_err(LedgerAppError::TransportError)
    }

    /// Retrieve the app version
    #[cfg(nyet)]
    async fn get_version<'a>(transport: &E, buff: &'a mut[u8]) -> Result<Version, LedgerAppError<E::Error>> {
        transport.exchange::<Version>(VersionGet{cla: Self::CLA}, buff).await
            .map_err(LedgerAppError::TransportError)
    }

    /// Stream a long request in chunks
    async fn send_chunks<'a, 'c, ANS: ApduBase<'a>>(
        transport: impl Exchange<Error=E::Error> + Sync,
        command: impl ApduCmd<'c>,
        message: &[u8],
        buff: &'a mut [u8],
    ) -> Result<ANS, LedgerAppError<E::Error>> {
        send_chunks_inner::<ANS, _>(transport, command, message, buff).await
    }
}

impl<T, E> AppExt<E> for T
where
    T: App,
    E: Exchange + Send + Sync,
    E::Error: std::error::Error,
{
}


/// Stream a long request in chunks
// TODO: we can't adapt between const generics -yet- so this type signature is a little more gnarley than would be ideal... should improve in the next month or so

async fn send_chunks_inner<'a, 'c, ANS, ERR>(
    transport: impl Exchange<Error=ERR>,
    command: impl ApduCmd<'c>,
    message: &[u8],
    buff: &'a mut [u8],
) -> Result<ANS, LedgerAppError<ERR>> 
where
    ANS: ApduBase<'a>,
    ERR: std::error::Error,
{
    let mut b = [0u8; APDU_LEN_MAX];

    // Compute chunks for streaming
    let chunks = message.chunks(USER_MESSAGE_CHUNK_SIZE);
    match chunks.len() {
        0 => return Err(LedgerAppError::InvalidEmptyMessage),
        n if n > 255 => return Err(LedgerAppError::InvalidMessageSize),
        _ => (),
    }

    let hdr = command.header();
    if hdr.p1 != ChunkPayloadType::Init as u8 {
        return Err(LedgerAppError::InvalidChunkPayloadType);
    }


    // Write first / init command
    transport.exchange::<Empty>(command, &mut b[..]).await
        .map_err(LedgerAppError::TransportError)?;


    // Send message chunks
    let last_chunk_index = chunks.len() - 1;
    for (packet_idx, chunk) in chunks.enumerate() {

        let mut p1 = ChunkPayloadType::Add as u8;
        if packet_idx == last_chunk_index {
            p1 = ChunkPayloadType::Last as u8
        }

        let command = Generic::new(hdr.cla, hdr.ins, p1, 0, chunk);

        // Parse answer to final packet
        if packet_idx < last_chunk_index {
            let _ = transport.exchange::<Empty>(command, &mut b[..]).await
                .map_err(LedgerAppError::TransportError)?;
        } else {
            let response = transport.exchange::<ANS>(command, &mut buff[..]).await
                .map_err(LedgerAppError::TransportError)?;
            
            return Ok(response);
        }
    }

    unreachable!()
}
