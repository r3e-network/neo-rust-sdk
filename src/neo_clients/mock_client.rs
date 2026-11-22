use crate::{
	crypto::{KeyPair, Secp256r1PrivateKey},
	neo_clients::{MockProvider, RpcClient},
	neo_config::TestConstants,
	neo_protocol::{Account, AccountTrait, ApplicationLog, NeoVersion, RawTransaction},
	prelude::{ContractParameter, InvocationResult},
};
use lazy_static::lazy_static;
use primitive_types::H160;
use serde_json::{Value, json};
use std::{path::PathBuf, str::FromStr};

lazy_static! {
	pub static ref ACCOUNT1: Account = Account::from_key_pair(
		KeyPair::from_secret_key(
			&Secp256r1PrivateKey::from_bytes(
				&hex::decode("e6e919577dd7b8e97805151c05ae07ff4f752654d6d8797597aca989c02c4cb3")
					.unwrap()
			)
			.unwrap()
		),
		None,
		None
	)
	.expect("Failed to create ACCOUNT1");
	pub static ref ACCOUNT2: Account = Account::from_key_pair(
		KeyPair::from_secret_key(
			&Secp256r1PrivateKey::from_bytes(
				&hex::decode("b4b2b579cac270125259f08a5f414e9235817e7637b9a66cfeb3b77d90c8e7f9")
					.unwrap()
			)
			.unwrap()
		),
		None,
		None
	)
	.expect("Failed to create ACCOUNT2");
}

pub struct MockClient {
	provider: MockProvider,
}

impl MockClient {
	pub async fn new() -> Self {
		Self { provider: MockProvider::new() }
	}

	pub async fn mock_response(
		&mut self,
		method_name: &str,
		params: serde_json::Value,
		result: serde_json::Value,
	) {
		self.provider.push_result_with_params(method_name.to_string(), params, result);
	}

	pub async fn mock_response_error(&mut self, error: serde_json::Value) {
		let json_error: crate::neo_clients::JsonRpcError =
			serde_json::from_value(error).unwrap_or(crate::neo_clients::JsonRpcError {
				code: -32000,
				message: "mock error".to_string(),
				data: None,
			});
		self.provider.push_error_any(json_error);
	}

	pub async fn mock_response_ignore_param(
		&mut self,
		method_name: &str,
		result: serde_json::Value,
	) -> &mut Self {
		self.provider.push_result(method_name.to_string(), result);
		self
	}

	pub async fn mock_response_with_file(
		&mut self,
		method_name: &str,
		response_file: &str,
		params: serde_json::Value,
	) -> &mut Self {
		let response = load_jsonrpc_response(response_file).await;
		match response {
			JsonRpcFixture::Result(value) => {
				self.provider.push_result_with_partial_params(
					method_name.to_string(),
					params,
					value,
				);
			},
			JsonRpcFixture::Error(error) => {
				self.provider.push_error(method_name.to_string(), error);
			},
		}
		self
	}

	pub async fn mock_response_with_file_ignore_param(
		&mut self,
		method_name: &str,
		response_file: &str,
	) -> &mut Self {
		let response = load_jsonrpc_response(response_file).await;
		match response {
			JsonRpcFixture::Result(value) => {
				self.provider.push_result(method_name.to_string(), value);
			},
			JsonRpcFixture::Error(error) => {
				self.provider.push_error(method_name.to_string(), error);
			},
		}
		self
	}

	pub async fn mock_response_for_balance_of(
		&mut self,
		contract_hash: &str,
		account_script_hash: &str,
		response_file: &str,
	) -> &mut Self {
		let response = load_jsonrpc_response(response_file).await;
		let expected_params = json!([
			contract_hash,
			"balanceOf",
			[
				{
					"type": "Hash160",
					"value": account_script_hash,
				}
			]
		]);

		match response {
			JsonRpcFixture::Result(value) => {
				self.provider.push_result_with_partial_params(
					"invokefunction".to_string(),
					expected_params,
					value,
				);
			},
			JsonRpcFixture::Error(error) => {
				self.provider.push_error("invokefunction".to_string(), error);
			},
		}
		self
	}

