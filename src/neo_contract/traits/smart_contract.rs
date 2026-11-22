use std::sync::Arc;

use crate::neo_crypto::utils::ToHexString;
use async_trait::async_trait;
use num_bigint::BigInt;
use primitive_types::H160;

// Replace prelude imports with specific types
use crate::{
	ScriptHashExtension,
	neo_builder::{CallFlags, ScriptBuilder},
	neo_clients::{APITrait, JsonRpcProvider, RpcClient},
	neo_contract::{ContractError, NeoIterator},
	neo_types::{
		Bytes, ContractManifest, ContractParameter, InvocationResult, OpCode, ScriptHash, StackItem,
	},
};

// Import transaction types from the correct modules
use crate::neo_builder::{Signer, TransactionBuilder};

#[async_trait]
pub trait SmartContractTrait<'a>: Send + Sync {
	const DEFAULT_ITERATOR_COUNT: usize = 100;
	type P: JsonRpcProvider;

	async fn try_name(&self) -> Result<String, ContractError> {
		self.try_get_manifest().await?.name.ok_or_else(|| {
			ContractError::InvalidResponse("Contract manifest is missing name".to_string())
		})
	}

	async fn name(&self) -> String {
		self.try_name().await.unwrap_or_else(|err| {
			let fallback = self.script_hash().to_hex();
			tracing::warn!(
				error = %err,
				fallback = %fallback,
				"Failed to resolve contract name; returning contract hash"
			);
			fallback
		})
	}
	/// Attempts to set the contract name. Ignored by default since most
	/// contract wrappers derive their name from the on-chain manifest.
	/// Override in implementations that support mutable names.
	fn set_name(&mut self, _name: String) {
		tracing::debug!("set_name ignored for this contract type");
	}

	fn script_hash(&self) -> H160;

	/// Attempts to set the script hash. Ignored by default since most
	/// contract wrappers are bound to a fixed script hash at construction.
	/// Override in implementations that support retargeting.
	fn set_script_hash(&mut self, _script_hash: H160) {
		tracing::debug!("set_script_hash ignored for this contract type");
	}

	fn provider(&self) -> Option<&RpcClient<Self::P>>;

	async fn invoke_function(
		&self,
		function: &str,
		params: Vec<ContractParameter>,
	) -> Result<TransactionBuilder<Self::P>, ContractError> {
		let script = self.build_invoke_function_script(function, params).await?;
		let mut builder = TransactionBuilder::new();
		builder.set_script(Some(script));
		Ok(builder)
	}

	async fn build_invoke_function_script(
		&self,
		function: &str,
		params: Vec<ContractParameter>,
	) -> Result<Bytes, ContractError> {
		if function.is_empty() {
			return Err(ContractError::InvalidNeoName("Function name cannot be empty".to_string()));
		}

		let script = ScriptBuilder::new()
			.contract_call(&self.script_hash(), function, params.as_slice(), Some(CallFlags::None))
			.map_err(|e| {
				ContractError::RuntimeError(format!("Failed to build contract call: {e}"))
			})?
			.to_bytes();

		Ok(script)
	}

	async fn call_function_returning_string(
		&self,
		function: &str,
		params: Vec<ContractParameter>,
	) -> Result<String, ContractError> {
		let output = self.call_invoke_function(function, params, vec![]).await?;
		self.throw_if_fault_state(&output)?;

		let item = output
			.get_first_stack_item()
			.map_err(|e| ContractError::InvalidResponse(e.to_string()))?;
		match item.as_string() {
			Some(s) => Ok(s),
			None => Err(ContractError::UnexpectedReturnType("String".to_string())),
		}
	}

	async fn call_function_returning_int(
		&self,
		function: &str,
		params: Vec<ContractParameter>,
	) -> Result<i64, ContractError> {
		let output = self.call_invoke_function(function, params, vec![]).await?;
		self.throw_if_fault_state(&output)?;

		let item = output
			.get_first_stack_item()
			.map_err(|e| ContractError::InvalidResponse(e.to_string()))?;
		match item.as_int() {
			Some(i) => Ok(i),
			None => Err(ContractError::UnexpectedReturnType("Int".to_string())),
		}
	}

	async fn call_function_returning_bool(
		&self,
		function: &str,
		params: Vec<ContractParameter>,
	) -> Result<bool, ContractError> {
		let output = self.call_invoke_function(function, params, vec![]).await?;
		self.throw_if_fault_state(&output)?;

		let item = output
			.get_first_stack_item()
			.map_err(|e| ContractError::InvalidResponse(e.to_string()))?;
		match item.as_bool() {
			Some(b) => Ok(b),
			None => Err(ContractError::UnexpectedReturnType("Bool".to_string())),
		}
	}

	// Other methods

	async fn call_invoke_function(
		&self,
		function: &str,
		params: Vec<ContractParameter>,
		signers: Vec<Signer>,
	) -> Result<InvocationResult, ContractError> {
		if function.is_empty() {
			return Err(ContractError::InvalidNeoName("Function cannot be empty".to_string()));
		}

		let provider = self.provider().ok_or_else(|| {
			ContractError::ProviderNotSet(
				"Provider is required for contract invocations".to_string(),
			)
		})?;

		provider
			.invoke_function(&self.script_hash(), function.into(), params, Some(signers))
			.await
			.map_err(ContractError::from)
	}

	fn throw_if_fault_state(&self, output: &InvocationResult) -> Result<(), ContractError> {
		if output.has_state_fault() {
			let message =
				output.exception.clone().unwrap_or_else(|| "Invocation faulted".to_string());
			Err(ContractError::InvocationFailed(message))
		} else {
			Ok(())
		}
	}

	// Other methods for different return types
	async fn call_function_returning_script_hash(
		&self,
		function: &str,
		params: Vec<ContractParameter>,
	) -> Result<H160, ContractError> {
		let output = self.call_invoke_function(function, params, vec![]).await?;
		self.throw_if_fault_state(&output)?;

		let item = output
			.get_first_stack_item()
			.map_err(|e| ContractError::InvalidResponse(e.to_string()))?;

		let bytes = item
			.as_bytes()
			.ok_or_else(|| ContractError::UnexpectedReturnType("ByteString".to_string()))?;

		if bytes.len() != 20 {
			return Err(ContractError::InvalidResponse(format!(
				"Expected 20 bytes for ScriptHash, got {}",
				bytes.len()
			)));
		}

		Ok(H160::from_slice(&bytes))
	}

	async fn call_function_returning_iterator<U>(
		&self,
		function: &str,
		params: Vec<ContractParameter>,
		mapper: Arc<dyn Fn(StackItem) -> Result<U, ContractError> + Send + Sync>,
	) -> Result<NeoIterator<U, Self::P>, ContractError>
	where
		U: Send + Sync, // Adding this bound if necessary
	{
		let output = self.call_invoke_function(function, params, vec![]).await?;
		self.throw_if_fault_state(&output)?;

		let session_id = output.session_id.clone().ok_or_else(|| {
			ContractError::InvalidResponse(
				"No session ID returned from iterator invocation".to_string(),
			)
		})?;

		let item = output
			.get_first_stack_item()
			.map_err(|e| ContractError::InvalidResponse(e.to_string()))?;
		let StackItem::InteropInterface { id, interface: _ } = item else {
			return Err(ContractError::UnexpectedReturnType(format!(
				"Expected InteropInterface, got {:?}",
				item
			)));
		};

		let provider = self.provider().ok_or_else(|| {
			ContractError::ProviderNotSet("Provider is required for iterator traversal".to_string())
		})?;

		Ok(NeoIterator::new(session_id, id.clone(), mapper, Some(provider)))
	}

	async fn call_function_and_unwrap_iterator<U>(
		&self,
		function: &str,
		params: Vec<ContractParameter>,
		_max_items: usize,
		mapper: impl Fn(StackItem) -> Result<U, ContractError> + Send,
	) -> Result<Vec<U>, ContractError> {
		let max_items = u32::try_from(_max_items).map_err(|_| {
			ContractError::InvalidArgError(format!(
				"Iterator item limit {_max_items} exceeds the supported u32 range"
			))
		})?;
		let script = ScriptBuilder::build_contract_call_and_unwrap_iterator(
			&self.script_hash(),
			function,
			&params,
			max_items,
			Some(CallFlags::All),
		)
		.map_err(|e| {
			ContractError::RuntimeError(format!("Failed to build iterator script: {e}"))
		})?;

		let provider = self.provider().ok_or_else(|| {
			ContractError::ProviderNotSet(
				"Provider is required for contract invocations".to_string(),
			)
		})?;

		let output = provider.invoke_script(script.to_hex_string(), vec![]).await?;

		self.throw_if_fault_state(&output)?;

		let stack_item = output
			.get_first_stack_item()
			.map_err(|e| ContractError::InvalidResponse(e.to_string()))?;

		let array = stack_item
			.as_array()
			.ok_or_else(|| ContractError::UnexpectedReturnType("Array".to_string()))?;

		let items = array.into_iter().map(mapper).collect::<Result<Vec<_>, _>>()?;

		Ok(items)
	}

	fn calc_native_contract_hash(contract_name: &str) -> Result<H160, ContractError> {
		Self::calc_contract_hash(H160::zero(), 0, contract_name)
	}

	/// Calculates a native contract hash, panicking on failure.
	///
	/// This is intended for use with known-good constants like `NeoToken`, `GasToken`, etc.,
	/// where the only failure mode would be an empty name (a programming error).
	fn calc_native_contract_hash_unchecked(contract_name: &str) -> H160 {
		Self::calc_native_contract_hash(contract_name).unwrap_or_else(|e| {
			panic!("BUG: failed to compute native contract hash for '{}': {}", contract_name, e)
		})
	}

	fn calc_contract_hash(
		sender: H160,
		nef_checksum: u32,
		contract_name: &str,
	) -> Result<H160, ContractError> {
		if contract_name.is_empty() {
			return Err(ContractError::InvalidNeoName("Contract name cannot be empty".to_string()));
		}

		let mut script = ScriptBuilder::new();
		script
			.op_code(&[OpCode::Abort])
			.push_data(sender.to_vec())
			.push_integer(BigInt::from(nef_checksum))
			.push_data(contract_name.as_bytes().to_vec());

		Ok(ScriptHash::from_script(&script.to_bytes()))
	}

	async fn try_get_manifest(&self) -> Result<ContractManifest, ContractError> {
		let provider = self.provider().ok_or_else(|| {
			ContractError::ProviderNotSet(
				"Provider is required to fetch contract manifest".to_string(),
			)
		})?;

		let state = provider.get_contract_state(self.script_hash()).await?;
		Ok(state.manifest)
	}

	async fn get_manifest(&self) -> ContractManifest {
		self.try_get_manifest().await.unwrap_or_else(|err| {
			tracing::warn!(error = %err, "Failed to fetch contract manifest; returning default");
			ContractManifest::default()
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		neo_clients::{MockProvider, RpcClient},
		neo_contract::ContractError,
		neo_types::{ContractManifest, ContractNef, ContractState},
	};
	use async_trait::async_trait;
	use primitive_types::H160;
	use serde_json::json;

	#[derive(Clone)]
	struct TestContract {
		script_hash: H160,
		provider: Option<RpcClient<MockProvider>>,
	}

	impl TestContract {
		fn without_provider(script_hash: H160) -> Self {
			Self { script_hash, provider: None }
		}

		fn with_provider(script_hash: H160, provider: RpcClient<MockProvider>) -> Self {
			Self { script_hash, provider: Some(provider) }
		}
	}

	#[async_trait]
	impl<'a> SmartContractTrait<'a> for TestContract {
		type P = MockProvider;

		fn script_hash(&self) -> H160 {
			self.script_hash
		}

		fn provider(&self) -> Option<&RpcClient<Self::P>> {
			self.provider.as_ref()
		}
	}

	fn test_manifest(name: &str) -> ContractManifest {
		ContractManifest::new(
			Some(name.to_string()),
			vec![],
			None,
			vec![],
			None,
			vec![],
			vec![],
			None,
		)
	}

	fn test_contract_state(hash: H160, manifest: ContractManifest) -> ContractState {
		ContractState::new(1, 0, hash, ContractNef::default(), manifest)
	}

	#[tokio::test]
	async fn test_try_get_manifest_returns_provider_not_set_without_provider() {
		let contract = TestContract::without_provider(H160::repeat_byte(0x11));

		assert!(matches!(
			contract.try_get_manifest().await,
			Err(ContractError::ProviderNotSet(message))
				if message.contains("contract manifest")
		));
	}

	#[tokio::test]
	async fn test_try_get_manifest_returns_manifest_from_provider() {
		let hash = H160::repeat_byte(0x22);
		let manifest = test_manifest("TestContract");
		let provider = MockProvider::new();
		provider.push_result_with_params(
			"getcontractstate",
			json!([hash.to_hex()]),
			serde_json::to_value(test_contract_state(hash, manifest.clone())).unwrap(),
		);
		let contract = TestContract::with_provider(hash, RpcClient::new(provider));

		let fetched = contract.try_get_manifest().await.unwrap();
		assert_eq!(fetched.name.as_deref(), Some("TestContract"));
	}

	#[tokio::test]
	async fn test_try_name_returns_manifest_name() {
		let hash = H160::repeat_byte(0x33);
		let manifest = test_manifest("FriendlyName");
		let provider = MockProvider::new();
		provider.push_result_with_params(
			"getcontractstate",
			json!([hash.to_hex()]),
			serde_json::to_value(test_contract_state(hash, manifest)).unwrap(),
		);
		let contract = TestContract::with_provider(hash, RpcClient::new(provider));

		assert_eq!(contract.try_name().await.unwrap(), "FriendlyName");
	}

	#[tokio::test]
	async fn test_try_name_rejects_missing_manifest_name() {
		let hash = H160::repeat_byte(0x34);
		let manifest = test_manifest("");
		let provider = MockProvider::new();
		provider.push_result_with_params(
			"getcontractstate",
			json!([hash.to_hex()]),
			serde_json::to_value(test_contract_state(
				hash,
				ContractManifest { name: None, ..manifest },
			))
			.unwrap(),
		);
		let contract = TestContract::with_provider(hash, RpcClient::new(provider));

		assert!(matches!(
			contract.try_name().await,
			Err(ContractError::InvalidResponse(message))
				if message.contains("missing name")
		));
	}

	#[tokio::test]
	async fn test_name_returns_contract_hash_when_manifest_name_missing() {
		let hash = H160::repeat_byte(0x35);
		let provider = MockProvider::new();
		provider.push_result_with_params(
			"getcontractstate",
			json!([hash.to_hex()]),
			serde_json::to_value(test_contract_state(hash, ContractManifest::default())).unwrap(),
		);
		let contract = TestContract::with_provider(hash, RpcClient::new(provider));

		assert_eq!(contract.name().await, hash.to_hex());
	}

	#[tokio::test]
	async fn test_get_manifest_returns_default_without_provider() {
		let contract = TestContract::without_provider(H160::repeat_byte(0x44));
		assert_eq!(contract.get_manifest().await.name, None);
	}

	#[tokio::test]
	async fn test_name_returns_contract_hash_without_provider() {
		let hash = H160::repeat_byte(0x55);
		let contract = TestContract::without_provider(hash);
		assert_eq!(contract.name().await, hash.to_hex());
	}
}
