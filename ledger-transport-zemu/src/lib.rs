mod errors;
mod zemu;
mod zemu_grpc;

use grpc::prelude::*;
use grpc::ClientConf;
use ledger_apdu::{APDUAnswer, APDUCommand};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE};
use reqwest::{Client as HttpClient, Response};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use zemu::ExchangeRequest;
use zemu_grpc::ZemuCommandClient;

use crate::zemu::ExchangeReply;
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

    pub async fn exchange(&self, command: &APDUCommand) -> Result<APDUAnswer, LedgerZemuError> {
        let mut request = ExchangeRequest::new();
        request.set_command(command.serialize());
        let response: ExchangeReply = self
            .client
            .exchange(grpc::RequestOptions::new(), request)
            .drop_metadata()
            .await
            .map_err(|e| {
                log::error!("grpc response error: {:?}", e);
                LedgerZemuError::ResponseError
            })?;
        Ok(APDUAnswer::from_answer(response.reply))
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

    pub async fn exchange(&self, command: &APDUCommand) -> Result<APDUAnswer, LedgerZemuError> {
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
                Ok(APDUAnswer::from_answer(
                    hex::decode(result.data).expect("decode error"),
                ))
            } else {
                Err(LedgerZemuError::ResponseError)
            }
        } else {
            log::error!("error response: {:?}", resp.status());
            Err(LedgerZemuError::ResponseError)
        }
    }
}