	pub async fn mock_default_responses(&mut self) -> &mut Self {
		self.mock_response_with_file_ignore_param(
			"invokescript",
			"invokescript_necessary_mock.json",
		)
		.await;
		self.mock_response_with_file(
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
		self.mock_response_with_file_ignore_param("getblockcount", "getblockcount_1000.json")
			.await;
		self.mock_response_with_file_ignore_param(
			"calculatenetworkfee",
			"calculatenetworkfee.json",
		)
		.await;
		self.mock_response_ignore_param("getversion", json!({})).await;
		self
	}

	pub async fn mock_invoke_script(&mut self, result: InvocationResult) -> &mut Self {
		self.mock_response_ignore_param("invokescript", serde_json::to_value(result).unwrap())
			.await;
		self
	}

	// pub async fn mock_get_block_count(&mut self, result: i32) -> &mut Self {
	// 	self.mock_response_ignore_param("getblockcount", json!(Ok::<i32, ()>(result)))
	// 		.await;
	// 	self
	// }

	pub async fn mock_get_block_count(&mut self, block_count: u32) -> &mut Self {
		let response = load_jsonrpc_response(&format!("getblockcount_{}.json", block_count)).await;
		match response {
			JsonRpcFixture::Result(value) => self.provider.push_result("getblockcount", value),
			JsonRpcFixture::Error(error) => self.provider.push_error("getblockcount", error),
		}
		self
	}

	// pub async fn mock_calculate_network_fee(&mut self, result: i32) -> &mut Self {
	// 	self.mock_response_ignore_param("calculatenetworkfee", json!(Ok::<i32, ()>(result)))
	// 		.await;
	// 	self
	// }

	pub async fn mock_send_raw_transaction(&mut self, _result: RawTransaction) -> &mut Self {
		self.mock_response_with_file_ignore_param("sendrawtransaction", "sendrawtransaction.json")
			.await;
		self
	}

	pub async fn mock_get_version(&mut self, result: NeoVersion) -> &mut Self {
		self.mock_response_ignore_param("getversion", serde_json::to_value(result).unwrap())
			.await;
		self
	}

	pub async fn mock_invoke_function(&mut self, result: InvocationResult) -> &mut Self {
		self.mock_response_ignore_param("invokefunction", serde_json::to_value(result).unwrap())
			.await;
		self
	}

	pub async fn mock_get_application_log(&mut self, result: Option<ApplicationLog>) -> &mut Self {
		self.mock_response_ignore_param("getapplicationlog", serde_json::to_value(result).unwrap())
			.await;
		self
	}

	pub async fn mount_mocks(&mut self) -> &mut Self {
		// In-memory provider does not require an explicit "mount" step.
		self
	}

	pub fn into_client(&self) -> RpcClient<MockProvider> {
		RpcClient::new(self.provider.clone())
	}
}

enum JsonRpcFixture {
	Result(Value),
	Error(crate::neo_clients::JsonRpcError),
}

async fn load_jsonrpc_response(response_file: &str) -> JsonRpcFixture {
	let mut response_file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	response_file_path.push("test_resources");
	response_file_path.push("responses");
	response_file_path.push(response_file);

	let response_body = tokio::fs::read_to_string(response_file_path)
		.await
		.unwrap_or_else(|e| panic!("Failed to read response file {response_file}: {e}"));

	let parsed: Value = serde_json::from_str(&response_body)
		.unwrap_or_else(|e| panic!("Failed to parse JSON response file {response_file}: {e}"));

	if let Some(error) = parsed.get("error") {
		let json_error: crate::neo_clients::JsonRpcError = serde_json::from_value(error.clone())
			.unwrap_or(crate::neo_clients::JsonRpcError {
				code: -32000,
				message: "mock error".to_string(),
				data: None,
			});
		return JsonRpcFixture::Error(json_error);
	}

	let Some(result) = parsed.get("result") else {
		panic!("Mock response file {response_file} missing 'result' or 'error' field");
	};

	JsonRpcFixture::Result(result.clone())
}
