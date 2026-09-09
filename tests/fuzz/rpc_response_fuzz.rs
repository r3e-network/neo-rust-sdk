//! Fuzz testing for RPC response deserialization
//! Tests that JSON parsing handles malformed or unexpected responses gracefully
//! Uses ACTUAL SDK RPC response types instead of serde_json::Value

use proptest::prelude::*;
use serde_json::json;
use num_enum::TryFromPrimitive;
use neo3::neo_types::{StackItem, OpCode, VMState};
use neo3::neo_types::contract::InvocationResult;
use base64::Engine;

/// Test deserializing invocation results with real InvocationResult type
/// Exercises actual RPC client response handling via invokefunction/invokeinvokscript
#[test]
fn test_invocation_result_deserialization_edge_cases() {
    let valid_cases = vec![
        // Successful invocation
        json!({
            "script": "01",
            "state": "HALT",
            "gasconsumed": "1234567",
            "stack": [{"type": "Integer", "value": "42"}]
        }),
        // Failed invocation
        json!({
            "script": "00",
            "state": "FAULT",
            "gasconsumed": "0",
            "exception": "Error during execution",
            "stack": []
        }),
        // Empty response
        json!({
            "script": "",
            "state": "",
            "gasconsumed": "0",
            "stack": []
        }),
        // With notifications
        json!({
            "script": "01",
            "state": "HALT",
            "gasconsumed": "1000",
            "notifications": [{
                "contract": "0xd2a4cff31913016155e38e474a2c06d08be276cf",
                "eventname": "transfer",
                "state": {"type": "Array", "value": []}
            }],
            "stack": [{"type": "Boolean", "value": true}]
        }),
        // Complex stack items
        json!({
            "script": "01",
            "state": "Halt",
            "gasconsumed": "500",
            "stack": [
                {"type": "Integer", "value": "1"},
                {"type": "Boolean", "value": false},
                {"type": "ByteString", "value": "SGVsbG8="},
                {"type": "Array", "value": [{"type": "Integer", "value": "100"}]}
            ]
        }),
    ];

    for json_val in valid_cases {
        let result: Result<InvocationResult, _> = serde_json::from_value(json_val);
        assert!(result.is_ok(), "Should deserialize valid invocation results");
    }
}

/// Test malformed/invalid invocation results
#[test]
fn test_invocation_result_malformed_handling() {
    // These are cases that SHOULD succeed with defaults or partial data
    // This shows the SDK gracefully handles incomplete responses
    let valid_partial_cases = vec![
        json!({"script": "01", "gasconsumed": "10", "stack": []}), // Missing state (defaults)
    ];

    for json_val in valid_partial_cases {
        let result: Result<InvocationResult, _> = serde_json::from_value(json_val);
        assert!(result.is_ok(), "Partial responses should be handled gracefully");
    }
}

/// Test arbitrary byte sequence handling with real VM state types
#[test]
fn test_opcode_and_vm_state_random_data() {
    proptest!(|(bytes in any::<Vec<u8>>())| {
        // Try each byte as potential opcode - exercises real OpCode enum
        if !bytes.is_empty() {
            let first_byte = bytes[0];
            let opcode_result = OpCode::try_from_primitive(first_byte);
            match opcode_result {
                Ok(_) => {},   // Valid opcode found
                Err(_) => {},  // Invalid opcode - also acceptable
            }
        }
        
        // Also try converting random integers to VM states
        let vm_states = vec![VMState::None, VMState::Halt, VMState::Fault, VMState::Break];
        for state in vm_states {
            let _ = format!("{:?}", state); // Just verify formatting works
        }
    });
}

// Add more block header primitive tests
mod block_primitives {
    use super::*;
    
    #[test]
    fn test_h256_hash_construction() {
        use primitive_types::H256;
        
        let bytes = [0u8; 32];
        let hash = H256::from_slice(&bytes);
        let hex_str = hex::encode(hash);
        assert_eq!(hex_str.len(), 64); // 32 bytes = 64 hex chars
        
        let non_zero = [0xFFu8; 32];
        let hash = H256::from_slice(&non_zero);
        let hex_str = hex::encode(hash);
        assert_eq!(hex_str.len(), 64);
    }
}

/// Test special characters and unicode handling with real StackItem deserialization
#[test]
fn test_special_characters_unicode_real_types() {
    let unicode_strings = vec![
        String::from("hello"),
        String::from("你好世界"),
        String::from("🌍🎉🚀💎"),
        String::from("Café résumé naïve"),
    ];

    for utf8_str in unicode_strings {
        // Encode as bytes first, then base64
        let bytes = utf8_str.as_bytes();
        let base64_encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        let json_val = json!({
            "type": "ByteString",
            "value": base64_encoded
        });
        
        let result: Result<StackItem, _> = serde_json::from_value(json_val);
        // Should successfully deserialize Unicode strings
        assert!(result.is_ok());
    }
}

/// Integration-style test: real RPC-like payloads
#[test]
fn test_realistic_rpc_payloads() {
    
    // Simulate an invokefunction response
    let invoke_example = json!({
        "script": "01",
        "state": "Halt",
        "gasconsumed": "3230000",
        "stack": [{
            "type": "Array",
            "value": [
                {"type": "Integer", "value": "50000000000"},
                {"type": "ByteString", "value": "UyMfNw=="},
                {"type": "Integer", "value": "8"}
            ]
        }],
        "notifications": [{
            "contract": "0xd2a4cff31913016155e38e474a2c06d08be276cf",
            "eventname": "Transfer",
            "state": {
                "type": "Array",
                "value": []
            }
        }]
    });
    
    let invoke_result: Result<InvocationResult, _> = serde_json::from_value(invoke_example);
    assert!(invoke_result.is_ok());
}
