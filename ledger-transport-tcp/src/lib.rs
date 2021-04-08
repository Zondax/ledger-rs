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

extern crate hidapi;
#[cfg(test)]
#[macro_use]
extern crate serial_test;

mod errors;
use byteorder::{BigEndian as BE, WriteBytesExt};
pub use errors::LedgerTcpError;
use ledger_apdu::{APDUAnswer, APDUCommand};
use std::result::Result;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use lazy_static::lazy_static;

use std::sync::{Mutex, Arc};

lazy_static! {
    static ref STREAM: Arc<Mutex<Option<TcpStream>>> =
        Arc::new(Mutex::new(None));
}


pub struct TransportTcp {}

impl TransportTcp {
    pub async fn new() -> Result<Self, errors::LedgerTcpError> {
        // test the connection but don't bother storing it
        TcpStream::connect("127.0.0.1:9999")
            .await
            .map_err(|_| LedgerTcpError::connection_refused())?;


        Ok(TransportTcp {})
    }
    pub async fn exchange(&self, command: &APDUCommand) -> Result<APDUAnswer, LedgerTcpError> {
        let mut stream =         TcpStream::connect("127.0.0.1:9999")
            .await
            .map_err(|_| LedgerTcpError::connection_refused())?;
        let payload = command.serialize();

        let command_length = payload.len() as usize;
        let mut data = Vec::with_capacity(command_length + 4);
        WriteBytesExt::write_u32::<BE>(&mut data, command_length as u32)?;
        data.extend_from_slice(payload.as_slice());

        stream.write_all(&data).await?;
        // Wait for the socket to be readable
        stream.readable().await?;

        let mut buf: [u8; 256] = [0; 256];
        // Try to read data, this may still fail with `WouldBlock`
        // if the readiness event is a false positive.
        match stream.try_read(&mut buf) {

            Ok(0) => Err(LedgerTcpError::connection_closed()),
            Ok(n) => {
                let _packet_len = u32::from_be_bytes([buf[0],buf[1],buf[2],buf[3]]) as usize;
                let apdu_frame = buf[4..n].to_vec();

                Ok(APDUAnswer::from_answer(apdu_frame))
            },
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                Err(LedgerTcpError::read_would_block())
            }
            Err(e) => Err(e.into()),
        }
    }
}
