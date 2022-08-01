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
//! Generic APDU transport library for Ledger Nano S/X apps

#![deny(trivial_casts, trivial_numeric_casts)]
#![deny(unused_import_braces, unused_qualifications)]
#![deny(missing_docs)]

use core::fmt::Debug;

pub use async_trait::async_trait;
pub use ledger_apdu::{ApduCmd, ApduBase, APDUErrorCode};

/// Use to talk to the ledger device
#[async_trait]
pub trait Exchange {
    /// Error defined by Transport used
    type Error: Debug;

    /// Send a command with the given transport and retrieve an answer or a transport error.
    /// 
    /// The provided buffer is used for TX and RX operations to mitigate the need for allocation
    async fn exchange<'a, CMD: ApduCmd<'a>, ANS: ApduBase<'a>>(
        &self,
        command: CMD,
        buff: &'a mut [u8],
    ) -> Result<ANS, Self::Error>;
}
