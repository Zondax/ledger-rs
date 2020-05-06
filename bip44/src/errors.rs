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
use std::num::ParseIntError;
use thiserror::Error;


/// BIP44Path Error
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum BIP44PathError {
    /// Invalid length for a Bip 44 path
    #[error("BIP44Path error : Invalid length for path")]
    InvalidLength,
    /// Bip 44 path string is missing the `m` prefix
    #[error("BIP44Path error : Path should start with `m`")]
    MissingPrefix,
    /// Not able to parse integer
    #[error("Cannot parse integer")]
    ParseIntError(#[from] ParseIntError),
}
