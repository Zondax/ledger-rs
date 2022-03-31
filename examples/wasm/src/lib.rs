#![allow(clippy::unused_unit)] //for wasm-bindgen

use js_sys::Promise;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use ledger_transport::{APDUCommand, APDUErrorCode, Exchange};
use ledger_transport_wasm::JsTransport;

#[macro_use]
mod log;

/// Ledger Device Info Answer
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DeviceInfo {
    /// Target ID
    #[serde(rename(serialize = "targetId"))]
    pub target_id: [u8; 4],
    /// Secure Element Version
    #[serde(rename(serialize = "seVersion"))]
    pub se_version: String,
    /// Device Flag
    pub flag: Vec<u8>,
    /// MCU Version
    #[serde(rename(serialize = "mcuVersion"))]
    pub mcu_version: String,
}

#[wasm_bindgen(js_name = deviceInfo)]
pub async fn device_info(apdu_transport: JsTransport) -> Promise {
    let command = APDUCommand {
        cla: 0xe0,
        ins: 0x01,
        p1: 0x00,
        p2: 0x00,
        data: Vec::new(),
    };

    let response = match apdu_transport.exchange(&command).await {
        Ok(ok) => ok,
        Err(e) => {
            console_log!("ledger exchange error: {:?}", e);
            panic!("Ledger returned error: {:?}", e)
        }
    };

    match response.error_code() {
        Ok(APDUErrorCode::NoError) => {}
        Ok(err) => panic!("Ledger returned error: {:?}", err),
        Err(err) => panic!("Unknown ledger error: {:x}", err),
    }

    let target_id_slice = &response.data()[0..4];
    let mut idx = 4;
    let se_version_len: usize = response.data()[idx] as usize;
    idx += 1;
    let se_version_bytes = &response.data()[idx..idx + se_version_len];

    idx += se_version_len;

    let flags_len: usize = response.data()[idx] as usize;
    idx += 1;
    let flag = &response.data()[idx..idx + flags_len];
    idx += flags_len;

    let mcu_version_len: usize = response.data()[idx] as usize;
    idx += 1;
    let mut tmp = &response.data()[idx..idx + mcu_version_len];
    if tmp[mcu_version_len - 1] == 0 {
        tmp = &response.data()[idx..idx + mcu_version_len - 1];
    }

    let mut target_id = [Default::default(); 4];
    target_id.copy_from_slice(target_id_slice);

    let se_version = std::str::from_utf8(se_version_bytes)
        .map_err(|_e| {
            Promise::reject(&js_sys::Error::new(
                "Error reading SE version (cannot convert bytes to utf8).",
            ))
        })
        .unwrap();
    let mcu_version = std::str::from_utf8(tmp)
        .map_err(|_e| {
            Promise::reject(&js_sys::Error::new(
                "Error reading MCU version (cannot convert bytes to utf8).",
            ))
        })
        .unwrap();

    let device_info = DeviceInfo {
        target_id,
        se_version: se_version.to_string(),
        flag: flag.to_vec(),
        mcu_version: mcu_version.to_string(),
    };

    let answer = JsValue::from_serde(&device_info)
        .map_err(|_e| {
            Promise::reject(&js_sys::Error::new(
                "Error converting answer message to javascript value.",
            ))
        })
        .unwrap();

    Promise::resolve(&answer)
}
