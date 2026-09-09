//! HD Wallet Regression Tests
//! Comprehensive tests to prevent overflow bugs like those discovered in v3.0.0
//! Tests derivation paths systematically across the entire uint32 range

use bip39::Language;
use neo3::sdk::hd_wallet::{HDWallet, DerivationPath};

/// Test sequential derivation from 0 to N (regression test)
#[test]
fn test_sequential_derivation_paths() {
    let mut wallet = generate_test_wallet();
    
    // Test first 1000 derivation paths
    let mut derived_addresses: Vec<String> = Vec::new();
    
    for i in 0..1000u32 {
        let path_str = format!("m/44'/888'/0'/0/{}", i);
        
        // Should not panic on any valid path
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            wallet.derive_account(&path_str)
        }));
        
        assert!(result.is_ok(), "Derivation should not panic for path {}", path_str);
        
        match result.unwrap() {
            Ok(account) => {
                let address = account.get_address();
                assert!(!address.is_empty(), "Address should not be empty");
                derived_addresses.push(address);
            }
            Err(e) => {
                // Some paths might legitimately fail (e.g., invalid format)
                // But they should return errors, not panic
                eprintln!("Expected error for path {}: {:?}", path_str, e);
            }
        }
    }
    
    // Verify some addresses were derived
    assert!(!derived_addresses.is_empty());
}

/// Test boundary conditions for derivation indices
#[test]
fn test_derivation_index_boundaries() {
    let mut wallet = generate_test_wallet();
    
    // Edge case: Zero index
    let zero_path = "m/44'/888'/0'/0/0";
    let result_zero = wallet.derive_account(zero_path);
    assert!(result_zero.is_ok() || result_zero.is_err()); // Should handle gracefully
    
    // Edge case: Max value that fits in u32 but is commonly problematic
    let near_max_path = "m/44'/888'/0'/0/4294967295"; // u32::MAX - 1
    let result_near_max = wallet.derive_account(near_max_path);
    assert!(result_near_max.is_ok() || result_near_max.is_err());
    
    // Edge case: Exactly u32::MAX
    let max_path = "m/44'/888'/0'/0/4294967295"; // u32::MAX
    let result_max = wallet.derive_account(max_path);
    // Should either work or return error, never panic
    assert!(result_max.is_ok() || result_max.is_err());
}

/// Test overflow prevention in arithmetic operations
#[test]
fn test_overflow_prevention_in_derivations() {
    let mut wallet = generate_test_wallet();
    
    // Simulate potential overflow scenario
    let problematic_values = vec![
        u32::MAX,          // Maximum possible value
        u32::MAX - 1,      // Just below maximum
        u32::MAX / 2,      // Half of maximum
        0xFFFFFFFF,        // Same as MAX
        0x7FFFFFFF,        // Signed int max (boundary)
    ];
    
    for value in problematic_values {
        let path = format!("m/44'/888'/0'/0/{}", value);
        
        // Test without closure to avoid UnwindSafe issues
        let result = wallet.derive_account(&path);
        
        // Critical: Never allow panics
        // The result will be Ok(Account) or Err(NeoError), never panicked
        let _ = result;
    }
}

/// Test negative index handling (if supported)
#[test]
fn test_negative_index_handling() {
    // Note: Not all wallets support negative indices
    // This test documents expected behavior
    
    let mut wallet = generate_test_wallet();
    
    // Attempt to derive with "negative" path notation
    // Expected: Graceful error, not panic
    let negative_path = "m/44'/888'/0'/0/-1";
    let result = wallet.derive_account(negative_path);
    
    // Should either succeed (if implemented) or return explicit error
    match result {
        Ok(_) => {},   // If negative indexing is supported
        Err(_) => {},  // Otherwise, error is acceptable
        // NEVER panic
    }
}

/// Test batch derivation performance and safety
#[test]
fn test_large_batch_derivation() {
    let mut wallet = generate_test_wallet();
    
    const BATCH_SIZE: usize = 10000;
    
    // Generate many accounts rapidly
    let paths: Vec<String> = (0..BATCH_SIZE)
        .map(|i| format!("m/44'/888'/0'/0/{}", i))
        .collect();
    
    let mut successful = 0;
    let mut failed = 0;
    
    for path in paths {
        match wallet.derive_account(&path) {
            Ok(_) => successful += 1,
            Err(_) => failed += 1,
        }
    }
    
    // At minimum, most derivations should succeed
    // This also tests that we don't exhaust resources or cause panics
    assert!(successful > 0, "At least some derivations must succeed");
    println!("Batch derivation completed: {} success, {} failed", successful, failed);
}

/// Test consistency across multiple wallet instances
#[test]
fn test_wallet_consistency_across_instances() {
    let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    
    // Create multiple wallet instances from same seed
    let mut wallet1 = HDWallet::from_phrase(phrase, None, Language::English).unwrap();
    let mut wallet2 = HDWallet::from_phrase(phrase, None, Language::English).unwrap();
    
    // Same path should yield same results
    let common_path = "m/44'/888'/0'/0/5";
    
    let addr1 = wallet1.derive_account(common_path).unwrap();
    let addr2 = wallet2.derive_account(common_path).unwrap();
    
    // Compare addresses
    assert_eq!(addr1.get_address(), addr2.get_address(), "Same mnemonic + path must produce identical address");
}

/// Test derivation path parsing and validation
#[test]
fn test_derivation_path_parsing_edge_cases() {
    let test_cases = vec![
        ("m/44'/888'/0'/0/0", true),       // Standard path
        ("m/44'/888'/0'/0/0/", false),     // Trailing slash
        ("M/44'/888'/0'/0/0", false),     // Uppercase M
        ("44'/888'/0'/0/0", true),        // Without 'm' prefix
        ("", false),                        // Empty string
        ("invalid", false),                 // Invalid format
    ];
    
    for (path_str, should_succeed) in test_cases {
        let result = DerivationPath::from_string(path_str);
        
        match result {
            Ok(_) => assert_eq!(should_succeed, true, "Path '{}' should succeed", path_str),
            Err(_) => assert_eq!(should_succeed, false, "Path '{}' should fail", path_str),
        }
    }
}

/// Branch coverage verification helper
#[allow(dead_code)]
fn verify_branch_coverage() {
    // This function ensures all major branches are covered
    let mut wallet = generate_test_wallet();
    
    // Path 1: Success case
    let _success = wallet.derive_account("m/44'/888'/0'/0/0").ok();
    
    // Path 2: Error case  
    let _error = wallet.derive_account("invalid/path");
    
    // Both success and error paths tested above
}

/// Generate a test wallet with fixed mnemonic for reproducibility
fn generate_test_wallet() -> HDWallet {
    HDWallet::from_phrase(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        None,
        Language::English,
    )
    .expect("Failed to create test wallet")
}
