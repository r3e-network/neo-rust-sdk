use async_trait::async_trait;
use primitive_types::H160;
use serde::{Deserialize, Serialize};

use crate::{
	ValueExtension,
	neo_builder::TransactionBuilder,
	neo_clients::{APITrait, JsonRpcProvider, RpcClient},
	neo_contract::{ContractError, traits::SmartContractTrait},
	neo_crypto::Secp256r1PublicKey,
	neo_types::{
		ContractParameter, ScriptHash, StackItem,
		serde_with_utils::{deserialize_script_hash, serialize_script_hash},
	},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleManagement<'a, P: JsonRpcProvider> {
	#[serde(deserialize_with = "deserialize_script_hash")]
	#[serde(serialize_with = "serialize_script_hash")]
	script_hash: ScriptHash,
	#[serde(skip)]
	provider: Option<&'a RpcClient<P>>,
}

impl<'a, P: JsonRpcProvider + 'static> RoleManagement<'a, P> {
	pub const NAME: &'static str = "RoleManagement";
	// const SCRIPT_HASH: H160 = Self::calc_native_contract_hash(Self::NAME).unwrap(); // compute hash

	pub fn new(provider: Option<&'a RpcClient<P>>) -> Self {
		Self { script_hash: Self::calc_native_contract_hash_unchecked(Self::NAME), provider }
	}

	pub async fn get_designated_by_role(
		&self,
		role: Role,
		block_index: i32,
	) -> Result<Vec<Secp256r1PublicKey>, ContractError> {
		self.check_block_index_validity(block_index).await?;

		let invocation = self
			.call_invoke_function(
				"getDesignatedByRole",
				vec![role.into(), block_index.into()],
				vec![],
			)
			.await?;
		self.throw_if_fault_state(&invocation)?;

		let stack_item = invocation
			.get_first_stack_item()
			.map_err(|e| ContractError::InvalidResponse(e.to_string()))?;

		let designated_items = stack_item
			.as_array()
			.ok_or_else(|| ContractError::UnexpectedReturnType("Array".to_string()))?;

		let designated = designated_items
			.into_iter()
			.map(|item| {
				let bytes = item.as_bytes().ok_or_else(|| {
					ContractError::UnexpectedReturnType("Public key bytes".to_string())
				})?;
				Secp256r1PublicKey::from_bytes(&bytes)
					.map_err(|e| ContractError::InvalidResponse(format!("Invalid public key: {e}")))
			})
			.collect::<Result<Vec<Secp256r1PublicKey>, ContractError>>()?;

		Ok(designated)
	}

	async fn check_block_index_validity(&self, block_index: i32) -> Result<(), ContractError> {
		if block_index < 0 {
			return Err(ContractError::InvalidNeoName("Block index must be positive".to_string()));
		}

		let provider = self.provider.ok_or_else(|| {
			ContractError::ProviderNotSet("Provider is required for RoleManagement".to_string())
		})?;

		let current_block_count = provider.get_block_count().await?;

		if block_index > current_block_count as i32 {
			return Err(ContractError::InvalidNeoName(format!(
				"Block index {} exceeds current block count {}",
				block_index, current_block_count
			)));
		}

		Ok(())
	}

	pub async fn designate_as_role(
		&self,
		role: Role,
		pub_keys: Vec<Secp256r1PublicKey>,
	) -> Result<TransactionBuilder<'_, P>, ContractError> {
		if pub_keys.is_empty() {
			return Err(ContractError::InvalidNeoName(
				"At least 1 public key is required".to_string(),
			));
		}

		let params: Vec<_> = pub_keys.into_iter().map(|key| key.to_value()).collect();

		self.invoke_function("designateAsRole", vec![role.into(), params.into()]).await
	}
}

#[async_trait]
impl<'a, P: JsonRpcProvider> SmartContractTrait<'a> for RoleManagement<'a, P> {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Role {
	/// State validation node.
	StateValidator = 4,
	/// Oracle node.
	Oracle = 8,
	/// NeoFS alphabet node.
	NeoFSAlphabetNode = 16,
	/// P2P notary node (available after HF_Echidna).
	P2PNotary = 32,
}

impl Role {
	pub const fn byte(self) -> u8 {
		self as u8
	}
}

impl From<Role> for StackItem {
	fn from(role: Role) -> Self {
		StackItem::Integer { value: role.byte() as i64 }
	}
}

impl From<Role> for ContractParameter {
	fn from(role: Role) -> ContractParameter {
		ContractParameter::integer(role.byte() as i64)
	}
}
