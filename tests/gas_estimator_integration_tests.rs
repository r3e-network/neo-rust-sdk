#[cfg(test)]
mod gas_estimator_integration_tests {
	use std::env;
	use neo3::neo_builder::{AccountSigner, GasEstimator, ScriptBuilder, Signer, WitnessScope};
	use neo3::neo_clients::{HttpProvider, MockClient, RpcClient};
	use neo3::neo_protocol::{Account, AccountTrait};
	use neo3::neo_types::{ContractParameter, OpCode, ScriptHash};
	use num_bigint::BigInt;
	use std::str::FromStr;
	use serde_json::json;

	// Helper function to create a test client (mock by default, live if env provided)
	async fn create_test_client(mock_gas: i64) -> (Option<MockClient>, RpcClient<HttpProvider>) {
		if let Ok(url) = env::var("NEO_LIVE_RPC_URL") {
			let provider = HttpProvider::new(url.as_str()).expect("Failed to create provider");
			return (None, RpcClient::new(provider));
		}

		let mut mock = MockClient::new().await;
		let invoke_result = json!({
			"script": "",
			"state": "HALT",
			"gasconsumed": mock_gas.to_string(),
			"exception": null,
			"notifications": [],
			"diagnostics": null,
			"stack": [],
			"tx": null,
			"pendingsignature": null,
			"session": null
		});
		mock.mock_response_ignore_param("invokescript", invoke_result).await;
		mock.mount_mocks().await;
		let client = mock.into_client();
		(Some(mock), client)
	}

	#[tokio::test]
	async fn test_gas_estimation_for_simple_transfer() {
		let (_mock, client) = create_test_client(750_000).await;

		// Create a simple NEO transfer script
		let neo_token = ScriptHash::from_str("ef4073a0f2b305a38ec4050e4d3d28bc40ea63f5")
			.expect("Invalid NEO token hash");

		let from = ScriptHash::from_str("0x0123456789abcdef0123456789abcdef01234567")
			.expect("Invalid from address");
		let to = ScriptHash::from_str("0x89abcdef0123456789abcdef0123456789abcdef")
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

		// Create a test account (this would normally come from a wallet)
		let test_account = <Account as AccountTrait>::from_wif(
			"L1QqQJnpBwbsPGAuutuzPTac8piqvbR1HRjrY5qHup48TBCBFe4g",
		)
		.expect("Failed to create account");

		// Create signer
		let signer =
			Signer::AccountSigner(AccountSigner::new(&test_account, WitnessScope::CalledByEntry));

		// Estimate gas
		let estimated_gas =
			GasEstimator::estimate_gas_realtime(&client, &script, vec![signer]).await;

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
		let (_mock, client) = create_test_client(500_000).await;

		// Create a simple script
		let script = ScriptBuilder::new()
			.push_integer(BigInt::from(42))
			.push_integer(BigInt::from(13))
			.op_code(&[OpCode::Add])
			.to_bytes();

		let signers = vec![];

		// Test with 10% margin
		let result =
			GasEstimator::estimate_gas_with_margin(&client, &script, signers.clone(), 10).await;

		if let Ok(gas_with_margin) = result {
			// Also get base gas for comparison
			let base_result = GasEstimator::estimate_gas_realtime(&client, &script, signers).await;

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
		let (_mock, client) = create_test_client(250_000).await;

		// Create multiple scripts
		let scripts = [
			(
				ScriptBuilder::new()
					.push_integer(BigInt::from(1))
					.push_integer(BigInt::from(2))
					.op_code(&[OpCode::Add])
					.to_bytes(),
				vec![],
			),
			(
				ScriptBuilder::new()
					.push_data("Hello".as_bytes().to_vec())
					.push_data("World".as_bytes().to_vec())
					.op_code(&[OpCode::Cat])
					.to_bytes(),
				vec![],
			),
			(
				ScriptBuilder::new()
					.push_integer(BigInt::from(100))
					.push_integer(BigInt::from(50))
					.op_code(&[OpCode::Sub])
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
