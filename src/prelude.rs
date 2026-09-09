//! # NeoRust prelude
//!
//! A curated re-export of the most commonly used types, traits, and helpers
//! from across the crate. Pull it in once and skip the import boilerplate:
//!
//! ```rust
//! use neo3::prelude::*;
//! ```
//!
//! ## What you get
//!
//! | Category | Highlights |
//! |----------|------------|
//! | **High-level SDK** | [`Neo`], [`NeoBuilder`], [`SdkConfig`], [`Token`], [`Balance`], [`Network`] |
//! | Primitives | [`Address`], [`ScriptHash`], [`Bytes`], `H160`, `H256`, `U256` |
//! | Contracts | [`ContractManifest`], [`ContractParameter`], [`InvocationResult`], [`NefFile`] |
//! | VM | [`OpCode`], [`StackItem`], [`VMState`] |
//! | NNS | [`NNSName`] |
//! | Errors | [`NeoError`] — unified error with `kind()` / `is_retryable()` / recovery hints |
//! | Module aliases | `builder`, `providers`, `codec`, `config`, `crypto`, `protocol`, `wallets`, `x` |
//! | Encoding helpers | [`Base64Encode`], [`StringExt`], `ToBase58`, `ToHexString`, … |
//!
//! ## Quick start via the prelude
//!
//! The high-level entry point is available straight from the prelude, so a
//! working client is a one-liner:
//!
//! ```no_run
//! use neo3::prelude::*;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), NeoError> {
//! let neo = Neo::testnet().await?;
//! let height = neo.get_block_height().await?;
//! println!("tip = {height}");
//! # Ok(())
//! # }
//! ```
//!
//! ## When to import the prelude vs explicit paths
//!
//! - **Use the prelude** when writing application code that touches many parts
//!   of the SDK (transactions, contracts, wallets, RPC).
//! - **Use explicit imports** in library code, or when you only need one or
//!   two types — it keeps namespaces clean and helps incremental compilation.
//!
//! [`NeoError`]: crate::neo_error::unified::NeoError
//! [`Neo`]: crate::sdk::Neo
//! [`NeoBuilder`]: crate::sdk::NeoBuilder
//! [`SdkConfig`]: crate::sdk::SdkConfig
//! [`Token`]: crate::sdk::Token
//! [`Balance`]: crate::sdk::Balance
//! [`Network`]: crate::sdk::Network
// Unified error type (modern). Carries kind()/is_retryable()/recovery hints,
// and has From impls for every domain error (ProviderError, BuilderError,
// ContractError, WalletError, CryptoError, Neo3Error, io, serde, ...), so a
// single `fn() -> Result<T, NeoError>` boundary composes with `?` across the
// whole SDK. This is the same type returned by the high-level `sdk::Neo`.
pub use crate::neo_error::unified::NeoError;

// SDK version
pub use crate::VERSION;

// === Core Types ===
// Basic blockchain types
pub use crate::neo_types::{
	Address, AddressOrScriptHash, Base64Encode, Bytes, NameOrAddress, ScriptHash,
	ScriptHashExtension, StringExt, ToBase58, TryBase64Encode, TryStringExt,
};

// Contract-related types
pub use crate::neo_types::{
	ContractManifest, ContractParameter, ContractParameterType, ContractState, InvocationResult,
	NefFile,
};

// VM and runtime types
pub use crate::neo_types::{OpCode, StackItem, VMState};

// NNS-related types
pub use crate::neo_types::NNSName;

// Common external types
pub use primitive_types::{H160, H256, U256};
pub use serde_json::Value as ParameterValue;
pub use url::Url;

// === Serialization Helpers ===
pub use crate::neo_types::{
	// H160/H256 serialization
	deserialize_h160,
	deserialize_h256,
	// Other serialization helpers
	deserialize_script_hash,
	// U256 serialization
	deserialize_u256,
	deserialize_u64,
	deserialize_vec_h256,
	deserialize_vec_u256,
	deserialize_wildcard,
	serialize_h160,
	serialize_h256,
	serialize_script_hash,
	serialize_u256,
	serialize_u64,

	serialize_vec_h256,

	serialize_vec_u256,
	serialize_wildcard,
};

// === Core Functionality Modules ===
// These are aliased module names for user convenience
pub use crate::{
	neo_builder as builder, neo_clients as providers, neo_codec as codec, neo_config as config,
	neo_crypto as crypto, neo_protocol as protocol, neo_wallets as wallets, neo_x as x,
};

// === Extension modules ===
// These are full modules that provide specialized functionality
pub use crate::neo_fs; // NeoFS distributed storage

// Re-export ValueExtension
pub use crate::neo_types::ValueExtension;

// === Utility Traits ===
// Hex and base64 encoding/decoding utilities
pub use crate::neo_crypto::utils::{FromBase64String, FromHexString, ToHexString};

// === High-level SDK API ===
// The opinionated, batteries-included entry point. `Neo::testnet()`,
// `Neo::from_env()`, `Neo::connect(url)` — see the crate-level docs for
// guidance on when to use this layer versus the lower-level `providers::RpcClient`.
pub use crate::sdk::{Balance, Neo, NeoBuilder, Network, SdkConfig, Token};

// === Smart Contracts & NFT Support ===
// Common contract interfaces including NEP-11 NFT standard support.
pub use crate::neo_contract::{NftContract, NonFungibleTokenTrait};

// NEP-27 contract event query builder
pub use crate::neo_contract::{ContractEventQuery, ContractEventResult};

// NEP-91 account event query builder
pub use crate::neo_contract::{AccountEventQuery, AccountEventResult};

// Session Key Management
// Temporary, limited-permission keys for enterprise/multi-sig delegation
pub use crate::neo_wallets::{SessionKeyConfig, SessionSigner, CallFlagsWrapper};

// Test Node Emulator
// In-process mock RPC server for deterministic testing and simulation
pub use crate::neo_protocol::{TestNode, TestNodeConfig, TestNodeError, TestBlock, TestTransaction};
