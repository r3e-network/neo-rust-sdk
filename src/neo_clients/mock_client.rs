use crate::{
	crypto::{KeyPair, Secp256r1PrivateKey},
	neo_clients::{HttpProvider, RpcClient},
	neo_config::TestConstants,
	neo_protocol::{Account, AccountTrait, ApplicationLog, NeoVersion, RawTransaction},
};
use lazy_static::lazy_static;
use neo3::prelude::*;
use primitive_types::H160;
use serde_json::json;
use std::{path::PathBuf, str::FromStr};
use url::Url;
use wiremock::{
	matchers::{body_json, body_partial_json, method, path}, Mock, MockServer, ResponseTemplate,
};

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
	server: MockServer,
	mocks: Vec<Mock>,
}

impl MockClient {
	pub async fn new() -> Self {
		let server = MockServer::start().await;
		Self { server, mocks: Vec::new() }
	}

	pub async fn mock_response(
		&mut self,
		method_name: &str,
		params: serde_json::Value,
		result: serde_json::Value,
	) {
		let mock = Mock::given(method("POST"))
			.and(path("/"))
			.and(body_json(json!({
				"jsonrpc": "2.0",
				"method": method_name,
				"params": params,
				"id": 1
			})))
			.respond_with(ResponseTemplate::new(200).set_body_json(json!({
				"jsonrpc": "2.0",
				"id": 1,
				"result": result
			})));
		self.mocks.push(mock);
	}

	pub async fn mock_response_error(&mut self, error: serde_json::Value) {
		let mock = Mock::given(method("POST")).and(path("/")).respond_with(
			ResponseTemplate::new(200).set_body_json(json!({
				"jsonrpc": "2.0",
				"id": 1,
				"error": error
			})),
		);
		self.mocks.push(mock);
	}

	pub async fn mock_response_ignore_param(
		&mut self,
		method_name: &str,
		result: serde_json::Value,
	) -> &mut Self {
		let mock = Mock::given(method("POST"))
			.and(path("/"))
			.and(body_partial_json(json!({
				"jsonrpc": "2.0",
				"method": method_name,
			})))
			.respond_with(ResponseTemplate::new(200).set_body_json(json!({
				"jsonrpc": "2.0",
				"id": 1,
				"result": result
			})));
		self.mocks.push(mock);
		self
	}

	pub async fn mock_response_with_file(
		&mut self,
		method_name: &str,
		response_file: &str,
		params: serde_json::Value,
	) -> &mut Self {
		// Construct the path to the response file relative to the project root
		let mut response_file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
		response_file_path.push("test_resources");
		response_file_path.push("responses");
		response_file_path.push(response_file);

		// Load the response body from the specified file
		let response_body = tokio::fs::read_to_string(response_file_path)
			.await
			.expect("Failed to read response file");

		let mock = Mock::given(method("POST"))
			.and(path("/"))
			.and(body_partial_json(json!({
				"jsonrpc": "2.0",
				"method": method_name,
				"params": params,
			})))
			.respond_with(ResponseTemplate::new(200).set_body_string(response_body));
		self.mocks.push(mock);
		self
	}

	pub async fn mock_response_with_file_ignore_param(
		&mut self,
		method_name: &str,
		response_file: &str,
	) -> &mut Self {
		// Construct the path to the response file relative to the project root
		let mut response_file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
		response_file_path.push("test_resources");
		response_file_path.push("responses");
		response_file_path.push(response_file);

		// Load the response body from the specified file
		let response_body = tokio::fs::read_to_string(response_file_path)
			.await
			.expect("Failed to read response file");
		// // Load the response body from the specified file
		// let response_body = tokio::fs::read_to_string(format!("/responses/{}", response_file))
		// .await
		// .expect("Failed to read response file");

		let mock = Mock::given(method("POST"))
			.and(path("/"))
			.and(body_partial_json(json!({
				"jsonrpc": "2.0",
				"method": method_name,
			})))
			.respond_with(ResponseTemplate::new(200).set_body_string(response_body));
		self.mocks.push(mock);
		self
	}

