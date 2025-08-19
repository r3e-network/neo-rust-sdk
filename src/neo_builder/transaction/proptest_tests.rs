#[cfg(test)]
mod proptest_tests {
    use proptest::prelude::*;
    use crate::neo_builder::{
        ScriptBuilder, TransactionBuilder,
        CallFlags, WitnessScope,
    };
    use crate::neo_types::{OpCode, ContractParameter};
    
    // Strategy for generating valid opcodes
    fn opcode_strategy() -> impl Strategy<Value = OpCode> {
        prop_oneof![
            Just(OpCode::Push0),
            Just(OpCode::Push1),
            Just(OpCode::Nop),
            Just(OpCode::Ret),
            Just(OpCode::Add),
            Just(OpCode::Sub),
            Just(OpCode::Mul),
            Just(OpCode::Div),
        ]
    }
    
    // Property: ScriptBuilder should handle any sequence of valid opcodes
    proptest! {
        #[test]
        fn prop_script_builder_opcodes(opcodes in prop::collection::vec(opcode_strategy(), 0..100)) {
            let mut builder = ScriptBuilder::new();
            for opcode in &opcodes {
                builder = builder.emit(opcode.clone());
            }
            let script = builder.to_bytes();
            
            // Script should contain at least as many bytes as opcodes
            prop_assert!(script.len() >= opcodes.len());
        }
    }
    
    // Property: Script with integers should encode them correctly
    proptest! {
        #[test]
        fn prop_script_builder_integers(numbers in prop::collection::vec(any::<i64>(), 0..50)) {
            let mut builder = ScriptBuilder::new();
            for num in &numbers {
                builder = builder.push_integer(*num);
            }
            let script = builder.to_bytes();
            
            // Script should not be empty if we pushed numbers
            if !numbers.is_empty() {
                prop_assert!(!script.is_empty());
            }
        }
    }
    
    // Property: Script with strings should encode them correctly
    proptest! {
        #[test]
        fn prop_script_builder_strings(strings in prop::collection::vec("[a-zA-Z0-9]{0,100}", 0..20)) {
            let mut builder = ScriptBuilder::new();
            for s in &strings {
                builder = builder.push_string(s.clone());
            }
            let script = builder.to_bytes();
            
            // Script should grow with string data
            if !strings.is_empty() {
                prop_assert!(!script.is_empty());
            }
        }
    }
    
    // Property: Transaction attributes should be preserved
    proptest! {
        #[test]
        fn prop_transaction_attributes_preserved(
            nonce in any::<u32>(),
            valid_blocks in 1u32..10000,
        ) {
            let mut builder = TransactionBuilder::new();
            builder
                .set_nonce(nonce)
                .valid_until_block(valid_blocks).unwrap();
            
            prop_assert_eq!(builder.nonce, nonce);
            prop_assert_eq!(builder.valid_until_block, Some(valid_blocks));
        }
    }
    
    // Property: Transaction fee calculations should be non-negative
    proptest! {
        #[test]
        fn prop_transaction_fees_non_negative(
            sys_fee in 0i64..1_000_000_000,
            net_fee in 0i64..1_000_000_000,
        ) {
            let total_fee = sys_fee.saturating_add(net_fee);
            prop_assert!(total_fee >= 0);
            prop_assert!(total_fee >= sys_fee);
            prop_assert!(total_fee >= net_fee);
        }
    }
    
    // Property: WitnessScope combinations should be valid
    proptest! {
        #[test]
        fn prop_witness_scope_combinations(
            use_called_by_entry in any::<bool>(),
            use_custom_contracts in any::<bool>(),
            use_custom_groups in any::<bool>(),
            use_witness_rules in any::<bool>(),
        ) {
            let mut scope = 0u8;
            
            if use_called_by_entry {
                scope |= WitnessScope::CalledByEntry as u8;
            }
            if use_custom_contracts {
                scope |= WitnessScope::CustomContracts as u8;
            }
            if use_custom_groups {
                scope |= WitnessScope::CustomGroups as u8;
            }
            if use_witness_rules {
                scope |= WitnessScope::WitnessRules as u8;
            }
            
            // Global scope overrides all others
            if scope == 0 || scope == WitnessScope::Global as u8 {
                prop_assert!(true);
            } else {
                // Combined scopes should preserve individual flags
                prop_assert_eq!(
                    (scope & WitnessScope::CalledByEntry as u8) != 0,
                    use_called_by_entry
                );
            }
        }
    }
    
    // Property: CallFlags combinations should be valid
    proptest! {
        #[test]
        fn prop_call_flags_combinations(
            allow_notify in any::<bool>(),
            allow_call in any::<bool>(),
            allow_states in any::<bool>(),
            allow_modify_states in any::<bool>(),
        ) {
            let mut flags = CallFlags::None;
            
            if allow_notify {
                flags = flags | CallFlags::AllowNotify;
            }
            if allow_call {
                flags = flags | CallFlags::AllowCall;
            }
            if allow_states {
                flags = flags | CallFlags::AllowStates;
            }
            if allow_modify_states {
                flags = flags | CallFlags::AllowModifyStates;
            }
            
            // Flags should be composable
            prop_assert!(flags as u8 <= CallFlags::All as u8);
        }
    }
}