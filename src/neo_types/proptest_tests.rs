#[cfg(test)]
mod proptest_tests {
    use proptest::prelude::*;
    use crate::neo_types::{
        ScriptHash, Address, Bytes,
        ContractParameter, ContractParameterType,
        StackItem, util::*,
    };
    use primitive_types::{H160, H256};
    
    // Property: ScriptHash from/to address should be reversible
    proptest! {
        #[test]
        fn prop_scripthash_address_roundtrip(bytes in prop::array::uniform20(any::<u8>())) {
            let hash = ScriptHash::from(H160::from(bytes));
            let address = hash.to_address();
            let hash2 = ScriptHash::from_address(&address).unwrap();
            prop_assert_eq!(hash, hash2);
        }
    }
    
    // Property: H160/H256 conversions should preserve data
    proptest! {
        #[test]
        fn prop_h160_conversions(bytes in prop::array::uniform20(any::<u8>())) {
            let h160 = H160::from(bytes);
            let bytes2: [u8; 20] = h160.into();
            prop_assert_eq!(bytes, bytes2);
        }
        
        #[test]
        fn prop_h256_conversions(bytes in prop::array::uniform32(any::<u8>())) {
            let h256 = H256::from(bytes);
            let bytes2: [u8; 32] = h256.into();
            prop_assert_eq!(bytes, bytes2);
        }
    }
    
    // Property: ContractParameter serialization should be consistent
    proptest! {
        #[test]
        fn prop_contract_parameter_integer(value in any::<i64>()) {
            let param = ContractParameter::Integer(value.into());
            match param {
                ContractParameter::Integer(v) => prop_assert_eq!(v.as_i64(), value),
                _ => prop_assert!(false, "Wrong parameter type"),
            }
        }
        
        #[test]
        fn prop_contract_parameter_boolean(value in any::<bool>()) {
            let param = ContractParameter::Boolean(value);
            match param {
                ContractParameter::Boolean(v) => prop_assert_eq!(v, value),
                _ => prop_assert!(false, "Wrong parameter type"),
            }
        }
        
        #[test]
        fn prop_contract_parameter_string(value in "[a-zA-Z0-9]{0,1000}") {
            let param = ContractParameter::String(value.clone());
            match param {
                ContractParameter::String(v) => prop_assert_eq!(v, value),
                _ => prop_assert!(false, "Wrong parameter type"),
            }
        }
    }
    
    // Property: StackItem conversions should preserve values
    proptest! {
        #[test]
        fn prop_stackitem_integer(value in any::<i64>()) {
            let item = StackItem::Integer(value.into());
            if let Some(v) = item.as_int() {
                prop_assert_eq!(v, value);
            } else {
                prop_assert!(false, "Failed to get integer from StackItem");
            }
        }
        
        #[test]
        fn prop_stackitem_boolean(value in any::<bool>()) {
            let item = StackItem::Boolean(value);
            if let Some(v) = item.as_bool() {
                prop_assert_eq!(v, value);
            } else {
                prop_assert!(false, "Failed to get boolean from StackItem");
            }
        }
        
        #[test]
        fn prop_stackitem_bytestring(bytes in prop::collection::vec(any::<u8>(), 0..1000)) {
            let item = StackItem::ByteString(Bytes::from(bytes.clone()));
            if let Some(v) = item.as_bytes() {
                prop_assert_eq!(v.to_vec(), bytes);
            } else {
                prop_assert!(false, "Failed to get bytes from StackItem");
            }
        }
    }
    
    // Property: Utility functions should handle edge cases
    proptest! {
        #[test]
        fn prop_reverse_hex_roundtrip(bytes in prop::collection::vec(any::<u8>(), 0..100)) {
            let hex = hex::encode(&bytes);
            let reversed = reverse_hex(&hex);
            let reversed_again = reverse_hex(&reversed);
            prop_assert_eq!(hex, reversed_again);
        }
        
        #[test]
        fn prop_variable_length_encoding(value in 0u64..u64::MAX) {
            let encoded = encode_variable_length(value);
            // Variable length encoding should be compact
            if value < 0xFD {
                prop_assert_eq!(encoded.len(), 1);
            } else if value <= 0xFFFF {
                prop_assert_eq!(encoded.len(), 3);
            } else if value <= 0xFFFFFFFF {
                prop_assert_eq!(encoded.len(), 5);
            } else {
                prop_assert_eq!(encoded.len(), 9);
            }
        }
    }
    
    // Property: Address validation should be consistent
    proptest! {
        #[test]
        fn prop_address_validation_consistency(s in "[a-zA-Z0-9]{34}") {
            let result1 = Address::is_valid(&s);
            let result2 = Address::is_valid(&s);
            prop_assert_eq!(result1, result2);
        }
    }
    
    // Property: Numeric conversions should handle boundaries
    proptest! {
        #[test]
        fn prop_bigint_conversions(value in any::<i128>()) {
            use num_bigint::BigInt;
            let bigint = BigInt::from(value);
            let bytes = bigint.to_signed_bytes_le();
            let bigint2 = BigInt::from_signed_bytes_le(&bytes);
            prop_assert_eq!(bigint, bigint2);
        }
    }
}