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

use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE};
use reqwest::{Client as HttpClient, Response};
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::time::Duration;

use ledger_transport::{async_trait, APDUAnswer, APDUCommand, Exchange};

mod errors;
pub use errors::LedgerSpeculosError;

pub struct TransportSpeculosHttp {
    url: String,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct SpeculosRequest {
    data: String,
}

#[derive(Deserialize, Debug, Clone)]
struct SpeculosResponse {
    data: String,
    error: Option<String>,
}

impl TransportSpeculosHttp {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            url: format!("http://{}:{}/apdu", host, port),
        }
    }
}

#[async_trait]
impl Exchange for TransportSpeculosHttp {
    type Error = LedgerSpeculosError;
    type AnswerType = Vec<u8>;

    async fn exchange<I>(
        &self,
        command: &APDUCommand<I>,
    ) -> Result<APDUAnswer<Self::AnswerType>, Self::Error>
    where
        I: Deref<Target = [u8]> + Send + Sync,
    {
        let raw_command = hex::encode(command.serialize());
        let request = SpeculosRequest { data: raw_command };

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
                LedgerSpeculosError::InnerError
            })?;
        log::debug!("http response: {:?}", resp);

        if resp.status().is_success() {
            let result: SpeculosResponse = resp.json().await.map_err(|e| {
                log::error!("error response: {:?}", e);
                LedgerSpeculosError::ResponseError
            })?;
            if result.error.is_none() {
                APDUAnswer::from_answer(hex::decode(result.data).expect("decode error"))
                    .map_err(|_| LedgerSpeculosError::ResponseError)
            } else {
                Err(LedgerSpeculosError::ResponseError)
            }
        } else {
            log::error!("error response: {:?}", resp.status());
            Err(LedgerSpeculosError::ResponseError)
        }
    }
}
