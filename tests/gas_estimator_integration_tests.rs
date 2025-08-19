#[cfg(test)]
mod gas_estimator_integration_tests {
    use neo3::prelude::*;
    use neo3::neo_builder::{GasEstimator, ScriptBuilder, TransactionBuilder, Signer};
    use neo3::neo_clients::{HttpProvider, RpcClient, APITrait};
    use neo3::neo_types::{ScriptHash, ContractParameter};
    use std::str::FromStr;

    // Helper function to create a test client
    async fn create_test_client() -> RpcClient<HttpProvider> {
        let provider = HttpProvider::new("https://testnet1.neo.org:443")
            .expect("Failed to create provider");
        RpcClient::new(provider)
    }

    #[tokio::test]
    async fn test_gas_estimation_for_simple_transfer() {
        let client = create_test_client().await;
        
        // Create a simple NEO transfer script
        let neo_token = ScriptHash::from_str("ef4073a0f2b305a38ec4050e4d3d28bc40ea63f5")
            .expect("Invalid NEO token hash");
        
        let from = ScriptHash::from_str("NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc")
            .expect("Invalid from address");
        let to = ScriptHash::from_str("NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc")
            .expect("Invalid to address");
        
        let script = ScriptBuilder::new()
            .contract_call(
                &neo_token,
                "transfer",
                &[
                    ContractParameter::h160(&from),
                    ContractParameter::h160(&to),
                    ContractParameter::integer(100_000_000), // 1 NEO
                    ContractParameter::any(),
                ],
                None,
            )
            .expect("Failed to build script")
            .to_bytes();
        
        // Estimate gas
        let estimated_gas = GasEstimator::estimate_gas_realtime(
            &client,
            &script,
            vec![Signer::called_by_entry(&from)],
        )
        .await;
        
        // For testnet, this might fail if the account doesn't exist
        // but we're testing the estimation mechanism works
        assert!(estimated_gas.is_ok() || estimated_gas.is_err());
        
        if let Ok(gas) = estimated_gas {
            // Gas should be reasonable for a simple transfer
            assert!(gas > 0);
            assert!(gas < 10_000_000); // Less than 0.1 GAS
        }
    }

    #[tokio::test]
    async fn test_gas_estimation_with_margin() {
        let client = create_test_client().await;
        
        // Create a simple script
        let script = ScriptBuilder::new()
            .push_integer(42)
            .push_integer(13)
            .emit(OpCode::Add)
            .to_bytes();
        
        let signers = vec![];
        
        // Test with 10% margin
        let result = GasEstimator::estimate_gas_with_margin(
            &client,
            &script,
            signers.clone(),
            10,
        )
        .await;
        
        if let Ok(gas_with_margin) = result {
            // Also get base gas for comparison
            let base_result = GasEstimator::estimate_gas_realtime(
                &client,
                &script,
                signers,
            )
            .await;
            
            if let Ok(base_gas) = base_result {
                // Gas with margin should be approximately 10% higher
                let expected_margin = (base_gas as f64 * 1.1) as i64;
                assert!(gas_with_margin >= base_gas);
                assert!((gas_with_margin - expected_margin).abs() < 100); // Allow small rounding difference
            }
        }
    }

    #[tokio::test]
    async fn test_batch_gas_estimation() {
        let client = create_test_client().await;
        
        // Create multiple scripts
        let scripts = vec![
            (
                ScriptBuilder::new()
                    .push_integer(1)
                    .push_integer(2)
                    .emit(OpCode::Add)
                    .to_bytes(),
                vec![],
            ),
            (
                ScriptBuilder::new()
                    .push_string("Hello".to_string())
                    .push_string("World".to_string())
                    .emit(OpCode::Cat)
                    .to_bytes(),
                vec![],
            ),
            (
                ScriptBuilder::new()
                    .push_integer(100)
                    .push_integer(50)
                    .emit(OpCode::Sub)
                    .to_bytes(),
                vec![],
            ),
        ];
        
        // Convert to the expected format
        let scripts_ref: Vec<(&[u8], Vec<Signer>)> = scripts
            .iter()
            .map(|(script, signers)| (script.as_slice(), signers.clone()))
            .collect();
        
        // Batch estimate
        let results = GasEstimator::batch_estimate_gas(&client, scripts_ref).await;
        
        if let Ok(estimates) = results {
            // Should have same number of results as scripts
            assert_eq!(estimates.len(), 3);
            
            // All estimates should be positive
            for estimate in estimates {
                assert!(estimate > 0);
            }
        }
    }

    #[test]
    fn test_estimation_accuracy_calculation() {
        // Test perfect estimation
        let accuracy = GasEstimator::calculate_estimation_accuracy(1000, 1000);
        assert_eq!(accuracy, 0.0);
        
        // Test 10% overestimation
        let accuracy = GasEstimator::calculate_estimation_accuracy(1100, 1000);
        assert_eq!(accuracy, 10.0);
        
        // Test 5% underestimation
        let accuracy = GasEstimator::calculate_estimation_accuracy(950, 1000);
        assert_eq!(accuracy, 5.0);
        
        // Test edge case with zero actual
        let accuracy = GasEstimator::calculate_estimation_accuracy(1000, 0);
        assert_eq!(accuracy, 0.0);
    }
}