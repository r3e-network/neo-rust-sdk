# Migration Guide: v0.4.x → v0.5.1

NeoRust v0.5.x introduces a high-level SDK, WebSockets, HD wallets, and transaction simulation. Core APIs remain but some helpers were renamed or removed.

## Key Changes
- New entrypoint `neo3::sdk::Neo` for common operations (balances, transfers, simulation).
- Signer helpers live under `neo_builder::transaction::signers` (e.g., `AccountSigner::called_by_entry`).
- `ScriptBuilder` now prefers `push_data`, `push_bool`, `push_integer(BigInt)` and `op_code` over older helpers.
- Unified error type `NeoError` with recovery suggestions; pattern match to access recovery metadata.

## Quick Migration

```rust
// v0.4.x
let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
let client = RpcClient::new(provider);
let balance = client.get_balance(&address).await?;

// v0.5.x (high-level)
let neo = Neo::testnet().await?;
let balance = neo.get_balance(&address).await?;
```

## Checklist
- Update `Cargo.toml` to `neo3 = "0.5.0"`.
- Replace deprecated `push_string`/`emit` helpers with `push_data`/`op_code`.
- Adjust examples and docs to use `Neo` and `TransactionSimulator` where appropriate.
- Audit any direct error field access; use pattern matching on `NeoError`.
