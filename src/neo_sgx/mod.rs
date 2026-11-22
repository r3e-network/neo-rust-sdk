//! # Neo SGX Support
//!
//! Support for running Neo blockchain operations in Intel SGX secure enclaves.
//!
//! ## Overview
//!
//! The neo_sgx module provides interfaces for running Neo blockchain operations
//! within Intel SGX (Software Guard Extensions) secure enclaves. This enables:
//!
//! - **Secure Key Management**: Private keys never leave the enclave
//! - **Remote Attestation**: Prove code integrity to remote parties
//! - **Confidential Computing**: Process sensitive data in isolated memory
//! - **Tamper-Resistant Execution**: Protected from privileged software attacks
//!
//! ## Feature Flag
//!
//! This module requires the `sgx` feature flag to be enabled:
//!
//! ```toml
//! [dependencies]
//! neo3 = { version = "0.5", features = ["sgx"] }
//! ```
//!
//! ## Architecture
//!
//! The SGX module is organized into several submodules:
//!
//! - **allocator**: Custom memory allocator for SGX enclaves
//! - **attestation**: Remote attestation and quote verification
//! - **crypto**: Cryptographic operations within the enclave
//! - **enclave**: Enclave lifecycle management
//! - **networking**: Secure network communication from enclaves
//! - **storage**: Sealed storage for persistent data
//!
//! ## Examples
//!
//! ### Initializing SGX Environment
//!
//! ```ignore
//! use neo3::neo_sgx::prelude::*;
//!
//! fn main() -> Result<(), SgxError> {
//!     // Initialize the SGX environment
//!     init_sgx()?;
//!
//!     println!("SGX environment initialized successfully");
//!     Ok(())
//! }
//! ```
//!
//! ### Creating a Secure Enclave for Key Management
//!
//! ```ignore
//! use neo3::neo_sgx::prelude::*;
//!
//! fn main() -> Result<(), SgxError> {
//!     // Configure the enclave
//!     let config = EnclaveConfig::builder()
//!         .debug_mode(false)
//!         .heap_size(1024 * 1024) // 1MB heap
//!         .stack_size(64 * 1024)  // 64KB stack
//!         .build();
//!
//!     // Create the enclave
//!     let enclave = SgxEnclave::create(config)?;
//!
//!     // Use the enclave for secure operations
//!     let key_manager = SgxKeyManager::new(&enclave)?;
//!     let keypair = key_manager.generate_keypair()?;
//!
//!     println!("Generated secure keypair in enclave");
//!     Ok(())
//! }
//! ```
//!
//! ### Remote Attestation
//!
//! ```ignore
//! use neo3::neo_sgx::prelude::*;
//!
//! async fn verify_enclave() -> Result<(), SgxError> {
//!     // Create attestation instance
//!     let attestation = RemoteAttestation::new()?;
//!
//!     // Generate a quote for remote verification
//!     let quote = attestation.generate_quote()?;
//!
//!     // Send quote to verifier and get attestation result
//!     let verifier = QuoteVerifier::new("https://attestation-service.example.com");
//!     let result = verifier.verify(&quote).await?;
//!
//!     if result.is_valid() {
//!         println!("Enclave attestation successful");
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Security Considerations
//!
//! - Always use release builds for production enclaves
//! - Disable debug mode in production
//! - Regularly update SGX SDK and platform software
//! - Implement proper error handling for attestation failures
//! - Use sealed storage for persistent sensitive data
//!
//! ## Platform Requirements
//!
//! - Intel CPU with SGX support (different generations have different capabilities)
//! - SGX-enabled BIOS settings
//! - Intel SGX SDK installed
//! - Linux with SGX driver or Windows with SGX PSW

#![cfg_attr(feature = "sgx", no_std)]
#![cfg_attr(feature = "sgx", feature(rustc_private))]

#[cfg(feature = "sgx")]
extern crate sgx_tstd as std;

#[cfg(feature = "sgx")]
use sgx_tstd::prelude::v1::*;

pub mod allocator;
pub mod attestation;
pub mod crypto;
pub mod enclave;
pub mod networking;
pub mod storage;

#[cfg(feature = "sgx")]
pub use allocator::SgxAllocator;
#[cfg(feature = "sgx")]
pub use crypto::SgxCrypto;
#[cfg(feature = "sgx")]
pub use enclave::SgxEnclave;
#[cfg(feature = "sgx")]
pub use networking::SgxNetworking;

/// Initialize SGX environment
#[cfg(feature = "sgx")]
pub fn init_sgx() -> Result<(), SgxError> {
	allocator::init_allocator()?;
	crypto::init_crypto()?;
	Ok(())
}

/// SGX-specific error types
#[derive(Debug, Clone)]
pub enum SgxError {
	InitializationFailed(String),
	CryptoError(String),
	NetworkError(String),
	AttestationError(String),
	MemoryError(String),
	EnclaveError(String),
}

impl core::fmt::Display for SgxError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			SgxError::InitializationFailed(msg) => write!(f, "SGX initialization failed: {}", msg),
			SgxError::CryptoError(msg) => write!(f, "SGX crypto error: {}", msg),
			SgxError::NetworkError(msg) => write!(f, "SGX network error: {}", msg),
			SgxError::AttestationError(msg) => write!(f, "SGX attestation error: {}", msg),
			SgxError::MemoryError(msg) => write!(f, "SGX memory error: {}", msg),
			SgxError::EnclaveError(msg) => write!(f, "SGX enclave error: {}", msg),
		}
	}
}

#[cfg(not(feature = "sgx"))]
impl std::error::Error for SgxError {}

/// Re-export commonly used SGX types
#[cfg(feature = "sgx")]
pub mod prelude {
	pub use super::allocator::SgxAllocator;
	pub use super::attestation::{QuoteVerifier, RemoteAttestation};
	pub use super::crypto::{SgxCrypto, SgxKeyManager};
	pub use super::enclave::{EnclaveConfig, SgxEnclave};
	pub use super::networking::{SecureChannel, SgxNetworking};
	pub use super::{SgxError, init_sgx};
}
