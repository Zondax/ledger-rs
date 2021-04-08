/*******************************************************************************
*   (c) 2020 Helium Systems, Inc
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
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LedgerTcpError {
    /// Connection refused error
    #[error("TCP Connection Refused")]
    ConnectionRefused,
    #[error("TCP io error")]
    Io(#[from] std::io::Error),
    #[error("TCP Connection Closed")]
    ConnectionClosed,
    #[error("TCP Read Would Block")]
    ReadWouldBlock,
}

impl LedgerTcpError {
    pub(crate) fn connection_refused() -> LedgerTcpError {
        LedgerTcpError::ConnectionRefused
    }
    pub(crate) fn connection_closed() -> LedgerTcpError {
        LedgerTcpError::ConnectionClosed
    }
    pub(crate) fn read_would_block() -> LedgerTcpError {
        LedgerTcpError::ReadWouldBlock
    }

}
