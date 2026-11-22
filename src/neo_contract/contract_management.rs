use async_trait::async_trait;
use primitive_types::H160;
use serde::{Deserialize, Serialize};

use crate::{
	ContractIdentifiers,
	neo_builder::TransactionBuilder,
	neo_clients::{APITrait, JsonRpcProvider, RpcClient},
	neo_contract::{ContractError, SmartContractTrait, checked_vm_integer},
	neo_types::{
		ContractParameter, ContractState, NefFile, ScriptHash, StackItem,
		serde_with_utils::{deserialize_script_hash, serialize_script_hash},
	},
};

/// A struct representing contract management functionalities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractManagement<'a, P: JsonRpcProvider> {
	#[serde(deserialize_with = "deserialize_script_hash")]
	#[serde(serialize_with = "serialize_script_hash")]
	script_hash: ScriptHash,
	#[serde(skip)]
	provider: Option<&'a RpcClient<P>>,
}

impl<'a, P: JsonRpcProvider + 'static> ContractManagement<'a, P> {
	pub const NAME: &'static str = "ContractManagement";

	pub fn new(provider: Option<&'a RpcClient<P>>) -> Self {
		Self { script_hash: Self::calc_native_contract_hash_unchecked(Self::NAME), provider }
	}

	/// Creates a ContractManagement with an explicit script hash (for non-native usage).
	pub fn with_script_hash(script_hash: H160, provider: Option<&'a RpcClient<P>>) -> Self {
		Self { script_hash, provider }
	}

	pub async fn get_minimum_deployment_fee(&self) -> Result<u64, ContractError> {
		let output = self.call_invoke_function("getMinimumDeploymentFee", vec![], vec![]).await?;
		self.throw_if_fault_state(&output)?;

		let item = output
			.get_first_stack_item()
			.map_err(|e| ContractError::InvalidResponse(e.to_string()))?;
		let value = item
			.as_int()
			.ok_or_else(|| ContractError::UnexpectedReturnType("Int".to_string()))?;

		u64::try_from(value).map_err(|_| {
			ContractError::InvalidResponse("Minimum deployment fee cannot be negative".to_string())
		})
	}

	pub async fn set_minimum_deployment_fee(
		&self,
		fee: u64,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		self.invoke_function("setMinimumDeploymentFee", vec![fee.into()]).await
	}

	pub async fn get_contract(&self, hash: H160) -> Result<ContractState, ContractError> {
		let provider = self.provider.ok_or_else(|| {
			ContractError::ProviderNotSet("Provider is required for ContractManagement".to_string())
		})?;
		Ok(provider.get_contract_state(hash).await?)
	}

	pub async fn get_contract_by_id(&self, id: i32) -> Result<ContractState, ContractError> {
		let hash = self.get_contract_hash_by_id(id).await?;
		self.get_contract(hash).await
	}

	pub async fn get_contract_hash_by_id(&self, id: i32) -> Result<ScriptHash, ContractError> {
		let result = self.call_invoke_function("getContractById", vec![id.into()], vec![]).await?;
		self.throw_if_fault_state(&result)?;

		let item = result
			.get_first_stack_item()
			.map_err(|e| ContractError::InvalidResponse(e.to_string()))?;

		let bytes = item
			.as_bytes()
			.ok_or_else(|| ContractError::UnexpectedReturnType("ByteString".to_string()))?;
		if bytes.len() != 20 {
			return Err(ContractError::InvalidScriptHash(format!(
				"Expected 20 bytes for ScriptHash, got {}",
				bytes.len()
			)));
		}
		Ok(ScriptHash::from_slice(&bytes))
	}

	pub async fn get_contract_hashes(&self) -> Result<ContractIdentifiers, ContractError> {
		let result = self.call_invoke_function("getContractHashes", vec![], vec![]).await?;
		self.throw_if_fault_state(&result)?;

		ContractIdentifiers::from_invocation_result(result)
			.map_err(|e| ContractError::InvalidResponse(e.to_string()))
	}

	/// Gets the ID of the contract with the specified hash.
	///
	/// Returns `None` if the contract does not exist. This is an optimized
	/// alternative to [`Self::get_contract`] when only the ID is needed, since
	/// the node skips the full contract state deserialization.
	///
	/// Available since Neo v3.11.2 (`getContractId`).
	pub async fn get_contract_id(&self, hash: H160) -> Result<Option<i32>, ContractError> {
		let result = self.call_invoke_function("getContractId", vec![hash.into()], vec![]).await?;
		self.throw_if_fault_state(&result)?;

		let item = result
			.get_first_stack_item()
			.map_err(|e| ContractError::InvalidResponse(e.to_string()))?;
		match item {
			StackItem::Any => Ok(None),
			_ => {
				let id = item
					.as_int()
					.ok_or_else(|| ContractError::UnexpectedReturnType("Int".to_string()))?;
				Ok(Some(checked_vm_integer::<i32>(id, "contract id")?))
			},
		}
	}

	pub async fn has_method(
		&self,
		hash: H160,
		method: &str,
		params: usize,
	) -> Result<bool, ContractError> {
		let result = self
			.call_invoke_function(
				"hasMethod",
				vec![hash.into(), method.into(), params.into()],
				vec![],
			)
			.await?;
		self.throw_if_fault_state(&result)?;

		let item = result
			.get_first_stack_item()
			.map_err(|e| ContractError::InvalidResponse(e.to_string()))?;
		item.as_bool()
			.ok_or_else(|| ContractError::UnexpectedReturnType("Bool".to_string()))
	}

	pub async fn deploy(
		&self,
		nef: &NefFile,
		manifest: &[u8],
		data: Option<ContractParameter>,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		let params = vec![nef.into(), manifest.into(), data.unwrap_or_else(ContractParameter::any)];
		self.invoke_function("deploy", params).await
	}

	pub async fn update(
		&self,
		nef: &NefFile,
		manifest: &[u8],
		data: Option<ContractParameter>,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		let params = vec![nef.into(), manifest.into(), data.unwrap_or_else(ContractParameter::any)];
		self.invoke_function("update", params).await
	}

	pub async fn destroy(&self) -> Result<TransactionBuilder<'_, P>, ContractError> {
		self.invoke_function("destroy", vec![]).await
	}
}

#[async_trait]
impl<'a, P: JsonRpcProvider> SmartContractTrait<'a> for ContractManagement<'a, P> {
	type P = P;

	fn script_hash(&self) -> H160 {
		self.script_hash
	}

	fn set_script_hash(&mut self, script_hash: H160) {
		self.script_hash = script_hash;
	}

	fn provider(&self) -> Option<&RpcClient<P>> {
		self.provider
	}
}
