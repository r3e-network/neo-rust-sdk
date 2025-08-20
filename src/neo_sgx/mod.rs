// SGX Support Module for NeoRust
// Enables running Neo blockchain operations in Intel SGX secure enclaves

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
	pub use super::{init_sgx, SgxError};
}
