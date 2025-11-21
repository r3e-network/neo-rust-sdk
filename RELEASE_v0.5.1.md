# Release 0.5.1 - 2025-11-20

## Highlights
- Hardened HD wallet derivation and safer entropy handling.
- Updated public API visibility/derives for attributes, name service, simulator responses, and unspent balances.
- Alignments with upstream base64/RPC APIs and cleaner encoding paths.
- Documentation refresh: HD wallet, transaction simulation, websockets, and v0.5 migration guides.
- Stability fixes: rate limiter concurrency test, ignored live-RPC integrations to reduce CI flaky runs.

## Compatibility
- No breaking changes expected; patch release focused on polish and correctness.
- Integration tests requiring live RPC remain `#[ignore]`.

## Testing
- `cargo test -p neo3 --lib --tests --quiet` (passes; integration tests requiring live RPC ignored)

## Known Issues
- Clippy with `-D warnings` still reports outstanding lints (mostly style/size/result_large_err). Address before enforcing clippy in CI.
