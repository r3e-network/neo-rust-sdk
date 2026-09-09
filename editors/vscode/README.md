# Neo Rust SDK VSCode Extension v3.2.0

Production-grade VSCode extension providing **Rust code snippets**, **syntax highlighting**, and **language support** for developing Neo N3 dApps with the [Neo Rust SDK](https://github.com/R3E-Network/NeoRust).

## Installation

### From VSCode Marketplace (placeholder)
```
Search "Neo Rust SDK" → Install
```

### Manual Install from this directory
```bash
code --install-extension neo-rust-sdk.vsix
```

Build the `.vsix` archive:
```bash
npm install -g vsce
vsce package
```

## Snippets Reference Table

All snippets are available in Rust files (`*.rs`). Type the **prefix** and press `Tab`.

| Prefix              | Description                                          | API Usage                                                                                              |
|---------------------|------------------------------------------------------|--------------------------------------------------------------------------------------------------------|
| `neo-connect`       | Connect to testnet/mainnet                           | `Neo::testnet().await?;`                                                                             |
| `neo-connect-url`   | Connect to custom RPC endpoint                       | `Neo::connect(url).await?;`                                                                          |
| `neo-wallet-new`    | Create new in-memory wallet                          | `Wallet::new(); wallet.create_account()?;`                                                           |
| `neo-wallet-load`   | Load encrypted NEP-6 wallet file                     | `Wallet::open_wallet(path, password)?;`                                                              |
| `neo-balance`       | Query NEO/GAS & NEP-17 balances                      | `neo.get_balance(address).await?;`                                                                   |
| `neo-send`          | Send a NEP-17 transfer                               | `neo.transfer(wallet, to, amount, Token::GAS).await?;`                                               |
| `neo-deploy`        | Deploy smart contract                                | `neo.deploy_contract(wallet, nef_bytes, manifest_json).await?;`                                      |
| `neo-invoke`        | Read + invoke contract method                        | `invoke_read(&contract, "balanceOf", params)?` / `invoke_write(&wallet, &contract, "transfer")`     |
| `neo-nft`           | NEP-11 NFT operations                                | `NftContract::owner_of()` / `.transfer()`                                                            |
| `neo-events-contract` | NEP-27 contract event query                        | `ContractEventQuery::new(hash).block_range(..).execute(&client)`                                     |
| `neo-events-account` | NEP-91 account event query                          | `AccountEventQuery::new(account).contract(hash).execute(&client)`                                    |
| `neo-session`       | Session key signer                                   | `SessionSigner::new(pubkey, parent_key, config);`                                                    |
| `neo-gasless`       | Sponsored (relayer) transaction                      | `RelayerClient::build_sponsored_call(...)`                                                           |
| `neo-fee`           | Dynamic fee policy with priority                     | `FeePolicy::dynamic(FeePriority::High)`                                                              |

### Example: Using `neo-balance` snippet

Type `neo-balance` → Tab:

```rust
use neo3::prelude::*;

// High-level balance: NEO (indivisible), GAS (8 decimals), plus NEP-17 tokens.
let balance = neo.get_balance("NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc").await?;
println!("NEO: {}  GAS: {}", balance.neo, balance.gas);
for token in &balance.tokens {
	println!("{}: {}", token.symbol, token.amount);
}
```

## Language Configuration

For `.neo` and `.neomanifest` files, automatic formatting is enabled:

- Auto-closing pairs: `{}`, `[]`, `()`, `""`
- Comments: `//` single-line, `/* */` block
- Indentation rules for structs, impls, enums, traits

## Syntax Highlighting Coverage

The TextMate grammar (scopeName: `source.neo`) highlights:

- Control keywords: `if`, `else`, `while`, `match`, `return`, `break`
- Storage types: `struct`, `enum`, `trait`, `impl`, `mod`
- CallFlags constants: `CallFlags::None`, `ReadOnly`, `ReadWrite`, `Storages`, `All`
- Token constants: `Token::NEO`, `Token::GAS`, `Token::Custom`
- Fee priorities: `FeePriority::Low`, `Medium`, `High`
- Contract types: `NftContract`, `Balance`, `Token`, `Wallet`, `SessionSigner`, `SessionKeyConfig`
- Event queries: `ContractEventQuery`, `AccountEventQuery`, `RelayerClient`, `GaslessConfig`
- Variables: addresses, owners, methods like `balanceOf`, `owner_of`, `transfer`
- Numbers: GAS base units (9 digits), decimal gas amounts, script hashes

## Contributing

Add more snippets based on actual SDK APIs by editing `snippets/rust.json`. Use tabstops `$1`, `$2`, `${3:default}` for interactivity.

## License

MIT — same as the core Neo Rust SDK crate.

---

Built as part of the v3.2.0 "IDE Extensions" deliverable.