	pub async fn mock_response_for_balance_of(
		&mut self,
		contract_hash: &str,
		account_script_hash: &str,
		response_file: &str,
	) -> &mut Self {
		// Construct the path to the response file relative to the project root
		let mut response_file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
		response_file_path.push("test_resources");
		response_file_path.push("responses");
		response_file_path.push(response_file);

		// Load the response body from the specified file
		let response_body = tokio::fs::read_to_string(response_file_path)
			.await
			.expect("Failed to read response file");

		let mock = Mock::given(method("POST"))
			.and(path("/"))
			.and(body_partial_json(json!({
				"jsonrpc": "2.0",
				"method": "invokefunction",
				"params": [
					contract_hash,
					"balanceOf",
					[
						{
							"type": "Hash160",
							"value": account_script_hash,
						}
					]
				],
			})))
			.respond_with(ResponseTemplate::new(200).set_body_string(response_body));

		self.mocks.push(mock);
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
		self
	}

	pub async fn mock_invoke_script(&mut self, result: InvocationResult) -> &mut Self {
		self.mock_response_ignore_param("invokescript", json!(Ok::<InvocationResult, ()>(result)))
			.await;
		self
	}

	// pub async fn mock_get_block_count(&mut self, result: i32) -> &mut Self {
	// 	self.mock_response_ignore_param("getblockcount", json!(Ok::<i32, ()>(result)))
	// 		.await;
	// 	self
	// }

	pub async fn mock_get_block_count(&mut self, block_count: u32) -> &mut Self {
		// Construct the path to the response file relative to the project root
		let mut response_file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
		response_file_path.push("test_resources");
		response_file_path.push("responses");
		response_file_path.push(format!("getblockcount_{}.json", block_count));

		// Load the response body from the specified file
		let response_body = tokio::fs::read_to_string(response_file_path)
			.await
			.expect("Failed to read response file");
		// // Load the response body from the specified file
		// let response_body = tokio::fs::read_to_string(format!("/responses/{}", response_file))
		// .await
		// .expect("Failed to read response file");

		let mock = Mock::given(method("POST"))
			.and(path("/"))
			.and(body_partial_json(json!({
				"jsonrpc": "2.0",
				"method": "getblockcount",
			})))
			.respond_with(ResponseTemplate::new(200).set_body_string(response_body));
		self.mocks.push(mock);
		self
	}

	// pub async fn mock_calculate_network_fee(&mut self, result: i32) -> &mut Self {
	// 	self.mock_response_ignore_param("calculatenetworkfee", json!(Ok::<i32, ()>(result)))
	// 		.await;
	// 	self
	// }

	pub async fn mock_send_raw_transaction(&mut self, result: RawTransaction) -> &mut Self {
		self.mock_response_with_file_ignore_param("sendrawtransaction", "sendrawtransaction.json")
			.await;
		self
	}

	pub async fn mock_get_version(&mut self, result: NeoVersion) -> &mut Self {
		self.mock_response_ignore_param("getversion", json!(Ok::<NeoVersion, ()>(result)))
			.await;
		self
	}

	pub async fn mock_invoke_function(&mut self, result: InvocationResult) -> &mut Self {
		self.mock_response_ignore_param(
			"invokefunction",
			json!(Ok::<InvocationResult, ()>(result)),
		)
		.await;
		self
	}

	pub async fn mock_get_application_log(&mut self, result: Option<ApplicationLog>) -> &mut Self {
		self.mock_response_ignore_param("getapplicationlog", json!(result)).await;
		self
	}

	pub async fn mount_mocks(&mut self) -> &mut Self {
		for mock in self.mocks.drain(..) {
			mock.mount(&self.server).await;
		}
		self
	}

	pub fn url(&self) -> Url {
		Url::parse(&self.server.uri()).expect("Invalid mock server URL")
	}

	pub fn into_client(&self) -> RpcClient<HttpProvider> {
		let http_provider = HttpProvider::new(self.url()).expect("Failed to create HTTP provider");
		RpcClient::new(http_provider)
	}

	pub fn server(&self) -> &MockServer {
		&self.server
	}
}
