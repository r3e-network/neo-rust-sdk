use std::sync::Arc;

use crate::neo_crypto::utils::ToHexString;
use async_trait::async_trait;
use num_bigint::BigInt;
use primitive_types::H160;

// Replace prelude imports with specific types
use crate::{
	neo_builder::{CallFlags, ScriptBuilder},
	neo_clients::{APITrait, JsonRpcProvider, RpcClient},
	neo_contract::{ContractError, NeoIterator},
	neo_types::{
		Bytes, ContractManifest, ContractParameter, InvocationResult, OpCode, ScriptHash, StackItem,
	},
	ScriptHashExtension,
};

// Import transaction types from the correct modules
use crate::neo_builder::{Signer, TransactionBuilder};

#[async_trait]
pub trait SmartContractTrait<'a>: Send + Sync {
	const DEFAULT_ITERATOR_COUNT: usize = 100;
	type P: JsonRpcProvider;

	async fn name(&self) -> String {
		self.get_manifest().await.name.clone().unwrap()
	}
	fn set_name(&mut self, _name: String) {
		// NNS contracts don't support setting names
		// This is intentionally a no-op as it's not supported
		eprintln!("Warning: Cannot set name for NNS contract - operation not supported");
	}

	fn script_hash(&self) -> H160;

	fn set_script_hash(&mut self, _script_hash: H160) {
		// NNS contracts don't support setting script hash
		// This is intentionally a no-op as it's not supported
		eprintln!("Warning: Cannot set script hash for NNS contract - operation not supported");
	}

	fn provider(&self) -> Option<&RpcClient<Self::P>>;

	async fn invoke_function(
		&self,
		function: &str,
		params: Vec<ContractParameter>,
	) -> Result<TransactionBuilder<Self::P>, ContractError> {
		let script = self.build_invoke_function_script(function, params).await.unwrap();
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
			.unwrap()
			.to_bytes();

		Ok(script)
	}

	async fn call_function_returning_string(
		&self,
		function: &str,
		params: Vec<ContractParameter>,
	) -> Result<String, ContractError> {
		let output = self.call_invoke_function(function, params, vec![]).await.unwrap();
		self.throw_if_fault_state(&output).unwrap();

		let item = output.stack[0].clone();
		match item.as_string() {
			Some(s) => Ok(s),
			None => Err(ContractError::UnexpectedReturnType("String".to_string())),
		}
	}

	async fn call_function_returning_int(
		&self,
		function: &str,
		params: Vec<ContractParameter>,
	) -> Result<i32, ContractError> {
		let output = self.call_invoke_function(function, params, vec![]).await.unwrap();
		self.throw_if_fault_state(&output).unwrap();

		let item = output.stack[0].clone();
		match item.as_int() {
			Some(i) => Ok(i as i32),
			None => Err(ContractError::UnexpectedReturnType("Int".to_string())),
		}
	}

	async fn call_function_returning_bool(
		&self,
		function: &str,
		params: Vec<ContractParameter>,
	) -> Result<bool, ContractError> {
		let output = self.call_invoke_function(function, params, vec![]).await.unwrap();
		self.throw_if_fault_state(&output).unwrap();

		let item = output.stack[0].clone();
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
			return Err(ContractError::from(ContractError::InvalidNeoName(
				"Function cannot be empty".to_string(),
			)));
		}

		let res = self
			.provider()
			.unwrap()
			.invoke_function(&self.script_hash().clone(), function.into(), params, Some(signers))
			.await?
			.clone();

		Ok(res)
	}

	fn throw_if_fault_state(&self, output: &InvocationResult) -> Result<(), ContractError> {
		if output.has_state_fault() {
			Err(ContractError::UnexpectedReturnType(output.exception.clone().unwrap()))
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
		let output = self.call_invoke_function(function, params, vec![]).await.unwrap();
		self.throw_if_fault_state(&output).unwrap();

		let item = &output.stack[0];
		item.as_bytes()
			.as_deref()
			.map(|b| ScriptHash::from_script(b))
			.ok_or_else(|| ContractError::UnexpectedReturnType("Script hash".to_string()))
	}

	async fn call_function_returning_iterator<U>(
		&self,
		function: &str,
		params: Vec<ContractParameter>,
		mapper: Arc<dyn Fn(StackItem) -> U + Send + Sync>,
	) -> Result<NeoIterator<U, Self::P>, ContractError>
	where
		U: Send + Sync, // Adding this bound if necessary
	{
		let output = self.call_invoke_function(function, params, vec![]).await.unwrap();
		self.throw_if_fault_state(&output).unwrap();

		let item = &output.stack[0];
		let StackItem::InteropInterface { id, interface: _ } = item else {
			return Err(ContractError::UnexpectedReturnType(format!(
				"Expected InteropInterface, got {:?}",
				item
			)));
		};

		let session_id = output
			.session_id
			.ok_or(ContractError::InvalidNeoNameServiceRoot("No session ID".to_string()))?;

		Ok(NeoIterator::new(session_id, id.clone(), mapper, None))
	}

	async fn call_function_and_unwrap_iterator<U>(
		&self,
		function: &str,
		params: Vec<ContractParameter>,
		_max_items: usize,
		mapper: impl Fn(StackItem) -> U + Send,
	) -> Result<Vec<U>, ContractError> {
		let script = ScriptBuilder::build_contract_call_and_unwrap_iterator(
			&self.script_hash(),
			function,
			&params,
			_max_items as u32, // Use the max_items parameter provided to the function
			Some(CallFlags::All),
		)
		.unwrap();

		let output = { self.provider().unwrap().invoke_script(script.to_hex_string(), vec![]) };

		let output = output.await.unwrap();

		self.throw_if_fault_state(&output).unwrap();

		let items = output.stack[0].as_array().unwrap().into_iter().map(mapper).collect();

		Ok(items)
	}

	fn calc_native_contract_hash(contract_name: &str) -> Result<H160, ContractError> {
		Self::calc_contract_hash(H160::zero(), 0, contract_name)
	}

	fn calc_contract_hash(
		sender: H160,
		nef_checksum: u32,
		contract_name: &str,
	) -> Result<H160, ContractError> {
		let mut script = ScriptBuilder::new();
		script
			.op_code(&[OpCode::Abort])
			.push_data(sender.to_vec())
			.push_integer(BigInt::from(nef_checksum))
			.push_data(contract_name.as_bytes().to_vec());

		Ok(H160::from_slice(&script.to_bytes()))
	}

	async fn get_manifest(&self) -> ContractManifest {
		let req =
			{ self.provider().unwrap().get_contract_state(self.script_hash()).await.unwrap() };

		req.manifest.clone()
	}
}
