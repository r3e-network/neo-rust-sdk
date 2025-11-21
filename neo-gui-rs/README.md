# neo-gui-rs

Native Rust desktop shell for the NeoRust SDK, built with `eframe/egui`.

## Features

- Connect/disconnect to Neo N3 RPC endpoints (mainnet/testnet/custom)
- Live status polling (height, peers, version)
- Local account creation and WIF import
- HD wallet generation/import with BIP-44 derivation
- Unclaimed GAS refresh and NEP-17 balance fetch (NEO/GAS)
- WebSocket monitor subscribing to new blocks/transactions/execution results
- Simple transaction simulator (invokescript) for dry runs
- Draft transfer UI with invoke-based estimation (signing/send planned next)
- Activity log for background tasks

## Run

```bash
cargo run -p neo-gui-rs
```

## Notes

- Uses the SDK’s async RPC client directly—no Node or Tauri toolchain required.
- Wallet actions are local only; signing/sending transactions will be added in a follow-up.
