# ledger-rs

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Github Actions](https://github.com/Zondax/ledger-rs/actions/workflows/main.yaml/badge.svg)](https://github.com/Zondax/ledger-rs)

Communication library between Rust and Ledger Nano S/X devices

# How to use

## Developing an App interface

To develop an app interface it's recommended to depend on `ledger-transport` and make the API generic over the an `Exchange` (trait).
An example can be found in [`ledger-zondax-hid` tests](./ledger-zondax-hid/src/lib.rs#L380) (provided by `ledger-zondax-generic`) where `get_device_info` is independent of the transport used.

## Using an App interface

To use an app interface, so when communicating with a ledger device (or emulator) the transports available are:
    * `ledger-transport-hid`
    * `ledger-transport-zemu`

# How to publish to crates.io

Obviously only members of the Zondax/crates team are allowed to publish.

Afterwards, there's a correct order to publish the crates, based on the crate dependencies:

* ledger-apdu
* ledger-transport
* ledger-zondax-generic

Then, the rest of the crates can be published in any order.

``sh
cargo login
cargo package -p ledger-apdu
cargo publish -p ledger-apdu

cargo package -p ledger-transport
cargo publish -p ledger-transport

cargo package -p ledger-zondax-generic
cargo publish -p ledger-zondax-generic

cargo package -p ledger-transport-hid
cargo publish -p ledger-transport-hid
``