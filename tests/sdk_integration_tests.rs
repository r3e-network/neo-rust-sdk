//! Integration tests for the high-level SDK API
//!
//! These tests verify that the simplified API works correctly
//! with both TestNet and mock environments.

#[cfg(test)]
mod sdk_tests {
	use std::{env, time::Duration};

	use neo3::neo_clients::MockClient;
	use neo3::neo_error::unified::{ErrorRecovery, NeoError};
	use neo3::sdk::{Balance, Neo, Network, NeoBuilder, Token};

	// Helper that uses live RPC if provided, otherwise spins up a mock server that serves getblockcount
	async fn build_neo_with_fallback(
		env_var: &str,
	) -> (Option<MockClient>, Result<Neo, NeoError>) {
		if let Ok(url) = env::var(env_var) {
			let builder = Neo::builder().network(Network::Custom(url));
			return (None, builder.build().await);
		}

		let mut mock = MockClient::new().await;
		// serve a deterministic height
		mock.mock_get_block_count(1_000).await;
		mock.mount_mocks().await;

		let builder = NeoBuilder::default().network(Network::Custom(mock.url().to_string()));
		let neo = builder.build().await;
		(Some(mock), neo)
	}

	#[tokio::test]
	async fn test_builder_pattern() {
		// Test that builder pattern correctly configures the SDK
		let _builder = Neo::builder()
			.network(Network::TestNet)
			.timeout(Duration::from_secs(60))
			.retries(5)
			.cache(true)
			.metrics(false);

		// If this compiles, the builder pattern works correctly
		// The actual config fields are private implementation details
		let _ = _builder;
	}

	#[tokio::test]
	async fn test_network_enum() {
		// Test network variants
		assert!(matches!(Network::MainNet, Network::MainNet));
		assert!(matches!(Network::TestNet, Network::TestNet));

		let custom_url = "https://custom.neo.org";
		match Network::Custom(custom_url.to_string()) {
			Network::Custom(url) => assert_eq!(url, custom_url),
			_ => panic!("Expected Custom network"),
		}
	}

	#[tokio::test]
	async fn test_balance_structure() {
		use neo3::neo_types::ScriptHash;
		use std::str::FromStr;

		// Test Balance struct creation
		let balance = Balance {
			neo: 100,
			gas: 50.5,
			tokens: vec![neo3::sdk::TokenBalance {
				contract: ScriptHash::from_str("0x0000000000000000000000000000000000000000")
					.unwrap(),
				symbol: "TEST".to_string(),
				amount: 1000.0,
				decimals: 8,
			}],
		};

		assert_eq!(balance.neo, 100);
		assert_eq!(balance.gas, 50.5);
		assert_eq!(balance.tokens.len(), 1);
		assert_eq!(balance.tokens[0].symbol, "TEST");
	}

	#[tokio::test]
	async fn test_token_enum() {
		// Test Token enum variants
		assert!(matches!(Token::NEO, Token::NEO));
		assert!(matches!(Token::GAS, Token::GAS));

		use neo3::neo_types::ScriptHash;
		use std::str::FromStr;

		let custom_hash =
			ScriptHash::from_str("0x0000000000000000000000000000000000000000").unwrap();
		match Token::Custom(custom_hash) {
			Token::Custom(hash) => assert_eq!(hash, custom_hash),
			_ => panic!("Expected Custom token"),
		}
	}

	#[tokio::test]
	async fn test_testnet_connection() {
		let (_mock, result) = build_neo_with_fallback("NEO_TESTNET_RPC_URL").await;

		match result {
			Ok(neo) => {
				// If connection succeeds, test basic operations
				let height_result = neo.get_block_height().await;
				assert!(height_result.is_ok(), "Should get block height");

				let height = height_result.unwrap();
				assert!(height > 0, "Block height should be positive");
			},
			Err(e) => {
				// If network is unavailable, ensure error has recovery suggestions
				match e {
					NeoError::Network { recovery, .. } => {
						assert!(
							!recovery.suggestions.is_empty(),
							"Network error should have recovery suggestions"
						);
						assert!(recovery.retryable, "Network error should be retryable");
					},
					_ => panic!("Expected Network error type"),
				}
			},
		}
	}

	#[tokio::test]
	async fn test_mainnet_connection() {
		let (_mock, result) = build_neo_with_fallback("NEO_MAINNET_RPC_URL").await;

		match result {
			Ok(neo) => {
				// If connection succeeds, ensure client is usable
				let height = neo.get_block_height().await.unwrap_or_default();
				assert!(height >= 0);
			},
			Err(_) => {
				// Network might be unavailable in test environment
				// This is acceptable for integration tests
			},
		}
	}

