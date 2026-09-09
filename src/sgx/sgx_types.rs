//! SGX Types Module
//!
//! This module defines the data structures required to parse and represent SGX quotes,
//! following Intel's SGX Quote Version 3+ ECDSA-based format.
//!
//! # References
//! - Intel SGX Provisioning Certification Spec v1.6+
//! - DCAP Quote Format Specification

use serde::{Deserialize, Serialize};

/// SGX Quote Version 3+ structure (ECDSA signature)
#[derive(Debug, Clone)]
pub struct SgxQuote {
    /// Quote header (version info)
    pub header: QuoteHeader,
    
    /// Report data section containing MRENCLAVE and other fields
    pub body: QuoteBody,
    
    /// ECDSA signature over the quote content
    pub signature: QuoteSignature,
}

impl SgxQuote {
    /// Parse an SGX quote from raw bytes
    /// 
    /// Expected input: Binary DCAP quote format (~1800 bytes for version 3+)
    pub fn parse(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 1720 {
            return Err("Quote too short");
        }
        
        // Extract header (bytes 0-47)
        let header = Self::parse_header(&data[0..47])?;
        
        // Validate version is 3+ (ECDSA-based)
        if header.version < 3 {
            return Err("Only ECDSA-based quotes (v3+) supported");
        }
        
        // Extract report body (bytes 48-1095)
        let body = Self::parse_body(&data[48..1095])?;
        
        // Extract signature (remaining bytes ~1096-end)
        let signature_data_start = 1095;
        let signature = Self::parse_signature(&data[signature_data_start..])?;
        
        Ok(Self { header, body, signature })
    }
    
    /// Extract MRENCLAVE from quote body
    /// MRENCLAVE is at offset 0 within report_data field
    pub fn mrenclave(&self) -> [u8; 32] {
        self.body.report_data[0..32].try_into().unwrap_or_default()
    }
    
    /// Extract MRSIGNER from quote body
    pub fn mr_signer(&self) -> [u8; 32] {
        self.body.report_data[128..160].try_into().unwrap_or_default()
    }
    
    /// Get the nonce/counter value for anti-replay protection
    pub fn get_nonce(&self) -> [u8; 16] {
        self.header.dynamic_data.nonce
    }
    
    /// Get timestamp in seconds since Unix epoch
    pub fn get_timestamp_seconds(&self) -> u64 {
        // Proper conversion - array [u8; 8] has FromBytes trait in Rust std
        u64::from_le_bytes(*&self.header.dynamic_data.timestamp)
    }
}

impl SgxQuote {
    fn parse_header(data: &[u8]) -> Result<QuoteHeader, &'static str> {
        if data.len() < 47 {
            return Err("Header too short");
        }
        
        Ok(QuoteHeader {
            version: u16::from_le_bytes([data[0], data[1]]),
            qe_svn: u16::from_le_bytes([data[2], data[3]]),
            pce_svn: u16::from_le_bytes([data[4], data[5]]),
            xfrm: u64::from_le_bytes([
                data[6], data[7], data[8], data[9], 
                data[10], data[11], data[12], data[13]
            ]),
            dynamic_data: DynamicDataFields {
                nonce: [data[32]; 16], // Simplified extraction
                timestamp: [0u8; 8],   // Placeholder - proper parsing needed
            },
        })
    }
    
    fn parse_body(data: &[u8]) -> Result<QuoteBody, &'static str> {
        if data.len() < 1047 {
            return Err("Report body too short");
        }
        
        // Copy report_data (348 bytes starting at offset 0)
        let mut report_data = [0u8; 348];
        report_data.copy_from_slice(&data[0..32]); // MRENCLAVE at offset 0
        
        Ok(QuoteBody {
            report_data,
            isv_product_id: u16::from_le_bytes([data[348], data[349]]),
            isvsvn: u16::from_le_bytes([data[350], data[351]]),
        })
    }
    
    fn parse_signature(data: &[u8]) -> Result<QuoteSignature, &'static str> {
        // Signature length varies based on certificate chain size
        // ECDSA P-256 signature is typically 64 bytes + variable cert chain
        if data.len() < 64 {
            return Err("Signature too short");
        }
        
        Ok(QuoteSignature {
            epid_or_ecdsa_sig: data[0..64].try_into().unwrap(),
            certificate_chain: data[64..].to_vec(),
        })
    }
}

/// Quote header structure
#[derive(Debug, Clone)]
pub struct QuoteHeader {
    /// Quote format version (3+ for ECDSA)
    pub version: u16,
    
    /// QE Software Synchronization Version Number
    pub qe_svn: u16,
    
    /// PCE Software Synchronization Version Number
    pub pce_svn: u16,
    
    /// Flags indicating supported extensions
    pub xfrm: u64,
    
    /// Dynamic data fields
    pub dynamic_data: DynamicDataFields,
}

/// Quote body containing the attestation report data
#[derive(Debug, Clone)]
pub struct QuoteBody {
    /// 348-byte report including MRENCLAVE, MRSIGNER, etc.
    pub report_data: [u8; 348],
    
    /// ISV Product ID (application-specific)
    pub isv_product_id: u16,
    
    /// ISV SVN (security version number)
    pub isvsvn: u16,
}

/// ECDSA quote signature with certificate chain
#[derive(Debug, Clone)]
pub struct QuoteSignature {
    /// 64-byte ECDSA P-256 signature
    pub epid_or_ecdsa_sig: [u8; 64],
    
    /// X.509 certificate chain (Intel Root CA → Intermediate → EPID/IAM Signing Cert)
    pub certificate_chain: Vec<u8>,
}

/// Fields that vary dynamically between quotes
#[derive(Debug, Clone)]
pub struct DynamicDataFields {
    /// 16-byte random nonce for anti-replay
    pub nonce: [u8; 16],
    
    /// 8-byte timestamp (Unix epoch seconds)
    pub timestamp: [u8; 8],
}

/// Serialized representation for logging/monitoring
#[derive(Debug, Serialize, Deserialize)]
pub struct QuoteSummary {
    /// Version number
    pub version: u16,
    
    /// MRENCLAVE (hex string)
    pub mrenclave_hex: String,
    
    /// MRSIGNER (hex string)
    pub mr_signer_hex: String,
    
    /// Quote age in seconds
    pub age_seconds: Option<u64>,
    
    /// Whether nonce has been seen before
    pub nonce_seen_before: bool,
}

impl From<&SgxQuote> for QuoteSummary {
    fn from(quote: &SgxQuote) -> Self {
        Self {
            version: quote.header.version,
            mrenclave_hex: hex::encode(&quote.mrenclave()),
            mr_signer_hex: hex::encode(&quote.mr_signer()),
            age_seconds: None, // Would be calculated from timestamp
            nonce_seen_before: false,
        }
    }
}
