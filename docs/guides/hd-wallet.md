# HD Wallet Guide (v0.5.1)

NeoRust ships a BIP-39/44 compatible HD wallet in `neo3::sdk::hd_wallet`.

## Generate and Derive

```rust,no_run
use neo3::sdk::hd_wallet::HDWallet;
use bip39::Language;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut wallet = HDWallet::generate(12, None)?;
    println!("Mnemonic: {}", wallet.mnemonic_phrase());

    let account = wallet.derive_account("m/44'/888'/0'/0/0")?;
    println!("Address: {}", account.get_address());

    // Restore from mnemonic
    let _restored = HDWallet::from_phrase(wallet.mnemonic_phrase(), None, Language::English)?;
    Ok(())
}
```

## Best Practices
- Store mnemonics offline and encrypt at rest.
- Derive accounts deterministically and track paths alongside addresses.
- Use different accounts for production, testnet, and development.
