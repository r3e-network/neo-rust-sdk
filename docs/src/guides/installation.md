# Installation Guide

## Prerequisites

- Rust and Cargo (stable or nightly)
- Optional: Ledger hardware device (for ledger features)
- Optional: AWS account (for AWS KMS features)

## Installation

Add NeoRust to your `Cargo.toml`:

```toml
[dependencies]
neo3 = "0.4.2"
```

Note: The crate is published as `neo3` but is imported as `neo` in code:

```rust,no_run
use neo3::prelude::*;
```

## Features

NeoRust provides several features to customize functionality:

- `futures`: Enables async/futures support (recommended)
- `ledger`: Enables hardware wallet support via Ledger devices
- `aws`: Enables AWS KMS integration

Example of enabling specific features:

```toml
[dependencies]
neo3 = { version = "0.4.2", features = ["futures", "ledger"] }
```

You can disable default features with:

```toml
[dependencies]
neo3 = { version = "0.4.2", default-features = false, features = ["futures"] }
```

## Verifying Installation

To verify that the SDK is installed correctly, create a simple test program:

```rust,no_run
use neo3::prelude::*;

fn main() {
    println!("NeoRust SDK installed successfully!");
}
```

Compile and run the program:

```bash
cargo run
```

If the program compiles and runs without errors, the SDK is installed correctly.

<!-- toc -->
