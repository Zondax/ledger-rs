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
use ledger_transport::errors::TransportError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// App Error
#[derive(Clone, Debug, Eq, Error, PartialEq, Deserialize, Serialize)]
pub enum LedgerAppError {
    /// Invalid version error
    #[error("This version is not supported")]
    InvalidVersion,
    /// The message cannot be empty
    #[error("message cannot be empty")]
    InvalidEmptyMessage,
    /// Invalid payload type in chunk
    #[error("The chunk payload type was invalid. First message should be Init")]
    InvalidChunkPayloadType,
    /// The size fo the message to sign is invalid
    #[error("message size is invalid (too big)")]
    InvalidMessageSize,
    /// Public Key is invalid
    #[error("received an invalid PK")]
    InvalidPK,
    /// No signature has been returned
    #[error("received no signature back")]
    NoSignature,
    /// The signature is not valid
    #[error("received an invalid signature")]
    InvalidSignature,
    /// The derivation is invalid
    #[error("invalid derivation path")]
    InvalidDerivationPath,
    /// The derivation is invalid
    #[error("Transport | {0}")]
    TransportError(#[from] TransportError),
    /// Crypto related errors
    #[error("Crypto")]
    Crypto,
    /// Utf8 related errors
    #[error("Utf8 conversion error")]
    Utf8,
    /// Format ID error
    #[error("response format ID not recognized")]
    InvalidFormatID,
    /// HexEncode
    #[error("Couldn't encode string to HEX")]
    HexEncode,
    /// Application specific error
    #[error("App Error: | {0} {1}")]
    AppSpecific(u16, String),
}
