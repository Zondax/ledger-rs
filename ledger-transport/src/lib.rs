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
//! Support library for Filecoin Ledger Nano S/X apps

#![deny(warnings, trivial_casts, trivial_numeric_casts)]
#![deny(unused_import_braces, unused_qualifications)]
#![deny(missing_docs)]

extern crate byteorder;

pub use ledger_generic::{APDUAnswer, APDUCommand, APDUErrorCodes};

/// APDU Errors
pub mod errors;

#[cfg(target_arch = "wasm32")]
/// APDU Transport wrapper for JS/WASM transports
pub mod apdu_transport_wasm;

#[cfg(target_arch = "wasm32")]
pub use crate::apdu_transport_wasm::{APDUTransport, TransportWrapperTrait};

#[cfg(not(target_arch = "wasm32"))]
/// APDU Errors
pub mod apdu_transport_native;

#[cfg(not(target_arch = "wasm32"))]
pub use crate::apdu_transport_native::APDUTransport;
