//! CRL (Certificate Revocation List) Checker Module
//! 
//! Placeholder for fetching and caching CRLs from Intel distribution points.

use std::collections::HashMap;

/// Simple in-memory CRL cache with TTL
pub struct CrLChecker {
    /// Map of serial number → revocation timestamp
    revoked_serials: HashMap<u64, u64>,
}

impl Default for CrLChecker {
    fn default() -> Self {
        Self {
            revoked_serials: HashMap::new(),
        }
    }
}

impl CrLChecker {
    /// Create a new CRL checker (empty cache)
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Check if a certificate serial is revoked
    pub fn is_revoked(&self, serial: u64) -> bool {
        self.revoked_serials.contains_key(&serial)
    }
    
    /// Add a serial to the revocation cache (for testing/demo)
    pub fn add_revoked(&mut self, serial: u64, timestamp: u64) {
        self.revoked_serials.insert(serial, timestamp);
    }
}
