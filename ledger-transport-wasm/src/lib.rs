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
//! APDU Transport wrapper library for JS transports

#![deny(trivial_numeric_casts)]
#![deny(unused_import_braces, unused_qualifications)]
#![deny(missing_docs)]
//false positive on wasm_bindgen below
#![allow(clippy::unused_unit)]

mod errors;
pub use errors::TransportError;

use std::{ops::Deref, pin::Pin};

use futures::{channel::oneshot::Sender, Future};
use ledger_transport::{APDUAnswer, APDUCommand, Exchange};

use wasm_bindgen::{prelude::*, JsCast};

use js_sys::Uint8Array;

#[wasm_bindgen]
extern "C" {
    pub type JsTransport;

    #[wasm_bindgen(method, catch)]
    async fn send(
        this: &JsTransport,
        cla: u8,
        ins: u8,
        p1: u8,
        p2: u8,
        data: Uint8Array,
        status_list: js_sys::Array,
    ) -> Result<JsValue, JsValue>;
}

impl Exchange for JsTransport {
    type Error = TransportError;

    type AnswerType = Vec<u8>;

    //manual implementation of `async_trait`
    // this is necessary due to the limitation of Promise and JsValue, which we need to bypass
    // by actually creating a sub-future and spawning it.
    // async_trait doesn't support this well as it tries to
    // wrap the entire method body in an async function, including JS stuff
    // which is !Send !Sync thus making the resulting future !Send !Sync
    fn exchange<'life0, 'life1, 'async_trait, I>(
        &'life0 self,
        command: &'life1 APDUCommand<I>,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<APDUAnswer<Self::AnswerType>, Self::Error>>
                + Send
                + 'async_trait,
        >,
    >
    where
        I: Deref<Target = [u8]> + Send + Sync,
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        //prepare a local copy of Self
        let this = {
            let value: &JsValue = self.as_ref();
            value.clone().unchecked_into::<Self>() //clone self...
        };
        let data = Uint8Array::from(command.data.deref());

        //this seems to work better than just an `async move` for some reason
        async fn _exchange(
            this: JsTransport,
            (cla, ins, p1, p2): (u8, u8, u8, u8),
            data: Uint8Array,
            tx: Sender<Result<Vec<u8>, TransportError>>,
        ) {
            use js_sys::Reflect;

            let result = match this
                .send(cla, ins, p1, p2, data, JsValue::UNDEFINED.unchecked_into())
                .await
            {
                Ok(val) => Ok(Uint8Array::from(val).to_vec()),
                Err(err) => {
                    let message_prop = JsValue::from_str("message");
                    let name_prop = JsValue::from_str("name");

                    match (
                        Reflect::get(&err, &message_prop),
                        Reflect::get(&err, &name_prop),
                    ) {
                        //if message is null or undefined, unknown error
                        (Ok(message), _) if message.is_null() || message.is_undefined() => {
                            Err(TransportError::UnknownError)
                        }
                        //if name is null or undefined, unknown error
                        (_, Ok(name)) if name.is_null() || name.is_undefined() => {
                            Err(TransportError::UnknownError)
                        }
                        //if both are strings, return the error
                        (Ok(message), Ok(name)) if message.is_string() && name.is_string() => {
                            Err(TransportError::JavascriptError(
                                name.as_string().unwrap(),
                                message.as_string().unwrap(),
                            ))
                        }
                        //if either message or name is anything else, like a number, unknown error
                        _ => Err(TransportError::UnknownError),
                    }
                }
            };

            let _ = tx.send(result);
        }

        //create channel and spawn js future locally (Promise is !Send !Sync)
        let (tx, rx) = futures::channel::oneshot::channel();
        wasm_bindgen_futures::spawn_local(_exchange(
            this,
            (command.cla, command.ins, command.p1, command.p2),
            data,
            tx,
        ));

        //retrieve the data from the _exchange wrapper
        let task = async move {
            match rx.await {
                Err(_) => Err(TransportError::UnknownError),
                Ok(reply) => match reply {
                    Err(e) => Err(e),
                    Ok(answer) => APDUAnswer::from_answer(answer)
                        .map_err(|_| TransportError::ResponseTooShort),
                },
            }
        };

        Box::pin(task) as _
    }
}
