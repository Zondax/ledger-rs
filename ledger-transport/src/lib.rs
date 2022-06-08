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

use std::ops::Deref;

pub use async_trait::async_trait;
pub use ledger_apdu::{APDUAnswer, APDUCommand, APDUErrorCode};

/// Use to talk to the ledger device
#[async_trait]
pub trait Exchange {
    /// Error defined by Transport used
    type Error;

    /// The concrete type containing the APDUAnswer
    type AnswerType: Deref<Target = [u8]> + Send;

    /// Send a command with the given transport and retrieve an answer or a transport error
    async fn exchange<I>(
        &self,
        command: &APDUCommand<I>,
    ) -> Result<APDUAnswer<Self::AnswerType>, Self::Error>
    where
        I: Deref<Target = [u8]> + Send + Sync;
}
