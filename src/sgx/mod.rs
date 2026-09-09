//! SGX Quote Verifier Module
//! 
//! This module provides Intel SGX DCAP (Data Center Attestation Primitives) quote verification
//! for production-ready secure enclave deployments in Neo N3 blockchain validators.
//! 
//! # Features
//! 
//! - ECDSA-based quote verification (Intel DCAP compliant)
//! - Certificate chain validation against Intel Root CA
//! - CRL (Certificate Revocation List) checking
//! - Anti-replay protection via nonce tracking
//! - Timestamp validation to prevent replay attacks
//! 
//! # Architecture
//! 
//! ```text
//! ┌─────────────┐     DCAP Quote      ┌──────────────────┐
//! │ Validator   │ ─────────────────▶ │  QuoteVerifier   │
//! │ Enclave     │                     ├─► Parse Quote    │
//! └─────────────┘                     ├─► Verify Chain   │
//!                                    │                  │
//!                                     ─────┬───────┬─────
//!                                           │       │
//!                                ┌──────────┘       └──────────┐
//!                                ▼                            ▼
//!                        ┌──────────────┐            ┌──────────────┐
//!                        │ CRL Fetcher  │            │ Nonce DB     │
//!                        │ (HTTP/Redis) │            │ (Redis)      │
//!                        └──────────────┘            └──────────────┘
//! ```
//! 
//! # Usage
//! 
//! ```ignore
//! use neo_sgx::quote_verifier::QuoteVerifier;
//! use neo_sgx::sgx_types::SgxQuote;
//! 
//! // Initialize verifier with configuration
//! let mut verifier = QuoteVerifier::new();
//! verifier.set_mrenclave_whitelist(vec![expected_enclave_hash]);
//! 
//! // Receive quote from validator
//! let quote_bytes = get_quote_from_network()?;
//! let quote = SgxQuote::parse(&quote_bytes)?;
//! 
//! // Verify quote authenticity
//! let verification_result = verifier.verify_quote(&quote).await?;
//! 
//! match verification_result {
//!     Ok(()) => println!("Quote is valid and trusted"),
//!     Err(e) => eprintln!("Verification failed: {}", e),
//! }
//! ```
//! 
//! # Security Considerations
//! 
//! - All cryptographic operations use vetted libraries (ring, openssl)
//! - Private keys never leave enclave boundary
//! - Memory sanitization via zeroize-on-drop pattern
//! - Constant-time comparisons to prevent timing attacks
//! 
//! References
//!
//! - Intel DCAP: https://github.com/intel/SGXDataCenterAttestationPrimitives
//! - Quote Verifier: https://github.com/intel/quote-verifier
//! - Intel ECDSA Based Attestation Spec

// External crates used in this module

pub mod sgx_types;
pub mod ecdsa_verification;
#[cfg(feature = "sgx")]
pub mod certificate_chain;
pub mod crl_checker;
pub mod nonce_tracker;

use thiserror::Error;

/// Core error types for SGX quote verification failures
#[derive(Error, Debug)]
pub enum QuoteVerificationError {
    #[error("Failed to parse SGX quote: {0}")]
    ParseError(String),
    
    #[error("Certificate chain validation failed: {0}")]
    CertificateChainError(String),
    
    #[error("ECDSA signature verification failed: {0}")]
    SignatureVerificationError(String),
    
    #[error("CRL check failed: {0}")]
    CrlCheckError(String),
    
    #[error("Nonce replay attack detected")]
    ReplayAttackDetected,
    
    #[error("MRENCLAVE not in whitelist: {0}")]
    MrenclaveNotWhitelisted(String),
    
    #[error("Timestamp validation failed: {0}")]
    TimestampValidationError(String),
    
    #[error("Quote version unsupported (required: 3+): {0}")]
    UnsupportedQuoteVersion(u16),
    
    #[error("Network error fetching CRL: {0}")]
    NetworkError(String),
    
    #[error("Memory safety violation during verification")]
    MemorySafetyViolation,
}

// Implement From<String> for each variant with string parameter
impl From<String> for QuoteVerificationError {
    fn from(s: String) -> Self {
        // Default to ParseError, callers should use specific constructors
        QuoteVerificationError::ParseError(s)
    }
}

// Additional From implementations for better error handling
impl From<&str> for QuoteVerificationError {
    fn from(s: &str) -> Self {
        QuoteVerificationError::ParseError(s.to_string())
    }
}

/// Result type alias for SGX verification operations
pub type VerificationResult<T> = Result<T, QuoteVerificationError>;

/// Configuration for the quote verifier
#[derive(Debug, Clone)]
pub struct QuoteVerifierConfig {
    /// Expected MRENCLAVE hashes (enclave identity)
    pub mrenclave_whitelist: Vec<[u8; 32]>,
    
    /// Acceptable quote versions (minimum: 3 for ECDSA)
    pub min_quote_version: u16,
    
    /// Maximum acceptable timestamp age (seconds since signing)
    pub max_timestamp_age_seconds: u64,
    
    /// Enable strict CRL checking
    pub strict_crl_checking: bool,
}

impl Default for QuoteVerifierConfig {
    fn default() -> Self {
        Self {
            mrenclave_whitelist: Vec::new(),
            min_quote_version: 3,
            max_timestamp_age_seconds: 2_592_000, // 30 days
            strict_crl_checking: true,
        }
    }
}
