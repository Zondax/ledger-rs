/*******************************************************************************
*   (c) 2022 Zondax GmbH
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

include!(env!("PROTO_MOD_RS"));

use grpc::prelude::*;
use grpc::ClientConf;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE};
use reqwest::{Client as HttpClient, Response};
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::time::Duration;

use zemu::{ExchangeReply, ExchangeRequest};
use zemu_grpc::ZemuCommandClient;

use ledger_transport::{async_trait, APDUAnswer, APDUCommand, Exchange};

mod errors;
pub use errors::LedgerZemuError;

pub struct TransportZemuGrpc {
    client: ZemuCommandClient,
}

impl TransportZemuGrpc {
    pub fn new(host: &str, port: u16) -> Result<Self, LedgerZemuError> {
        let client = ZemuCommandClient::new_plain(host, port, ClientConf::new())
            .map_err(|_| LedgerZemuError::ConnectError)?;
        Ok(Self { client })
    }
}

#[async_trait]
impl Exchange for TransportZemuGrpc {
    type Error = LedgerZemuError;
    type AnswerType = Vec<u8>;

    async fn exchange<I>(
        &self,
        command: &APDUCommand<I>,
    ) -> Result<APDUAnswer<Self::AnswerType>, Self::Error>
    where
        I: Deref<Target = [u8]> + Send + Sync,
    {
        let request = {
            let mut r = ExchangeRequest::new();
            r.set_command(command.serialize());
            r
        };

        let response: ExchangeReply = self
            .client
            .exchange(grpc::RequestOptions::new(), request)
            .drop_metadata()
            .await
            .map_err(|e| {
                log::error!("grpc response error: {:?}", e);
                LedgerZemuError::ResponseError
            })?;

        APDUAnswer::from_answer(response.reply).map_err(|_| LedgerZemuError::ResponseError)
    }
}

pub struct TransportZemuHttp {
    url: String,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct ZemuRequest {
    apdu_hex: String,
}

#[derive(Deserialize, Debug, Clone)]
struct ZemuResponse {
    data: String,
    error: Option<String>,
}

impl TransportZemuHttp {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            url: format!("http://{}:{}", host, port),
        }
    }
}

#[async_trait]
impl Exchange for TransportZemuHttp {
    type Error = LedgerZemuError;
    type AnswerType = Vec<u8>;

    async fn exchange<I>(
        &self,
        command: &APDUCommand<I>,
    ) -> Result<APDUAnswer<Self::AnswerType>, Self::Error>
    where
        I: Deref<Target = [u8]> + Send + Sync,
    {
        let raw_command = hex::encode(command.serialize());
        let request = ZemuRequest {
            apdu_hex: raw_command,
        };

        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let resp: Response = HttpClient::new()
            .post(&self.url)
            .headers(headers)
            .timeout(Duration::from_secs(5))
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                log::error!("create http client error: {:?}", e);
                LedgerZemuError::InnerError
            })?;
        log::debug!("http response: {:?}", resp);

        if resp.status().is_success() {
            let result: ZemuResponse = resp.json().await.map_err(|e| {
                log::error!("error response: {:?}", e);
                LedgerZemuError::ResponseError
            })?;
            if result.error.is_none() {
                APDUAnswer::from_answer(hex::decode(result.data).expect("decode error"))
                    .map_err(|_| LedgerZemuError::ResponseError)
            } else {
                Err(LedgerZemuError::ResponseError)
            }
        } else {
            log::error!("error response: {:?}", resp.status());
            Err(LedgerZemuError::ResponseError)
        }
    }
}