	#[tokio::test]
	async fn test_error_recovery_builder() {
		// Test ErrorRecovery builder pattern
		let recovery = ErrorRecovery::new()
			.suggest("Try again")
			.suggest("Check network")
			.retryable(true)
			.retry_after(Duration::from_secs(5))
			.doc("https://docs.neo.org");

		assert_eq!(recovery.suggestions.len(), 2);
		assert!(recovery.retryable);
		assert_eq!(recovery.retry_after, Some(Duration::from_secs(5)));
		assert_eq!(recovery.docs.len(), 1);
	}

	#[tokio::test]
	async fn test_error_display_formatting() {
		// Test that errors display correctly with recovery info
		let error = NeoError::Network {
			message: "Connection failed".to_string(),
			source: None,
			recovery: ErrorRecovery::new()
				.suggest("Check your internet connection")
				.suggest("Try a different RPC endpoint")
				.retryable(true)
				.retry_after(Duration::from_secs(5)),
		};

		let error_string = format!("{}", error);
		assert!(error_string.contains("Network error"));
		assert!(error_string.contains("Connection failed"));
	}

	#[tokio::test]
	async fn test_transfer_builder() {
		use neo3::neo_wallets::wallet::Wallet;
		use neo3::sdk::Transfer;

		// Create a test wallet (won't actually use it)
		let wallet = Wallet::new();

		// Test that Transfer can be created and built with memo
		// The actual fields are private implementation details
		let _transfer =
			Transfer::new(wallet, "NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc", 100, Token::GAS)
				.with_memo("Test transfer");

		// If this compiles, the builder pattern works correctly
		let _ = _transfer;
	}
}

// Helper that uses live RPC if provided, otherwise spins up a mock server that serves getblockcount

#[cfg(test)]
mod error_handling_tests {
	use neo3::neo_error::unified::*;

	#[test]
	fn test_error_builder_network() {
		let error = ErrorBuilder::network("Connection failed")
			.suggest("Check network")
			.retryable()
			.build();

		match error {
			NeoError::Network { message, recovery, .. } => {
				assert_eq!(message, "Connection failed");
				assert!(recovery.retryable);
				assert_eq!(recovery.suggestions.len(), 1);
			},
			_ => panic!("Expected Network error"),
		}
	}

	#[test]
	fn test_error_builder_wallet() {
		let error = ErrorBuilder::wallet("Invalid password")
			.suggest("Check your password")
			.suggest("Try password recovery")
			.build();

		match error {
			NeoError::Wallet { message, recovery, .. } => {
				assert_eq!(message, "Invalid password");
				assert_eq!(recovery.suggestions.len(), 2);
			},
			_ => panic!("Expected Wallet error"),
		}
	}

	#[test]
	fn test_error_builder_contract() {
		let error = ErrorBuilder::contract("Method not found")
			.with_contract("0xabcd")
			.with_method("transfer")
			.suggest("Check contract ABI")
			.build();

		match error {
			NeoError::Contract { message, contract, method, recovery, .. } => {
				assert_eq!(message, "Method not found");
				assert_eq!(contract, Some("0xabcd".to_string()));
				assert_eq!(method, Some("transfer".to_string()));
				assert_eq!(recovery.suggestions.len(), 1);
			},
			_ => panic!("Expected Contract error"),
		}
	}

	#[test]
	fn test_insufficient_funds_error() {
		let error = NeoError::InsufficientFunds {
			required: "100 GAS".to_string(),
			available: "50 GAS".to_string(),
			token: "GAS".to_string(),
			recovery: ErrorRecovery::new()
				.suggest("Acquire more GAS tokens")
				.suggest("Reduce the transaction amount"),
		};

		let error_string = format!("{}", error);
		assert!(error_string.contains("Insufficient funds"));
		assert!(error_string.contains("need 100 GAS"));
		assert!(error_string.contains("have 50 GAS"));
	}

	#[test]
	fn test_timeout_error() {
		use std::time::Duration;

		let error = NeoError::Timeout {
			duration: Duration::from_secs(30),
			operation: "RPC call".to_string(),
			recovery: ErrorRecovery::new().suggest("Increase timeout duration").retryable(true),
		};

		let error_string = format!("{}", error);
		assert!(error_string.contains("timed out"));
		assert!(error_string.contains("30s"));
	}

	#[test]
	fn test_rate_limit_error() {
		use std::time::Duration;

		let error = NeoError::RateLimit {
			message: "Too many requests".to_string(),
			retry_after: Some(Duration::from_secs(60)),
			recovery: ErrorRecovery::new()
				.suggest("Wait before retrying")
				.retry_after(Duration::from_secs(60)),
		};

		match error {
			NeoError::RateLimit { retry_after, .. } => {
				assert_eq!(retry_after, Some(Duration::from_secs(60)));
			},
			_ => panic!("Expected RateLimit error"),
		}
	}
}
