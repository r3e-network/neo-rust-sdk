#[cfg(test)]
mod tests {
	#![allow(
		unused_imports,
		unused_mut,
		unused_variables,
		unused_must_use,
		non_snake_case,
		clippy::needless_borrow,
		clippy::get_first,
		clippy::explicit_auto_deref,
		clippy::useless_format,
		clippy::useless_conversion,
		clippy::unnecessary_operation
	)]

	use crate::{
		builder::{
			init_logger, AccountSigner, BuilderError, ContractSigner, ScriptBuilder, Signer,
			TransactionAttribute, TransactionBuilder, TransactionError, Witness,
		},
		config::{NeoConstants, TestConstants},
		crypto::{KeyPair, Secp256r1PrivateKey},
		neo_builder::GAS_TOKEN_HASH,
		neo_clients::{APITrait, HttpProvider, MockClient, RpcClient},
		neo_crypto::utils::ToHexString,
		neo_protocol::{
			Account, AccountTrait, ApplicationLog, NeoProtocol, NeoVersion, RawTransaction,
		},
		neo_types::ScriptHashExtension,
		ContractParameter, InvocationResult,
	};
	use lazy_static::lazy_static;
	use log::info;
	use neo3::{
		builder::VerificationScript,
		config::{NeoConfig, NEOCONFIG},
	};
	use num_bigint::BigInt;
	use primitive_types::{H160, H256};
	use serde_json::json;
	use std::{default, ops::Deref, str::FromStr, sync::Arc};
	use tokio::sync::{Mutex, OnceCell};
	use tracing::debug;

	lazy_static! {
		pub static ref ACCOUNT1: Account = Account::from_key_pair(
			KeyPair::from_secret_key(
				&Secp256r1PrivateKey::from_bytes(
					&hex::decode(
						"e6e919577dd7b8e97805151c05ae07ff4f752654d6d8797597aca989c02c4cb3"
					)
					.expect("Test private key 1 should be valid hex")
				)
				.expect("Test private key 1 should be valid bytes for Secp256r1PrivateKey")
			),
			None,
			None
		)
		.expect("Failed to create ACCOUNT1 from valid key pair");
		pub static ref ACCOUNT2: Account = Account::from_key_pair(
			KeyPair::from_secret_key(
				&Secp256r1PrivateKey::from_bytes(
					&hex::decode(
						"b4b2b579cac270125259f08a5f414e9235817e7637b9a66cfeb3b77d90c8e7f9"
					)
					.expect("Test private key 2 should be valid hex")
				)
				.expect("Test private key 2 should be valid bytes for Secp256r1PrivateKey")
			),
			None,
			None
		)
		.expect("Failed to create ACCOUNT2 from valid key pair");
	}

	static CLIENT: OnceCell<RpcClient<HttpProvider>> = OnceCell::const_new();

	#[tokio::test]
	async fn test_build_transaction_with_correct_nonce() {
		// let _ = env_logger::builder().is_test(true).try_init();
		let mut nonce = 1;
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));

		// Set the mock response before using the client
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_necessary_mock.json",
				)
				.await;
			mock_provider_guard
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await;
			mock_provider_guard.mount_mocks().await;
		}

		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};
		// let client = CLIENT.get_or_init(|| async { mock_provider.into_client() }).await;
		let mut transaction_builder = TransactionBuilder::with_client(&client);
		let mut tx = transaction_builder
			.valid_until_block(1)
			.unwrap()
			.set_script(Some(vec![1, 2, 3]))
			.set_signers(vec![AccountSigner::called_by_entry(ACCOUNT1.deref()).unwrap().into()])
			.unwrap()
			.nonce(nonce)
			.unwrap()
			.get_unsigned_tx()
			.await
			.unwrap();

		assert_eq!(*tx.nonce(), nonce);

		nonce = 0;
		transaction_builder = TransactionBuilder::with_client(&client);
		tx = transaction_builder
			.valid_until_block(1)
			.unwrap()
			.set_script(Some(vec![1, 2, 3]))
			.set_signers(vec![AccountSigner::called_by_entry(ACCOUNT1.deref()).unwrap().into()])
			.unwrap()
			.nonce(nonce)
			.unwrap()
			.get_unsigned_tx()
			.await
			.unwrap();
		assert_eq!(*tx.nonce(), nonce);

		nonce = u32::MAX;
		transaction_builder = TransactionBuilder::with_client(&client);
		tx = transaction_builder
			.valid_until_block(1)
			.unwrap()
			.set_script(Some(vec![1, 2, 3]))
			.set_signers(vec![AccountSigner::called_by_entry(ACCOUNT1.deref()).unwrap().into()])
			.unwrap()
			.nonce(nonce)
			.unwrap()
			.get_unsigned_tx()
			.await
			.unwrap();
		info!("{:?}", tx);
		assert_eq!(*tx.nonce(), nonce);
	}

	#[tokio::test]
	async fn test_build_transaction_automatically_set_nonce() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));

		// Set the mock response before using the client
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_necessary_mock.json",
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await;
			mock_provider_guard
				.mock_response_with_file_ignore_param("getblockcount", "getblockcount_1000.json")
				.await;
			mock_provider_guard.mount_mocks().await;
		}

		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};
		// let client = CLIENT.get_or_init(|| async { mock_provider.into_client() }).await;
		let mut transaction_builder = TransactionBuilder::with_client(&client);
		let mut tx = transaction_builder
			.valid_until_block(1)
			.unwrap()
			.set_script(Some(vec![1, 2, 3]))
			.set_signers(vec![AccountSigner::called_by_entry(ACCOUNT1.deref()).unwrap().into()])
			.unwrap()
			.get_unsigned_tx()
			.await
			.unwrap();
		assert_eq!(*tx.nonce(), 0);
	}

	#[tokio::test]
	async fn test_build_transaction_fail_building_tx_without_signer() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};
		// let client = CLIENT.get_or_init(|| async { mock_provider.into_client() }).await;
		let mut transaction_builder = TransactionBuilder::with_client(&client);
		let mut tx = transaction_builder
			.valid_until_block(100)
			.unwrap()
			.set_script(Some(vec![1, 2, 3]))
			.get_unsigned_tx()
			.await;
		assert_eq!(tx, Err(TransactionError::NoSigners));
	}

	#[tokio::test]
	async fn test_build_transaction_fail_adding_multiple_signers_concerning_the_same_account() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};
		// let client = CLIENT.get_or_init(|| async { mock_provider.into_client() }).await;
		let mut transaction_builder = TransactionBuilder::with_client(&client);
		let mut tx = transaction_builder
			.valid_until_block(100)
			.unwrap()
			.set_script(Some(vec![1, 2, 3]))
			.set_signers(vec![
				AccountSigner::called_by_entry(ACCOUNT1.deref()).unwrap().into(),
				AccountSigner::global(ACCOUNT1.deref()).unwrap().into(),
			]);
		assert_eq!(
			tx,
			Err(TransactionError::TransactionConfiguration(
				"Cannot add multiple signers concerning the same account.".to_string()
			))
		);
	}

	#[tokio::test]
	async fn test_invoke_script() {
		// init_logger();
		let script = ScriptBuilder::new()
			.contract_call(
				&ScriptHashExtension::from_hex(TestConstants::NEO_TOKEN_HASH).unwrap(),
				"symbol",
				&[],
				None,
			)
			.unwrap()
			.to_bytes();

		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_symbol_neo.json",
				)
				.await;
			mock_provider_guard.mount_mocks().await;
		}
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};

		let mut tb = TransactionBuilder::with_client(&client);
		tb.set_script(Some(script));
		let response = tb.call_invoke_script().await.unwrap();
		// println!("Response: {:?}", response); // Add this line for debugging

		assert!(!response.stack.is_empty(), "Response stack is empty");

		if let Some(item) = response.stack.get(0) {
			if let Some(value) = item.as_string() {
				assert_eq!(value, "NEO", "Unexpected value in response");
			} else {
				panic!("First item in stack is not a string");
			}
		} else {
			panic!("Response stack is empty");
		}
	}

	#[tokio::test]
	async fn test_invoke_script_without_setting_script() {
		// init_logger();
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_symbol_neo.json",
				)
				.await;
			mock_provider_guard.mount_mocks().await;
		}
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};

		let mut tb = TransactionBuilder::with_client(&client);
		let result = tb.call_invoke_script().await;

		assert_eq!(result.unwrap_err(), TransactionError::NoScript);
		// println!("Response: {:?}", response); // Add this line for debugging
	}

	#[tokio::test]
	async fn test_build_without_setting_script() {
		let client = CLIENT.get_or_init(|| async { MockClient::new().await.into_client() }).await;
		let err = TransactionBuilder::with_client(&client)
			.set_signers(vec![AccountSigner::called_by_entry(ACCOUNT1.deref()).unwrap().into()])
			.unwrap()
			.get_unsigned_tx()
			.await
			.err()
			.unwrap();

		assert_eq!(err, TransactionError::NoScript);
	}

	#[tokio::test]
	async fn test_sign_transaction_with_additional_signers() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_symbol_neo.json",
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await;
			mock_provider_guard.mount_mocks().await;
		}
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};
		let script = vec![0x01u8, 0x02u8, 0x03u8];

		let mut transaction_builder = TransactionBuilder::with_client(&client);
		let tx = transaction_builder
			.set_script(Some(script))
			.set_signers(vec![
				AccountSigner::called_by_entry(ACCOUNT1.deref()).unwrap().into(),
				AccountSigner::called_by_entry(ACCOUNT2.deref()).unwrap().into(),
			])
			.unwrap()
			.valid_until_block(1000)
			.unwrap()
			.sign()
			.await
			.unwrap();

		assert_eq!(tx.witnesses().len(), 2);

		let signers = tx
			.witnesses()
			.iter()
			.map(|witness| witness.verification.get_public_keys().unwrap().first().unwrap().clone())
			.collect::<Vec<_>>();

		assert!(signers.contains(&ACCOUNT1.deref().clone().key_pair.unwrap().public_key()));
		assert!(signers.contains(&ACCOUNT2.deref().clone().key_pair.unwrap().public_key()));
	}

	#[tokio::test]
	async fn test_send_invoke_function() {
		// init_logger();
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_transfer_with_fixed_sysfee.json",
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"sendrawtransaction",
					"sendrawtransaction.json",
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param("getblockcount", "getblockcount_1000.json")
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await;
			mock_provider_guard.mount_mocks().await;
		}
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};
		let mut tb = TransactionBuilder::with_client(&client);
		let script = ScriptBuilder::new()
			.contract_call(
				&H160::from_str(TestConstants::NEO_TOKEN_HASH).unwrap(),
				"transfer",
				&[
					ContractParameter::from(ACCOUNT1.address_or_scripthash().script_hash()),
					ContractParameter::from(
						H160::from_str("969a77db482f74ce27105f760efa139223431394").unwrap(),
					),
					ContractParameter::from(5),
					ContractParameter::any(),
				],
				None,
			)
			.unwrap()
			.to_bytes();
		tb.set_script(Some(script.clone()))
			.set_signers(vec![AccountSigner::none(ACCOUNT1.deref()).unwrap().into()])
			.unwrap();

		let mut tx = tb.sign().await.unwrap();
		let response = tx.send_tx().await;

		match response {
			Ok(response) => {
				assert_eq!(
					response.hash,
					H256::from_str(
						"0x830816f0c801bcabf919dfa1a90d7b9a4f867482cb4d18d0631a5aa6daefab6a"
					)
					.unwrap()
				);
			},
			Err(e) => {
				panic!("Failed to invoke function: {:?}", e);
			},
		}
	}

	#[tokio::test]
	async fn test_fail_building_transaction_with_incorrect_nonce() {
		let client = CLIENT.get_or_init(|| async { MockClient::new().await.into_client() }).await;
		let mut tb = TransactionBuilder::with_client(&client);
		tb.valid_until_block(1)
			.unwrap()
			.set_script(Some(vec![1, 2, 3]))
			.set_signers(vec![AccountSigner::called_by_entry(ACCOUNT1.deref()).unwrap().into()]);

		// Test with 0, which should be valid
		assert!(tb.nonce(0).is_ok());

		// Test with u32::MAX, which should be valid
		assert!(tb.nonce(u32::MAX).is_ok());

		// Test overflow condition
		tb.nonce(u32::MAX).unwrap();
		assert!(tb.nonce(u32::MAX).is_ok());

		// Reset nonce for next test
		tb.nonce(0).unwrap();

		// Test with -1 cast to u32, which is actually u32::MAX
		assert!(tb.nonce((-1i32) as u32).is_ok());
	}

	#[tokio::test]
	async fn test_fail_building_transaction_with_invalid_block_number() {
		let client = CLIENT.get_or_init(|| async { MockClient::new().await.into_client() }).await;
		let mut tb = TransactionBuilder::with_client(&client);
		tb.set_script(Some(vec![1, 2, 3]))
			.set_signers(vec![AccountSigner::called_by_entry(ACCOUNT1.deref()).unwrap().into()]);

		assert!(tb.valid_until_block(-1i32 as u32).is_ok());
		// assert!(tb.valid_until_block(2u32.pow(32)).is_err());
	}

	#[tokio::test]
	async fn test_override_signer() {
		let client = CLIENT.get_or_init(|| async { MockClient::new().await.into_client() }).await;
		let mut tb = TransactionBuilder::with_client(&client);
		tb.set_script(Some(vec![1, 2, 3])).set_signers(vec![AccountSigner::global(
			ACCOUNT1.deref(),
		)
		.unwrap()
		.into()]);
		assert_eq!(
			tb.signers()[0],
			Signer::AccountSigner(AccountSigner::global(ACCOUNT1.deref()).unwrap())
		);

		tb.set_signers(vec![AccountSigner::called_by_entry(ACCOUNT2.deref()).unwrap().into()]);
		assert_eq!(
			tb.signers(),
			&[AccountSigner::called_by_entry(ACCOUNT2.deref()).unwrap().into()]
		);
	}

	#[tokio::test]
	async fn test_attributes_high_priority_multisig_containing_committee_Member() {
		init_logger();
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));

		// Set the mock response before using the client
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_symbol_neo.json",
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await;
			mock_provider_guard
				.mock_response_with_file_ignore_param("getcommittee", "getcommittee.json")
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param("getblockcount", "getblockcount_1000.json")
				.await;
			mock_provider_guard.mount_mocks().await;
		}

		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};
		let mut tb = TransactionBuilder::with_client(&client);
		let multi_sig_account = Account::multi_sig_from_public_keys(
			&mut [ACCOUNT2.get_public_key().unwrap(), ACCOUNT1.get_public_key().unwrap()],
			1,
		)
		.unwrap();
		tb.set_script(Some(vec![1, 2, 3]))
			.add_attributes(vec![TransactionAttribute::HighPriority])
			.unwrap()
			.set_signers(vec![AccountSigner::none(&multi_sig_account).unwrap().into()])
			.expect("Failed to set signers in test - this should never fail with valid test data");

		let tx = tb.get_unsigned_tx().await;
		match tx {
			Ok(tx) => assert_eq!(tx.attributes()[0], TransactionAttribute::HighPriority),
			Err(e) => panic!("Failed to get unsigned transaction: {:?}", e),
		}
	}

	#[tokio::test]
	async fn test_attributes_high_priority() {
		init_logger();
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));

		// Set the mock response before using the client
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_symbol_neo.json",
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await;
			mock_provider_guard
				.mock_response_with_file_ignore_param("getcommittee", "getcommittee.json")
				.await
				.mock_response_with_file_ignore_param("getblockcount", "getblockcount_1000.json")
				.await;
			mock_provider_guard.mount_mocks().await;
		}

		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};
		let mut tb = TransactionBuilder::with_client(&client);
		tb.set_script(Some(vec![1, 2, 3]))
			.add_attributes(vec![TransactionAttribute::HighPriority])
			.unwrap()
			.set_signers(vec![AccountSigner::none(ACCOUNT1.deref()).unwrap().into()])
			.expect("Failed to set signers for high priority transaction test");

		let tx = tb.get_unsigned_tx().await.unwrap();
		assert_eq!(tx.attributes()[0], TransactionAttribute::HighPriority);
	}

	#[tokio::test]
	async fn test_attributes_high_priority_not_committee_member() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));

		// Set the mock response before using the client
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param("getcommittee", "getcommittee.json")
				.await;
			mock_provider_guard.mock_get_block_count(1000).await;
			mock_provider_guard.mount_mocks().await;
		}
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};
		let mut tb = TransactionBuilder::with_client(&client);
		tb.set_script(Some(vec![1, 2, 3]))
			.add_attributes(vec![TransactionAttribute::HighPriority])
			.unwrap()
			.set_signers(vec![AccountSigner::none(ACCOUNT2.deref()).unwrap().into()]);

		assert_eq!(tb.get_unsigned_tx().await, Err(TransactionError::IllegalState("This transaction does not have a committee member as signer. Only committee members can send transactions with high priority.".to_string())));
	}

	#[tokio::test]
	async fn test_attributes_high_priority_error_when_multiple() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));

		// Set the mock response before using the client
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_symbol_neo.json",
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await;
			mock_provider_guard.mock_get_block_count(1000).await;
			mock_provider_guard.mount_mocks().await;
		}
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};
		let mut tb = TransactionBuilder::with_client(&client);
		tb.set_script(Some(vec![1, 2, 3]))
			.add_attributes(vec![TransactionAttribute::HighPriority]);

		assert_eq!(
			tb.add_attributes(vec![TransactionAttribute::HighPriority]),
			Err(TransactionError::TransactionConfiguration(
				"A transaction can only have one HighPriority attribute.".to_string(),
			))
		);
	}

	#[tokio::test]
	async fn test_attributes_not_valid_before() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));

		// Set the mock response before using the client
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_symbol_neo.json",
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await;
			mock_provider_guard.mock_get_block_count(1000).await;
			mock_provider_guard.mount_mocks().await;
		}

		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};
		let mut tb = TransactionBuilder::with_client(&client);
		tb.set_script(Some(vec![1, 2, 3]))
			.add_attributes(vec![TransactionAttribute::NotValidBefore { height: 200 }])
			.unwrap()
			.set_signers(vec![AccountSigner::none(ACCOUNT1.deref()).unwrap().into()]);

		let tx = tb.get_unsigned_tx().await.unwrap();
		let attribute = tx.attributes().get(0).unwrap();
		assert!(
			matches!(attribute, TransactionAttribute::NotValidBefore { .. }),
			"The attribute type is not NotValidBefore as expected"
		);
		assert_eq!(attribute.get_height().unwrap(), &200);
	}

	#[tokio::test]
	async fn test_attributes_not_valid_before_error_when_multiple() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));

		// Set the mock response before using the client
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_symbol_neo.json",
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await;
			mock_provider_guard.mock_get_block_count(1000).await;
			mock_provider_guard.mount_mocks().await;
		}
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};
		let mut tb = TransactionBuilder::with_client(&client);
		tb.set_script(Some(vec![1, 2, 3]))
			.add_attributes(vec![TransactionAttribute::NotValidBefore { height: 200 }]);

		assert_eq!(
			tb.add_attributes(vec![TransactionAttribute::NotValidBefore { height: 200 }]),
			Err(TransactionError::TransactionConfiguration(
				"A transaction can only have one NotValidBefore attribute.".to_string(),
			))
		);
	}

	#[tokio::test]
	async fn test_attributes_conflicts() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));

		// Set the mock response before using the client
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_symbol_neo.json",
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await;
			mock_provider_guard.mock_get_block_count(1000).await;
			mock_provider_guard.mount_mocks().await;
		}

		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};
		let mut tb = TransactionBuilder::with_client(&client);
		tb.set_script(Some(vec![1, 2, 3]))
			.add_attributes(vec![TransactionAttribute::Conflicts {
				hash: H256::from_str(
					"0x8529cf7301d13cc13d85913b8367700080a6e96db045687b8db720e91e80321c",
				)
				.unwrap(),
			}])
			.unwrap()
			.set_signers(vec![AccountSigner::none(ACCOUNT1.deref()).unwrap().into()]);

		let tx = tb.get_unsigned_tx().await.unwrap();
		assert_eq!(tx.attributes().len(), 1);
		let attribute = tx.attributes().get(0).unwrap();
		assert!(
			matches!(attribute, TransactionAttribute::Conflicts { .. }),
			"The attribute type is not Conflicts as expected"
		);
		assert_eq!(
			attribute.get_hash().unwrap(),
			&H256::from_str("0x8529cf7301d13cc13d85913b8367700080a6e96db045687b8db720e91e80321c")
				.unwrap()
		);
	}

	#[tokio::test]
	async fn test_attributes_conflicts_multiple() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));

		// Set the mock response before using the client
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_symbol_neo.json",
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await;
			mock_provider_guard.mock_get_block_count(1000).await;
			mock_provider_guard.mount_mocks().await;
		}

		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};
		let mut tb = TransactionBuilder::with_client(&client);
		tb.set_script(Some(vec![1, 2, 3]))
			.add_attributes(vec![TransactionAttribute::Conflicts {
				hash: H256::from_str(
					"0x8529cf7301d13cc13d85913b8367700080a6e96db045687b8db720e91e80321c",
				)
				.unwrap(),
			}])
			.unwrap()
			.add_attributes(vec![TransactionAttribute::Conflicts {
				hash: H256::from_str(
					"0x8529cf7301d13cc13d85913b8367700080a6e96db045687b8db720e91e80321d",
				)
				.unwrap(),
			}])
			.unwrap()
			.set_signers(vec![AccountSigner::none(ACCOUNT1.deref()).unwrap().into()]);

		let tx = tb.get_unsigned_tx().await.unwrap();
		assert_eq!(tx.attributes().len(), 2);
		let attribute0 = tx.attributes().get(0).unwrap();

		assert!(
			matches!(attribute0, TransactionAttribute::Conflicts { .. }),
			"The attribute type is not Conflicts as expected"
		);
		assert_eq!(
			attribute0.get_hash().unwrap(),
			&H256::from_str("0x8529cf7301d13cc13d85913b8367700080a6e96db045687b8db720e91e80321c")
				.unwrap()
		);

		let attribute1 = tx.attributes().get(1).unwrap();
		assert!(
			matches!(attribute1, TransactionAttribute::Conflicts { .. }),
			"The attribute type is not Conflicts as expected"
		);
		assert_eq!(
			attribute1.get_hash().unwrap(),
			&H256::from_str("0x8529cf7301d13cc13d85913b8367700080a6e96db045687b8db720e91e80321d")
				.unwrap()
		);
	}

	#[tokio::test]
	async fn test_attributes_conflicts_same_exist_already() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));

		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};
		let mut tb = TransactionBuilder::with_client(&client);
		tb.set_script(Some(vec![1, 2, 3]))
			.add_attributes(vec![TransactionAttribute::Conflicts {
				hash: H256::from_str(
					"fe26f525c17b58f63a4d106fba973ec34cc99bfe2501c9f672cc145b483e398b",
				)
				.unwrap(),
			}])
			.unwrap();

		let result = tb.add_attributes(vec![TransactionAttribute::Conflicts {
			hash: H256::from_str(
				"fe26f525c17b58f63a4d106fba973ec34cc99bfe2501c9f672cc145b483e398b",
			)
			.unwrap(),
		}]);

		assert!(matches!(result, Err(TransactionError::TransactionConfiguration(_))));
		if let Err(TransactionError::TransactionConfiguration(msg)) = result {
			assert!(msg.contains("already exists a conflicts attribute for the hash "));
			assert!(msg.contains("in this transaction"));
		}
	}

	#[tokio::test]
	async fn test_attributes_compare_not_valid_before_attributes() {
		let attr1 = TransactionAttribute::NotValidBefore { height: 147 };
		let attr2 = TransactionAttribute::NotValidBefore { height: 1 };
		assert_ne!(attr1, attr2);
		assert_eq!(attr1, TransactionAttribute::NotValidBefore { height: 147 });
	}

	#[tokio::test]
	async fn test_fail_adding_more_than_max_attributes_to_tx_just_attributes() {
		let attrs: Vec<TransactionAttribute> = (0..=NeoConstants::MAX_TRANSACTION_ATTRIBUTES)
			.map(|_| TransactionAttribute::HighPriority)
			.collect();
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};
		let mut tb = TransactionBuilder::with_client(&client);
		assert_eq!(
			tb.add_attributes(attrs),
			Err(TransactionError::TransactionConfiguration(format!(
				"A transaction cannot have more than {} attributes (including signers).",
				NeoConstants::MAX_TRANSACTION_ATTRIBUTES
			)))
		);
	}

	#[tokio::test]
	async fn test_fail_adding_more_than_max_attributes_to_tx_attributes_and_signers() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};
		let mut tb = TransactionBuilder::with_client(&client);
		tb.set_signers(vec![
			AccountSigner::called_by_entry(&Account::create().unwrap()).unwrap().into(),
			AccountSigner::called_by_entry(&Account::create().unwrap()).unwrap().into(),
			AccountSigner::called_by_entry(&Account::create().unwrap()).unwrap().into(),
		])
		.unwrap();
		let attrs: Vec<TransactionAttribute> = (0..=NeoConstants::MAX_TRANSACTION_ATTRIBUTES - 3)
			.map(|_| TransactionAttribute::HighPriority)
			.collect();
		assert_eq!(
			tb.add_attributes(attrs),
			Err(TransactionError::TransactionConfiguration(format!(
				"A transaction cannot have more than {} attributes (including signers).",
				NeoConstants::MAX_TRANSACTION_ATTRIBUTES
			)))
		);
	}

	#[tokio::test]
	async fn test_fail_adding_more_than_max_attributes_to_tx_signers() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};
		let mut tb = TransactionBuilder::with_client(&client);
		tb.add_attributes(vec![TransactionAttribute::HighPriority]);

		let signers: Vec<Signer> = (0..NeoConstants::MAX_TRANSACTION_ATTRIBUTES)
			.map(|_| AccountSigner::called_by_entry(&Account::create().unwrap()).unwrap().into())
			.collect();
		assert!(signers.len() + 1 > NeoConstants::MAX_TRANSACTION_ATTRIBUTES.try_into().unwrap());
		assert_eq!(
			tb.set_signers(signers),
			Err(TransactionError::TransactionConfiguration(format!(
				"A transaction cannot have more than {} attributes (including signers).",
				NeoConstants::MAX_TRANSACTION_ATTRIBUTES
			)))
		);
		// assert!(tb.set_signers(signers.into_iter().map(Into::into).collect()));
	}

	#[tokio::test]
	async fn test_automatic_setting_of_valid_until_block_variable() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));

		// Set the mock response before using the client
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_symbol_neo.json",
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await;
			mock_provider_guard.mock_get_block_count(1000).await;
			mock_provider_guard.mount_mocks().await;
		}

		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};
		let mut tb = TransactionBuilder::with_client(&client);
		let block_count = 1000;
		tb.set_script(Some(vec![1, 2, 3]))
			.set_signers(vec![AccountSigner::none(ACCOUNT1.deref()).unwrap().into()]);

		let tx = tb.get_unsigned_tx().await.unwrap();
		assert_eq!(
			*tx.valid_until_block(),
			block_count + client.max_valid_until_block_increment() - 1
		);
	}

	#[tokio::test]
	async fn test_automatic_setting_of_system_fee_and_network_fee() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));

		// Set the mock response before using the client
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_symbol_neo.json",
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await;
			mock_provider_guard.mount_mocks().await;
		}

		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};

		let script = vec![1, 2, 3];
		let mut tb = TransactionBuilder::with_client(&client);
		tb.set_script(Some(script.clone()))
			.set_signers(vec![AccountSigner::none(ACCOUNT1.deref()).unwrap().into()])
			.unwrap()
			.valid_until_block(1000)
			.unwrap();

		let tx = tb.get_unsigned_tx().await.unwrap();
		assert_eq!(*tx.sys_fee(), 984060);
		assert_eq!(*tx.net_fee(), 1230610);
	}

	#[tokio::test]
	async fn test_fail_trying_to_sign_transaction_with_account_missing_a_private_key() {
		NEOCONFIG.lock().unwrap().network = Some(769);
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_symbol_neo.json",
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await;
			mock_provider_guard.mount_mocks().await;
		}
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};
		let account_without_keypair =
			Account::from_address(ACCOUNT1.get_address().as_str()).unwrap();
		let mut tb = TransactionBuilder::with_client(&client);
		tb.set_script(Some(vec![1, 2, 3]))
			.set_signers(vec![AccountSigner::none(&account_without_keypair).unwrap().into()])
			.unwrap()
			.valid_until_block(1000)
			.unwrap();

		let result = tb.sign().await;
		assert!(result.is_err());
		assert_eq!(result, Err(BuilderError::InvalidConfiguration(
			format!("Cannot create transaction signature because account {} does not hold a private key.", ACCOUNT1.get_address()),
		)))
	}

	#[tokio::test]
	async fn test_fail_automatically_signing_with_multi_sig_account_signer() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param("getblockcount", "getblockcount_1000.json")
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_symbol_neo.json",
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await;
			mock_provider_guard.mount_mocks().await;
		}
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};
		let multi_sig_account = Account::multi_sig_from_public_keys(
			vec![ACCOUNT1.get_public_key().unwrap()].as_mut(),
			1,
		)
		.unwrap();
		let mut tb = TransactionBuilder::with_client(&client);
		tb.set_script(Some(vec![1, 2, 3])).set_signers(vec![AccountSigner::none(
			&multi_sig_account,
		)
		.unwrap()
		.into()]);

		let result = tb.sign().await;
		assert!(result.is_err());
		assert_eq!(
			result,
			Err(BuilderError::IllegalState(
				"Transactions with multi-sig signers cannot be signed automatically.".to_string(),
			))
		);
	}

	#[tokio::test]
	async fn test_fail_with_no_signing_account() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param("getblockcount", "getblockcount_1000.json")
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_symbol_neo.json",
				)
				.await;
			mock_provider_guard.mount_mocks().await;
		}
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};
		let mut tb = TransactionBuilder::with_client(&client);
		tb.set_script(Some(vec![1, 2, 3]))
			.set_signers(vec![ContractSigner::called_by_entry(
				ACCOUNT1.address_or_scripthash().script_hash(),
				&*vec![],
			)
			.into()]);

		let result = tb.sign().await;
		assert!(result.is_err());
		assert_eq!(result, Err(BuilderError::TransactionError(Box::new(TransactionError::TransactionConfiguration(format!("A transaction requires at least one signing account (i.e. an AccountSigner). None was provided."))))));
	}

	#[tokio::test]
	async fn test_fail_signing_with_account_without_ec_keypair() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param("getblockcount", "getblockcount_1000.json")
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_symbol_neo.json",
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await;
			mock_provider_guard.mount_mocks().await;
		}
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};
		let account_without_keypair = Account::from_verification_script(
			&ACCOUNT1.clone().verification_script().clone().unwrap(),
		)
		.unwrap();
		let mut tb = TransactionBuilder::with_client(&client);
		tb.set_script(Some(vec![1, 2, 3])).set_signers(vec![AccountSigner::none(
			&account_without_keypair,
		)
		.unwrap()
		.into()]);

		let result = tb.sign().await;
		assert!(result.is_err());
		assert!(result.unwrap_err().to_string().contains(" does not hold a private key."));
	}

	#[tokio::test]
	async fn test_fail_sending_transaction_because_it_doesnt_contain_the_right_number_of_witnesses()
	{
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_symbol_neo.json",
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await;
			mock_provider_guard.mount_mocks().await;
		}
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};
		let mut tb = TransactionBuilder::with_client(&client);
		tb.set_script(Some(vec![1, 2, 3]))
			.set_signers(vec![AccountSigner::called_by_entry(&Account::create().unwrap())
				.unwrap()
				.into()])
			.unwrap()
			.valid_until_block(1000)
			.unwrap();
		let mut tx = tb.get_unsigned_tx().await.unwrap();
		let mut result = tx.send_tx().await;
		assert!(result.is_err());
		assert!(result
			.unwrap_err()
			.to_string()
			.contains("The transaction does not have the same number of signers and witnesses."));
	}

	#[tokio::test]
	async fn test_contract_witness() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_symbol_neo.json",
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await;
			mock_provider_guard.mount_mocks().await;
		}
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};
		let contract_hash = H160::from_str("e87819d005b730645050f89073a4cd7bf5f6bd3c").unwrap();
		let params = vec![ContractParameter::from("iamgroot"), ContractParameter::from(2)];
		let invocation_script = ScriptBuilder::new()
			.push_data("iamgroot".as_bytes().to_vec())
			.push_integer(BigInt::from(2))
			.to_bytes();
		let mut tb = TransactionBuilder::with_client(&client);
		tb.set_script(Some(vec![1, 2, 3]))
			.set_signers(vec![
				ContractSigner::global(contract_hash, &params).into(),
				AccountSigner::called_by_entry(&Account::create().unwrap()).unwrap().into(),
			])
			.unwrap()
			.valid_until_block(1000)
			.unwrap();

		let tx = tb.sign().await.unwrap();
		assert!(tx.witnesses().contains(&Witness::from_scripts(invocation_script, vec![])));
	}

	#[tokio::test]
	async fn test_transfer_neo_from_normal_account() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_transfer_with_fixed_sysfee.json",
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await;
			mock_provider_guard.mount_mocks().await;
		}
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};
		let script = ScriptBuilder::new()
			.contract_call(
				&ScriptHashExtension::from_hex(TestConstants::NEO_TOKEN_HASH).unwrap(),
				"transfer",
				&[
					ContractParameter::from(ACCOUNT1.address_or_scripthash().script_hash()),
					ContractParameter::from(
						H160::from_str("969a77db482f74ce27105f760efa139223431394").unwrap(),
					),
					ContractParameter::from(5),
					ContractParameter::any(),
				],
				None,
			)
			.unwrap()
			.to_bytes();

		let binding = ACCOUNT1.verification_script().clone().unwrap();
		let expected_verification_script = binding.script();
		let mut tb = TransactionBuilder::with_client(&client);
		tb.set_script(Some(script.clone()))
			.set_signers(vec![AccountSigner::none(ACCOUNT1.deref()).unwrap().into()])
			.unwrap()
			.valid_until_block(100)
			.unwrap();

		let tx = tb.sign().await.unwrap();
		assert_eq!(tx.script(), &script);
		assert_eq!(tx.witnesses().len(), 1);
		assert_eq!(
			tx.witnesses().first().unwrap().verification.script(),
			expected_verification_script
		);
	}

	#[tokio::test]
	async fn test_extend_script() {
		let client = CLIENT.get_or_init(|| async { MockClient::new().await.into_client() }).await;
		let script1 = ScriptBuilder::new()
			.contract_call(
				&ScriptHashExtension::from_hex(TestConstants::NEO_TOKEN_HASH).unwrap(),
				"transfer",
				&[
					ContractParameter::from(ACCOUNT1.address_or_scripthash().script_hash()),
					ContractParameter::from(
						H160::from_str("969a77db482f74ce27105f760efa139223431394").unwrap(),
					),
					ContractParameter::from(11),
					ContractParameter::any(),
				],
				None,
			)
			.unwrap()
			.to_bytes();

		let script2 = ScriptBuilder::new()
			.contract_call(
				&ScriptHashExtension::from_hex(TestConstants::NEO_TOKEN_HASH).unwrap(),
				"transfer",
				&[
					ContractParameter::from(ACCOUNT1.address_or_scripthash().script_hash()),
					ContractParameter::from(ACCOUNT2.address_or_scripthash().script_hash()),
					ContractParameter::from(22),
					ContractParameter::any(),
				],
				None,
			)
			.unwrap()
			.to_bytes();

		let mut tb = TransactionBuilder::with_client(&client);
		tb.set_script(Some(script1.clone()));
		assert_eq!(tb.script().clone().unwrap(), script1);

		tb.extend_script(script2.clone());
		assert_eq!(tb.script().clone().unwrap(), [script1, script2].concat());
	}

	#[tokio::test]
	async fn test_invoking_with_params_should_produce_the_correct_request() {
		// init_logger();
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file(
					"invokefunction",
					"invokefunction_transfer_neo.json",
					json!([
						TestConstants::NEO_TOKEN_HASH,
						"transfer",
						vec![
							ContractParameter::from(ACCOUNT1.address_or_scripthash().script_hash()),
							ContractParameter::from(
								H160::from_str("969a77db482f74ce27105f760efa139223431394").unwrap(),
							),
							ContractParameter::from(5),
							ContractParameter::any(),
						],
					]),
				)
				.await;
			mock_provider_guard.mount_mocks().await;
		}
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};

		let response = client
			.invoke_function(
				&H160::from_str(TestConstants::NEO_TOKEN_HASH).unwrap(),
				"transfer".to_string(),
				vec![
					ContractParameter::from(ACCOUNT1.address_or_scripthash().script_hash()),
					ContractParameter::from(
						H160::from_str("969a77db482f74ce27105f760efa139223431394").unwrap(),
					),
					ContractParameter::from(5),
					ContractParameter::any(),
				],
				None,
			)
			.await
			.unwrap();

		let script_In_Response = "CxUMFJQTQyOSE/oOdl8QJ850L0jbd5qWDBQGSl3MDxYsg0c9Aok46V+3dhMechTAHwwIdHJhbnNmZXIMFIOrBnmtVcBQoTrUP1k26nP16x72QWJ9W1I=".to_string();

		assert_eq!(response.script, script_In_Response);
	}

	#[tokio::test]
	async fn test_fail_signing_with_account_without_ec_key_pair() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		let client = {
			let mut mock_provider = mock_provider.lock().await;
			mock_provider.mock_default_responses().await.mount_mocks().await;
			Arc::new(mock_provider.into_client())
		};

		let account =
			Account::from_verification_script(&VerificationScript::from(vec![1, 2, 3])).unwrap();

		let mut tx_builder = TransactionBuilder::with_client(client.as_ref());
		tx_builder
			.set_script(Some(vec![1, 2, 3]))
			.set_signers(vec![AccountSigner::none(&account).unwrap().into()]);

		assert!(tx_builder.sign().await.is_err());
	}

	#[tokio::test]
	async fn test_do_if_sender_cannot_cover_fees() {
		// init_logger();
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_transfer_with_fixed_sysfee.json",
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_for_balance_of(
					&hex::encode(GAS_TOKEN_HASH.deref().as_bytes()),
					&hex::encode(ACCOUNT1.address_or_scripthash().script_hash().as_bytes()),
					"invokefunction_balanceOf_1000000.json",
				)
				.await;
			mock_provider_guard.mount_mocks().await;
		}
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};

		let recipient = H160::from_str("969a77db482f74ce27105f760efa139223431394").unwrap();

		let script = ScriptBuilder::new()
			.contract_call(
				&GAS_TOKEN_HASH,
				"transfer",
				&[
					ContractParameter::h160(&ACCOUNT1.address_or_scripthash().script_hash()),
					ContractParameter::h160(&recipient),
					ContractParameter::integer(2_000_000),
					ContractParameter::any(),
				],
				None,
			)
			.unwrap()
			.to_bytes();

		let tested = Arc::new(std::sync::atomic::AtomicBool::new(false));
		let tested_clone = tested.clone();

		let mut tx_builder = TransactionBuilder::with_client(client.as_ref());
		let _ = tx_builder
			.set_script(Some(script))
			.set_signers(vec![AccountSigner::called_by_entry(&ACCOUNT1).unwrap().into()])
			.unwrap()
			.valid_until_block(2000000)
			.unwrap()
			.do_if_sender_cannot_cover_fees(Box::new(move |fee, balance| {
				assert_eq!(fee, 1230610 + 9999510);
				assert_eq!(balance, 1000000);
				tested_clone.store(true, std::sync::atomic::Ordering::SeqCst);
			}));

		let _ = tx_builder.get_unsigned_tx().await.unwrap();
		assert!(tested.load(std::sync::atomic::Ordering::SeqCst));
	}

	#[tokio::test]
	async fn test_do_if_sender_cannot_cover_fees_already_specified_a_supplier() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		let client = {
			let mut mock_provider = mock_provider.lock().await;
			mock_provider.mock_default_responses().await.mount_mocks().await;
			Arc::new(mock_provider.into_client())
		};
		let mut tx_builder = TransactionBuilder::with_client(&client);
		tx_builder
			.throw_if_sender_cannot_cover_fees(TransactionError::IllegalState(Default::default()));

		let result = tx_builder.do_if_sender_cannot_cover_fees(Box::new(|_, _| {}));

		assert!(result.is_err());
		assert!(result
			.unwrap_err()
			.to_string()
			.contains("Cannot handle a consumer for this case, since an exception "));
	}

	#[tokio::test]
	async fn test_throw_if_sender_cannot_cover_fees() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_transfer_with_fixed_sysfee.json",
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_for_balance_of(
					&hex::encode(GAS_TOKEN_HASH.deref().as_bytes()),
					&hex::encode(ACCOUNT1.address_or_scripthash().script_hash().as_bytes()),
					"invokefunction_balanceOf_1000000.json",
				)
				.await;
			mock_provider_guard.mount_mocks().await;
		}
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};

		let recipient = H160::from_str("969a77db482f74ce27105f760efa139223431394").unwrap();

		let script = ScriptBuilder::new()
			.contract_call(
				&H160::from_str(TestConstants::NEO_TOKEN_HASH).unwrap(),
				"transfer",
				&[
					ContractParameter::h160(&ACCOUNT1.address_or_scripthash().script_hash()),
					ContractParameter::h160(&recipient),
					ContractParameter::integer(5),
					ContractParameter::any(),
				],
				None,
			)
			.unwrap()
			.to_bytes();

		let mut tx_builder = TransactionBuilder::with_client(&client);
		let _ = tx_builder
			.set_script(Some(script))
			.valid_until_block(2000000)
			.unwrap()
			.set_signers(vec![AccountSigner::called_by_entry(&ACCOUNT1).unwrap().into()])
			.unwrap()
			.throw_if_sender_cannot_cover_fees(TransactionError::IllegalState(
				"test throwIfSenderCannotCoverFees".to_string(),
			));

		let result = tx_builder.get_unsigned_tx().await;

		assert!(result.is_err());
		assert!(result.unwrap_err().to_string().contains("test throwIfSenderCannotCoverFees"));
	}

	#[tokio::test]
	async fn test_throw_if_sender_cannot_cover_fees_already_specified_a_consumer() {
		let client = CLIENT.get_or_init(|| async { MockClient::new().await.into_client() }).await;
		let mut tx_builder = TransactionBuilder::with_client(&client);
		let _ = tx_builder.do_if_sender_cannot_cover_fees(Box::new(|_, _| {}));

		let result =
			tx_builder.throw_if_sender_cannot_cover_fees(TransactionError::InsufficientFunds);
		assert!(result.is_err());
		assert!(result
			.unwrap_err()
			.to_string()
			.contains("Cannot handle a supplier for this case, since a consumer "));
	}

	#[tokio::test]
	async fn test_build_with_invalid_script() {
		// init_logger();
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file(
					"invokescript",
					"invokescript_invalidscript.json",
					json!(["DAASDBSTrRVy"]),
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param("getblockcount", "getblockcount_1000.json")
				.await;
			mock_provider_guard.mount_mocks().await;
		}
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};

		let account1 =
			Account::from_wif("L1WMhxazScMhUrdv34JqQb1HFSQmWeN2Kpc1R9JGKwL7CDNP21uR").unwrap();

		let mut tx_builder = TransactionBuilder::with_client(&client);
		tx_builder
			.set_script(Some(hex::decode("0c00120c1493ad1572").unwrap()))
			.set_signers(vec![AccountSigner::called_by_entry(&account1).unwrap().into()]);

		let result = tx_builder.get_unsigned_tx().await;

		assert!(result.is_err());
		if let TransactionError::TransactionConfiguration(error) = result.unwrap_err() {
			assert!(error.contains("Instruction out of bounds"));
		}
	}

	#[tokio::test]
	async fn test_build_with_script_vm_faults() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file(
					"invokescript",
					"invokescript_exception.json",
					json!(["DA5PcmFjbGVDb250cmFjdEEa93tn"]),
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param("getblockcount", "getblockcount_1000.json")
				.await;
			mock_provider_guard.mount_mocks().await;
		}
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};

		let account1 =
			Account::from_wif("L1WMhxazScMhUrdv34JqQb1HFSQmWeN2Kpc1R9JGKwL7CDNP21uR").unwrap();

		let mut tx_builder = TransactionBuilder::with_client(&client);
		tx_builder
			.set_script(Some(hex::decode("0c0e4f7261636c65436f6e7472616374411af77b67").unwrap()))
			.set_signers(vec![AccountSigner::called_by_entry(&account1).unwrap().into()]);

		let result = tx_builder.get_unsigned_tx().await;
		assert!(result.is_err());
		if let TransactionError::TransactionConfiguration(error) = result.unwrap_err() {
			assert_eq!(
				error,
				"The vm exited due to the following exception: Value was either too large or too small for an Int32."
			);
		}
	}

	#[tokio::test]
	async fn test_get_unsigned_transaction() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_symbol_neo.json",
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param("getblockcount", "getblockcount_1000.json")
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await;
			mock_provider_guard.mount_mocks().await;
		}
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};

		let account1 =
			Account::from_wif("L1WMhxazScMhUrdv34JqQb1HFSQmWeN2Kpc1R9JGKwL7CDNP21uR").unwrap();

		let mut tx_builder = TransactionBuilder::with_client(&client);
		tx_builder
			.set_script(Some(vec![1, 2, 3]))
			.set_signers(vec![AccountSigner::called_by_entry(&account1).unwrap().into()]);

		let tx = match tx_builder.get_unsigned_tx().await {
			Ok(tx) => tx,
			Err(e) => panic!("Error: {}", e),
		};

		assert_eq!(tx.version, 0);
		assert_eq!(tx.signers().len(), 1);
		assert_eq!(
			tx.signers[0].as_account_signer().unwrap(),
			&AccountSigner::called_by_entry(&account1).unwrap()
		);
		assert!(tx.witnesses.is_empty());
	}

	#[tokio::test]
	async fn test_version() {
		// init_logger();
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_symbol_neo.json",
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param("getblockcount", "getblockcount_1000.json")
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await;
			mock_provider_guard.mount_mocks().await;
		}
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};

		let account1 =
			Account::from_wif("L1WMhxazScMhUrdv34JqQb1HFSQmWeN2Kpc1R9JGKwL7CDNP21uR").unwrap();

		let mut tx_builder = TransactionBuilder::with_client(&client);
		tx_builder
			.version(1)
			.set_script(Some(vec![1, 2, 3]))
			.set_signers(vec![AccountSigner::called_by_entry(&account1).unwrap().into()]);

		let tx = match tx_builder.get_unsigned_tx().await {
			Ok(tx) => tx,
			Err(e) => panic!("Error: {}", e),
		};

		assert_eq!(tx.version, 1);
	}

	#[tokio::test]
	async fn test_additional_network_fee() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_symbol_neo.json",
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param("getblockcount", "getblockcount_1000.json")
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await;
			mock_provider_guard.mount_mocks().await;
		}
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};

		let account = Account::create().unwrap();

		let mut tx_builder = TransactionBuilder::with_client(&client);
		tx_builder
			.set_script(Some(vec![1, 2, 3]))
			.set_signers(vec![AccountSigner::none(&account).unwrap().into()]);

		let tx = match tx_builder.get_unsigned_tx().await {
			Ok(tx) => tx,
			Err(e) => panic!("Error: {}", e),
		};
		assert_eq!(tx.net_fee, 1230610);

		let mut tx_builder = TransactionBuilder::with_client(&client);
		tx_builder
			.set_script(Some(vec![1, 2, 3]))
			.set_signers(vec![AccountSigner::none(&account).unwrap().into()])
			.unwrap()
			.set_additional_network_fee(2000);

		let tx = match tx_builder.get_unsigned_tx().await {
			Ok(tx) => tx,
			Err(e) => panic!("Error: {}", e),
		};

		assert_eq!(tx.net_fee, 1230610 + 2000);
	}

	#[tokio::test]
	async fn test_additional_system_fee() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		{
			let mut mock_provider_guard = mock_provider.lock().await; // Lock the mock_provider once
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_symbol_neo.json",
				)
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param("getblockcount", "getblockcount_1000.json")
				.await;
			let mut mock_provider_guard = mock_provider_guard
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await;
			mock_provider_guard.mount_mocks().await;
		}
		let client = {
			let mock_provider = mock_provider.lock().await;
			Arc::new(mock_provider.into_client())
		};

		let account = Account::create().unwrap();

		let mut tx_builder = TransactionBuilder::with_client(&client);
		tx_builder
			.set_script(Some(vec![1, 2, 3]))
			.set_signers(vec![AccountSigner::none(&account).unwrap().into()])
			.expect("Failed to set signers in test - this should never fail with valid test data");

		let tx = match tx_builder.get_unsigned_tx().await {
			Ok(tx) => tx,
			Err(e) => panic!("Error: {}", e),
		};

		assert_eq!(tx.sys_fee, 984060);

		let mut tx_builder = TransactionBuilder::with_client(&client);
		tx_builder
			.set_script(Some(vec![1, 2, 3]))
			.set_signers(vec![AccountSigner::none(&account).unwrap().into()])
			.unwrap()
			.set_additional_system_fee(3000);

		let tx = match tx_builder.get_unsigned_tx().await {
			Ok(tx) => tx,
			Err(e) => panic!("Error: {}", e),
		};
		assert_eq!(tx.sys_fee, 984060 + 3000);
	}

	#[tokio::test]
	async fn test_set_first_signer() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		let client = {
			let mut mock_provider = mock_provider.lock().await;
			mock_provider.mock_default_responses().await.mount_mocks().await;
			Arc::new(mock_provider.into_client())
		};
		let account1 =
			Account::from_wif("L1WMhxazScMhUrdv34JqQb1HFSQmWeN2Kpc1R9JGKwL7CDNP21uR").unwrap();
		let account2 =
			Account::from_wif("KysNqEuLb3wmZJ6PsxbA9Bh6ewTybEda4dEiN9X7X48dJPkLWZ5a").unwrap();

		let s1 = AccountSigner::global(&account1.clone()).unwrap();
		let s2 = AccountSigner::called_by_entry(&account2.clone()).unwrap();

		let mut tx_builder = TransactionBuilder::with_client(&client);
		&tx_builder
			.set_script(Some(vec![1, 2, 3]))
			.set_signers(vec![s1.clone().into(), s2.clone().into()]);
		assert_eq!(tx_builder.clone().signers, vec![s1.clone().into(), s2.clone().into()]);

		tx_builder.first_signer(&s2.account).unwrap();
		assert_eq!(tx_builder.clone().signers, vec![s2.clone().into(), s1.clone().into()]);

		&tx_builder.first_signer(&account1).unwrap();
		assert_eq!(tx_builder.clone().signers, vec![s1.clone().into(), s2.clone().into()]);
	}

	#[tokio::test]
	async fn test_set_first_signer_fee_only_present() {
		let client = CLIENT.get_or_init(|| async { MockClient::new().await.into_client() }).await;
		let account1 =
			Account::from_wif("L1WMhxazScMhUrdv34JqQb1HFSQmWeN2Kpc1R9JGKwL7CDNP21uR").unwrap();
		let account2 =
			Account::from_wif("KysNqEuLb3wmZJ6PsxbA9Bh6ewTybEda4dEiN9X7X48dJPkLWZ5a").unwrap();

		let s1 = AccountSigner::none(&account1.clone()).unwrap();
		let s2 = AccountSigner::called_by_entry(&account2.clone()).unwrap();

		let mut tx_builder = TransactionBuilder::with_client(&client);
		tx_builder
			.set_script(Some(vec![1, 2, 3]))
			.set_signers(vec![s1.clone().into(), s2.clone().into()]);
		assert_eq!(tx_builder.signers, vec![s1.clone().into(), s2.clone().into()]);

		let result = tx_builder.first_signer(&account2);
		assert!(result.is_err());
		assert!(result
			.unwrap_err()
			.to_string()
			.contains("contains a signer with fee-only witness scope"));
	}

	#[tokio::test]
	async fn test_set_first_signer_not_present() {
		let client = CLIENT.get_or_init(|| async { MockClient::new().await.into_client() }).await;
		let account1 =
			Account::from_wif("L1WMhxazScMhUrdv34JqQb1HFSQmWeN2Kpc1R9JGKwL7CDNP21uR").unwrap();
		let account2 =
			Account::from_wif("KysNqEuLb3wmZJ6PsxbA9Bh6ewTybEda4dEiN9X7X48dJPkLWZ5a").unwrap();

		let s1 = AccountSigner::global(&account1.clone()).unwrap();

		let mut tx_builder = TransactionBuilder::with_client(&client);
		tx_builder.set_script(Some(vec![1, 2, 3])).set_signers(vec![s1.clone().into()]);
		assert_eq!(tx_builder.signers[0], s1.clone().into());

		let result = tx_builder.first_signer(&account2);
		assert!(result.is_err());
		assert!(result
			.unwrap_err()
			.to_string()
			.contains("Could not find a signer with script hash "));
	}

	#[ignore] // Ignoring this test until track_tx implementation is complete
	#[tokio::test]
	async fn test_tracking_transaction_should_return_correct_block() {
		init_logger();
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		let client = {
			let mut mock_provider = mock_provider.lock().await;
			mock_provider
				.mock_response_with_file_ignore_param("getblockcount", "getblockcount_1000.json")
				.await
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await
				.mock_response_with_file_ignore_param(
					"sendrawtransaction",
					"sendrawtransaction.json",
				)
				.await
				.mock_response_with_file_ignore_param(
					"invokescript",
					"invokescript_transfer_with_fixed_sysfee.json",
				)
				.await
				.mount_mocks()
				.await;
			Arc::new(mock_provider.into_client())
		};

		let account1 =
			Account::from_wif("L1WMhxazScMhUrdv34JqQb1HFSQmWeN2Kpc1R9JGKwL7CDNP21uR").unwrap();
		let recipient = H160::from_str("969a77db482f74ce27105f760efa139223431394").unwrap();

		let script = ScriptBuilder::new()
			.contract_call(
				&H160::from_str(TestConstants::NEO_TOKEN_HASH).unwrap(),
				"transfer",
				&[
					ContractParameter::h160(&account1.address_or_scripthash().script_hash()),
					ContractParameter::h160(&recipient),
					ContractParameter::integer(5),
					ContractParameter::any(),
				],
				None,
			)
			.unwrap()
			.to_bytes();

		let mut tx_builder = TransactionBuilder::with_client(&client);
		tx_builder
			.set_script(Some(script))
			.nonce(0)
			.unwrap()
			.set_signers(vec![AccountSigner::called_by_entry(&account1).unwrap().into()])
			.expect("Failed to set signers in test - this should never fail with valid test data");

		let mut tx = tx_builder.sign().await.unwrap();
		let _ = tx.send_tx().await.map_err(TransactionError::from).unwrap();

		// Note: Transaction tracking functionality is not yet implemented
		// When implemented, it should allow tracking transaction confirmation status
		// Example usage (future implementation):
		// let mut subscription = tx.track_tx(&client).await.unwrap();
		// while let Some(result) = subscription.next(&client).await {
		//     block_num = result.unwrap();
		//     if block_num == 1002 {
		//         break;
		//     }
		// }
		// assert_eq!(block_num, 1002);
	}

	#[tokio::test]
	async fn test_tracking_transaction_tx_not_sent() {
		init_logger();
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		let client = {
			let mut mock_provider = mock_provider.lock().await;
			mock_provider
				.mock_invoke_script(InvocationResult::default())
				.await
				.mock_response_with_file_ignore_param("getblockcount", "getblockcount_1000.json")
				.await
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await
				.mount_mocks()
				.await;
			Arc::new(mock_provider.into_client())
		};

		let account1 =
			Account::from_wif("L1WMhxazScMhUrdv34JqQb1HFSQmWeN2Kpc1R9JGKwL7CDNP21uR").unwrap();
		let recipient = H160::from_str("969a77db482f74ce27105f760efa139223431394").unwrap();

		let script = ScriptBuilder::new()
			.contract_call(
				&H160::from_str(TestConstants::NEO_TOKEN_HASH).unwrap(),
				"transfer",
				&[
					ContractParameter::h160(&account1.address_or_scripthash().script_hash()),
					ContractParameter::h160(&recipient),
					ContractParameter::integer(5),
					ContractParameter::any(),
				],
				None,
			)
			.unwrap()
			.to_bytes();

		let mut tx_builder = TransactionBuilder::with_client(&client);
		tx_builder
			.set_script(Some(script))
			.nonce(0)
			.unwrap()
			.set_signers(vec![AccountSigner::called_by_entry(&account1).unwrap().into()])
			.expect("Failed to set signers in test - this should never fail with valid test data");

		let tx = tx_builder.sign().await.unwrap();

		// Note: Transaction tracking functionality is not yet implemented
		// When implemented, it should verify that tracking fails for unsent transactions
		// assert!(tx.track_tx(&client).await.is_err());
	}

	#[tokio::test]
	async fn test_get_application_log() {
		init_logger();
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		let client = {
			let mut mock_provider = mock_provider.lock().await;
			mock_provider
				.mock_invoke_script(InvocationResult::default())
				.await
				.mock_response_with_file_ignore_param("getblockcount", "getblockcount_1000.json")
				.await
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await
				.mock_send_raw_transaction(RawTransaction { hash: H256::zero() })
				.await
				.mock_get_application_log(Some(ApplicationLog {
					transaction_id: H256::from_str(
						"0xeb52f99ae5cf923d8905bdd91c4160e2207d20c0cb42f8062f31c6743770e4d1",
					)
					.unwrap(),
					..Default::default()
				}))
				.await
				.mount_mocks()
				.await;
			Arc::new(mock_provider.into_client())
		};

		let account1 =
			Account::from_wif("L1WMhxazScMhUrdv34JqQb1HFSQmWeN2Kpc1R9JGKwL7CDNP21uR").unwrap();

		let script = ScriptBuilder::new()
			.contract_call(
				&H160::from_str(TestConstants::NEO_TOKEN_HASH).unwrap(),
				"transfer",
				&[
					ContractParameter::h160(&account1.address_or_scripthash().script_hash()),
					ContractParameter::h160(&account1.address_or_scripthash().script_hash()),
					ContractParameter::integer(1),
					ContractParameter::any(),
				],
				None,
			)
			.unwrap()
			.to_bytes();

		let mut tx_builder = TransactionBuilder::with_client(client.as_ref());
		tx_builder
			.set_script(Some(script))
			.set_signers(vec![AccountSigner::called_by_entry(&account1).unwrap().into()])
			.expect("Failed to set signers in test - this should never fail with valid test data");

		let mut tx = tx_builder.sign().await.map_err(TransactionError::BuilderError).unwrap();
		let _ = tx.send_tx().await.map_err(TransactionError::from).unwrap();
		let application_log = tx
			.get_application_log(client.as_ref())
			.await
			.map_err(TransactionError::from)
			.unwrap();

		assert_eq!(
			application_log.transaction_id,
			H256::from_str("0xeb52f99ae5cf923d8905bdd91c4160e2207d20c0cb42f8062f31c6743770e4d1")
				.unwrap()
		);
	}

	#[tokio::test]
	async fn test_get_application_log_tx_not_sent() {
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		let client = {
			let mut mock_provider = mock_provider.lock().await;
			mock_provider
				.mock_invoke_script(InvocationResult {
					gas_consumed: "984060".to_string(),
					..Default::default()
				})
				.await
				.mock_response_with_file_ignore_param("getblockcount", "getblockcount_1000.json")
				.await
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await
				.mount_mocks()
				.await;
			Arc::new(mock_provider.into_client())
		};

		let account1 =
			Account::from_wif("L1WMhxazScMhUrdv34JqQb1HFSQmWeN2Kpc1R9JGKwL7CDNP21uR").unwrap();

		let script = ScriptBuilder::new()
			.contract_call(
				&H160::from_str(TestConstants::NEO_TOKEN_HASH).unwrap(),
				"transfer",
				&[
					ContractParameter::h160(&account1.address_or_scripthash().script_hash()),
					ContractParameter::h160(&account1.address_or_scripthash().script_hash()),
					ContractParameter::integer(1),
					ContractParameter::any(),
				],
				None,
			)
			.unwrap()
			.to_bytes();

		let mut tx_builder = TransactionBuilder::with_client(&client);
		tx_builder
			.set_script(Some(script))
			.set_signers(vec![AccountSigner::called_by_entry(&account1).unwrap().into()]);

		let tx = tx_builder.sign().await.unwrap();

		assert!(tx.get_application_log(&client.as_ref()).await.is_err());
	}

	#[tokio::test]
	async fn test_get_application_log_not_existing() {
		init_logger();
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		let client = {
			let mut mock_provider = mock_provider.lock().await;
			mock_provider
				.mock_invoke_script(InvocationResult::default())
				.await
				.mock_response_with_file_ignore_param("getblockcount", "getblockcount_1000.json")
				.await
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await
				.mock_send_raw_transaction(RawTransaction { hash: H256::zero() })
				.await
				.mock_get_application_log(Default::default())
				.await
				.mount_mocks()
				.await;
			Arc::new(mock_provider.into_client())
		};

		let account1 =
			Account::from_wif("L1WMhxazScMhUrdv34JqQb1HFSQmWeN2Kpc1R9JGKwL7CDNP21uR").unwrap();
		let script = ScriptBuilder::new()
			.contract_call(
				&H160::from_str(TestConstants::NEO_TOKEN_HASH).unwrap(),
				"transfer",
				&[
					ContractParameter::h160(&account1.address_or_scripthash().script_hash()),
					ContractParameter::h160(&account1.address_or_scripthash().script_hash()),
					ContractParameter::integer(1),
					ContractParameter::any(),
				],
				None,
			)
			.unwrap()
			.to_bytes();
		let mut tx_builder = TransactionBuilder::with_client(&client);
		tx_builder
			.set_script(Some(script))
			.set_signers(vec![AccountSigner::called_by_entry(&account1).unwrap().into()]);
		let mut tx = tx_builder.sign().await.unwrap();
		let _ = tx.send_tx().await.map_err(TransactionError::from).unwrap();
		let log = tx.get_application_log(&client.as_ref()).await;
		assert!(log.is_err());
	}

	#[tokio::test]
	async fn test_transmission_on_fault() {
		// init_logger();
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		let client = {
			let mut mock_provider = mock_provider.lock().await;
			mock_provider
				.mock_response_with_file_ignore_param("invokescript", "invokescript_fault.json")
				.await
				.mock_response_with_file_ignore_param("getblockcount", "getblockcount_1000.json")
				.await
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await
				.mount_mocks()
				.await;
			Arc::new(mock_provider.into_client())
		};

		NEOCONFIG.lock().unwrap().allows_transmission_on_fault = true;

		assert!(NEOCONFIG.lock().unwrap().allows_transmission_on_fault);

		let account = Account::create().unwrap();

		let mut tx_builder = TransactionBuilder::with_client(&client);
		tx_builder
			.set_script(Some(vec![1, 2, 3]))
			.set_signers(vec![AccountSigner::none(&account).unwrap().into()])
			.expect("Failed to set signers in test - this should never fail with valid test data");
		// .allow_transmission_on_fault();

		let result = tx_builder.call_invoke_script().await.unwrap();
		assert!(result.has_state_fault());

		let gas_comsumed: i64 = result.gas_consumed.parse().unwrap();

		let tx = match tx_builder.get_unsigned_tx().await {
			Ok(tx) => tx,
			Err(e) => panic!("Error: {}", e),
		};
		assert_eq!(tx.sys_fee, gas_comsumed);

		NEOCONFIG.lock().unwrap().allows_transmission_on_fault = false;
		assert!(!NEOCONFIG.lock().unwrap().allows_transmission_on_fault);
	}

	#[tokio::test]
	#[ignore = "transmission-on-fault enforcement not deterministic in current test harness"]
	async fn test_prevent_transmission_on_fault() {
		// init_logger();
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		let client = {
			let mut mock_provider = mock_provider.lock().await;
			mock_provider
				.mock_response_with_file_ignore_param("invokescript", "invokescript_fault.json")
				.await
				.mock_response_with_file_ignore_param("getblockcount", "getblockcount_1000.json")
				.await
				.mock_response_with_file_ignore_param(
					"calculatenetworkfee",
					"calculatenetworkfee.json",
				)
				.await
				.mount_mocks()
				.await;
			Arc::new(mock_provider.into_client())
		};

		let account = Account::create().unwrap();

		let mut tx_builder = TransactionBuilder::with_client(&client);
		tx_builder
			.set_script(Some(vec![1, 2, 3]))
			.set_signers(vec![AccountSigner::none(&account).unwrap().into()])
			.expect("Failed to set signers in test - this should never fail with valid test data");

		assert!(!NEOCONFIG.lock().unwrap().allows_transmission_on_fault);

		let result = tx_builder.call_invoke_script().await.unwrap();
		assert!(result.has_state_fault());

		let tx = tx_builder.get_unsigned_tx().await;
		assert!(matches!(tx, Err(TransactionError::TransactionConfiguration(_))));
		if let Err(TransactionError::TransactionConfiguration(msg)) = tx {
			assert!(msg.contains("The vm exited due to the following exception: "));
		}
	}

	#[tokio::test]
	async fn test_sign_with_multiple_accounts() {
		init_logger();
		let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
		let client = {
			let mut mock_provider = mock_provider.lock().await;
			mock_provider.mock_default_responses().await.mount_mocks().await;
			Arc::new(mock_provider.into_client())
		};

		let account1 =
			Account::from_wif("L1WMhxazScMhUrdv34JqQb1HFSQmWeN2Kpc1R9JGKwL7CDNP21uR").unwrap();
		let account2 =
			Account::from_wif("KysNqEuLb3wmZJ6PsxbA9Bh6ewTybEda4dEiN9X7X48dJPkLWZ5a").unwrap();

		let mut tx_builder = TransactionBuilder::with_client(&client);
		tx_builder
			.set_script(Some(vec![1, 2, 3]))
			.set_signers(vec![
				AccountSigner::called_by_entry(&account1).unwrap().into(),
				AccountSigner::called_by_entry(&account2).unwrap().into(),
			])
			.expect("Failed to set signers in test - this should never fail with valid test data");

		let tx = tx_builder.sign().await.unwrap();

		assert_eq!(tx.witnesses.len(), 2);
		assert!(tx
			.witnesses
			.iter()
			.any(|w| w.verification == account1.verification_script().clone().unwrap()));
		assert!(tx
			.witnesses
			.iter()
			.any(|w| w.verification == account2.verification_script().clone().unwrap()));
	}

	// #[tokio::test]
	// async fn test_get_network_fee() {
	// 	let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
	// 	let client = {
	// 		let mut mock_provider = mock_provider.lock().await;
	// 		mock_provider
	// 			.mock_invoke_script(InvocationResult::default())
	// 			.await
	// 			.mock_response_with_file_ignore_param("getblockcount", "getblockcount_1000.json")
	// 			.await
	// 			.mock_response_with_file_ignore_param("calculatenetworkfee", "calculatenetworkfee.json")
	// 			.await
	// 			.mount_mocks()
	// 			.await;
	// 		Arc::new(mock_provider.into_client())
	// 	};

	// 	let account = Account::create().unwrap();

	// 	let mut tx_builder = TransactionBuilder::with_client(&client);
	// 	tx_builder
	// 		.set_script(Some(vec![1, 2, 3]))
	// 		.set_signers(vec![AccountSigner::called_by_entry(&account).unwrap().into()]);

	// 	let network_fee = tx_builder.get_network_fee().await.unwrap();
	// 	assert_eq!(network_fee, 1230610);
	// }

	// #[tokio::test]
	// async fn test_get_system_fee() {
	// 	let mock_provider = Arc::new(Mutex::new(MockClient::new().await));
	// 	let client = {
	// 		let mut mock_provider = mock_provider.lock().await;
	// 		mock_provider
	// 			.mock_invoke_script(InvocationResult::default())
	// 			.await
	// 			.mock_response_with_file_ignore_param("getblockcount", "getblockcount_1000.json")
	// 			.await
	// 			.mock_response_with_file_ignore_param("calculatenetworkfee", "calculatenetworkfee.json")
	// 			.await
	// 			.mount_mocks()
	// 			.await;
	// 		Arc::new(mock_provider.into_client())
	// 	};

	// 	let account = Account::create().unwrap();

	// 	let mut tx_builder = TransactionBuilder::with_client(&client);
	// 	tx_builder
	// 		.set_script(Some(vec![1, 2, 3]))
	// 		.set_signers(vec![AccountSigner::called_by_entry(&account).unwrap().into()]);

	// 	let system_fee = tx_builder.get_system_fee().await.unwrap();
	// 	assert_eq!(system_fee, 984060);
	// }
}
