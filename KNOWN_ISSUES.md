# Known Issues

## Documentation Tests

Currently, many documentation tests are failing due to import path issues. The doc tests were written before the module structure was finalized and use incorrect import paths.

### Issue Details

The main issues are:
1. Many doc tests use `use neo3::prelude::*` but then try to use types like `HttpProvider` and `RpcClient` directly, when they should be accessed through module aliases like `providers::HttpProvider`.
2. Some doc tests have incorrect crate names (e.g., `NeoRust` instead of `neo3`).
3. Missing imports for types like `OpCode`, `ContractParameter`, etc.

### Temporary Solution

Doc tests have been marked with `no_run` or `ignore` where appropriate to prevent CI failures while maintaining the documentation value.

### Long-term Solution

A comprehensive review and update of all documentation examples is needed to:
1. Use the correct import paths
2. Ensure all required types are imported
3. Test that examples actually compile and run

### Affected Files

- src/lib.rs
- src/neo_builder/mod.rs
- src/neo_builder/script/script_builder.rs
- src/neo_builder/transaction/*.rs
- src/neo_clients/*.rs
- src/neo_crypto/*.rs
- src/neo_protocol/*.rs
- src/neo_types/*.rs
- src/neo_contract/*.rs

### Tracking Issue

TODO: Create GitHub issue to track the documentation test fixes.