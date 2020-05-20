use wasm_bindgen::prelude::*;
use js_sys::Promise;
use serde::{Deserialize, Serialize};
use ledger_transport::{TransportWrapperTrait, APDUTransport};
use ledger_generic::{APDUAnswer, APDUCommand, APDUErrorCodes};

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

// This defines the Node.js Buffer type
#[wasm_bindgen]
extern "C" {
    pub type Buffer;

    #[wasm_bindgen(constructor)]
    fn from(buffer_array: &[u8]) -> Buffer;
}

#[wasm_bindgen(module = "/src/transportWrapper.js")]
extern "C" {
    pub type TransportWrapper;

    #[wasm_bindgen(method)]
    pub fn exchange(this: &TransportWrapper, apdu_command: Buffer) -> Promise;
}

impl TransportWrapperTrait for TransportWrapper {
    fn exchange_apdu(&self, apdu_command: &[u8]) -> js_sys::Promise {
        self.exchange(Buffer::from(apdu_command))
    }
}

#[wasm_bindgen(js_name = deviceInfo)]
pub async fn device_info(transport_wrapper: TransportWrapper) -> Promise {
    let apdu_transport = APDUTransport {
        transport_wrapper: Box::new(transport_wrapper),
    };

    let command = APDUCommand {
        cla: 0xe0,
        ins: 0x01,
        p1: 0x00,
        p2: 0x00,
        data: Vec::new(),
    };

    let response = apdu_transport.exchange(command).await.unwrap();
    if response.retcode != APDUErrorCodes::NoError as u16 {
        // Panic ! Ledger returned an error code
        panic!();
    }

    let target_id_slice = &response.data[0..4];
    let mut idx = 4;
    let se_version_len: usize = response.data[idx] as usize;
    idx += 1;
    let se_version_bytes = &response.data[idx..idx + se_version_len];

    idx += se_version_len;

    let flags_len: usize = response.data[idx] as usize;
    idx += 1;
    let flag = &response.data[idx..idx + flags_len];
    idx += flags_len;

    let mcu_version_len: usize = response.data[idx] as usize;
    idx += 1;
    let mut tmp = &response.data[idx..idx + mcu_version_len];
    if tmp[mcu_version_len - 1] == 0 {
        tmp = &response.data[idx..idx + mcu_version_len - 1];
    }

    let mut target_id = [Default::default(); 4];
    target_id.copy_from_slice(target_id_slice);

    let se_version = std::str::from_utf8(se_version_bytes).map_err(|_e| {
        Promise::reject(&js_sys::Error::new(
            "Error reading SE version (cannot convert bytes to utf8).",
        ))
    }).unwrap();
    let mcu_version = std::str::from_utf8(tmp).map_err(|_e| {
        Promise::reject(&js_sys::Error::new(
            "Error reading MCU version (cannot convert bytes to utf8).",
        ))
    }).unwrap();

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
