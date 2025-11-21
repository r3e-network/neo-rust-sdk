# NeoFS Examples

These examples show how to use the NeoFS functionality in NeoRust.

## Available Examples

1. **Basic Usage** (`basic_usage.rs`): Builds real NeoFS container/object payloads, probes the TestNet gateway, and optionally lists containers when `NEOFS_WALLET` is set.
2. **Multipart Upload** (`multipart_upload.rs`): Plans a multipart upload locally (splitting a payload into parts), validates assembly, and optionally exercises the initiate call.

## Running the Examples

> **Note**: NeoFS REST/gRPC endpoints may require valid auth/session tokens. The examples
> run in a safe "probe" mode by default and will not panic if the remote returns an error.

To run the examples:

```bash
cargo run --example neo_fs_basic_usage

cargo run --example neo_fs_multipart_upload

# Optional: supply your wallet address to attempt authenticated calls
NEOFS_WALLET=NdemoAddressHere cargo run --example neo_fs_basic_usage
```

## Example Output

With no `NEOFS_WALLET` set, you’ll see a gateway probe and payload previews:

```
🌐 Endpoint: https://rest.testnet.fs.neo.org
🔐 Auth: not provided (read-only probe)
🧱 Container request payload:
{ ... }
🔎 Probing NeoFS REST gateway...
   ✅ Gateway responded with HTTP 200
ℹ️ Set NEOFS_WALLET to attempt authenticated container listing.
```

## Requirements

- NeoRust SDK
- Network connectivity to NeoFS TestNet gateway
- Optional `NEOFS_WALLET` for authenticated operations
