# Examples

Examples of ledger-rs integrations.

## In NodeJS

NodeJS can integrate WASM modules. Thanks to `wasm-bindgen`, Rust code can be compiled in WASM. This let us use `ledger-rs` inside NodeJS but also the browser (next section).

### Build

You will need `wasm-pack` installed. Go to the `wasm` folder.

```
$ wasm-pack build -t nodejs --no-typescript --out-dir ../node/ledger-node
```

## In Browser

### Build

```
$ wasm-pack build -t browser --no-typescript --out-dir ../browser/ledger-browser
```
