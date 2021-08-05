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
//! Generic APDU transport library for Ledger Nano S/X apps

#![deny(warnings, trivial_casts, trivial_numeric_casts)]
#![deny(unused_import_braces, unused_qualifications)]
#![deny(missing_docs)]

pub use ledger_apdu::{APDUAnswer, APDUCommand, APDUErrorCodes};

/// APDU Errors
pub mod errors;

#[cfg(target_arch = "wasm32")]
/// APDU Transport wrapper for JS/WASM transports
pub mod apdu_transport_wasm;

#[cfg(target_arch = "wasm32")]
pub use crate::apdu_transport_wasm::{APDUTransport, TransportWrapperTrait};

#[cfg(not(target_arch = "wasm32"))]
pub mod apdu_transport_native;

#[cfg(not(target_arch = "wasm32"))]
pub use crate::apdu_transport_native::APDUTransport;

#[cfg(not(target_arch = "wasm32"))]
pub use exchange::Exchange;

#[cfg(all(not(target_arch = "wasm32"), feature = "transport-hid"))]
pub use ledger::{TransportNativeHID, LedgerHIDError};
#[cfg(all(not(target_arch = "wasm32"), feature = "transport-tcp"))]
pub use ledger_tcp::{TransportTcp, TransportTcpError};
#[cfg(all(not(target_arch = "wasm32"), feature = "transport-zemu"))]
pub use ledger_zemu::{TransportZemuGrpc, TransportZemuHttp, LedgerZemuError};

#[cfg(not(target_arch = "wasm32"))]
pub mod exchange {
    //! Some implementation on transport typos for the Exchange trait
    use super::*;

    use trait_async::trait_async;

    use crate::errors::TransportError;

    use ledger_apdu::{APDUAnswer, APDUCommand};

    /// Use to talk to the ledger device
    #[trait_async]
    pub trait Exchange: Send + Sync {
        /// Use to talk to the ledger device
        async fn exchange(&self, command: &APDUCommand) -> Result<APDUAnswer, TransportError>;
    }

    #[cfg(feature = "transport-hid")]
    #[trait_async]
    impl Exchange for TransportNativeHID {
        async fn exchange(&self, command: &APDUCommand) -> Result<APDUAnswer, TransportError> {
            use futures::future;
            let call = self
                .exchange(command)
                .map_err(|_| TransportError::APDUExchangeError)?;
            future::ready(Ok(call)).await
        }
    }

    #[cfg(feature = "transport-zemu")]
    #[trait_async]
    impl Exchange for TransportZemuGrpc {
        async fn exchange(&self, command: &APDUCommand) -> Result<APDUAnswer, TransportError> {
            self.exchange(command)
                .await
                .map_err(|_| TransportError::APDUExchangeError)
        }
    }

    #[cfg(feature = "transport-zemu")]
    #[trait_async]
    impl Exchange for TransportZemuHttp {
        async fn exchange(&self, command: &APDUCommand) -> Result<APDUAnswer, TransportError> {
            self.exchange(command)
                .await
                .map_err(|_| TransportError::APDUExchangeError)
        }
    }

    #[cfg(feature = "transport-tcp")]
    #[trait_async]
    impl Exchange for TransportTcp {
        async fn exchange(&self, command: &APDUCommand) -> Result<APDUAnswer, TransportError> {
            use ledger_tcp::TransportTcp;
            TransportTcp::exchange(self, command)
                .await
                .map_err(|_| TransportError::APDUExchangeError)
        }
    }
}
