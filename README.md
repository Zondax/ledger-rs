# ledger-rs

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![CircleCI](https://circleci.com/gh/Zondax/ledger-rs.svg?style=shield)](https://circleci.com/gh/Zondax/ledger-rs)

Communication library between Rust and Ledger Nano S/X devices

# How to use

## Developing an App interface

To develop an app interface it's recommended to depend on `ledger-transport` and make the API generic over the an `Exchange` (trait).
An example can be found in [`ledger-zondax-hid` tests](./ledger-zondax-hid/src/lib.rs#L380) (provided by `ledger-zondax-generic`) where `get_device_info` is independent of the transport used.

## Using an App interface

To use an app interface, so when communicating with a ledger device (or emulator) the transports available are:
    * `ledger-zondax-hid`
    * `ledger-zondax-zemu`
    * `ledger-zondax-wasm`
    
### WASM
Each transport has its usecase, but most importantly the wasm transport wraps a JS transport (like [@ledgerhq/hw-transport-node-hid](https://www.npmjs.com/package/@ledgerhq/hw-transport-node-hid))
so it can be used from within rust.

An example is usage with wasm can be found in [the examples](./examples/wasm/src/lib.rs), where a transport from JS is used and a function (`device_info`) is exposed to be called [from js](./tests/test.js#L49).
