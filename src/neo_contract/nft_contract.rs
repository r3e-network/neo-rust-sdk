use async_trait::async_trait;
use primitive_types::H160;
use std::collections::HashMap;

use crate::{
	neo_clients::{JsonRpcProvider, RpcClient},
	neo_contract::{
		traits::{NonFungibleTokenTrait, SmartContractTrait, TokenTrait},
		ContractError,
	},
	neo_types::{Bytes, NNSName, StackItem},
};

#[derive(Debug)]
pub struct NftContract<'a, P: JsonRpcProvider> {
	script_hash: H160,
	total_supply: Option<u64>,
	decimals: Option<u8>,
	symbol: Option<String>,
	provider: Option<&'a RpcClient<P>>,
}

impl<'a, P: JsonRpcProvider> NftContract<'a, P> {
	pub fn new(script_hash: &H160, provider: Option<&'a RpcClient<P>>) -> Self {
		Self {
			script_hash: *script_hash,
			total_supply: None,
			decimals: None,
			symbol: None,
			provider,
		}
	}
}

#[async_trait]
impl<'a, P: JsonRpcProvider> TokenTrait<'a, P> for NftContract<'a, P> {
	fn total_supply(&self) -> Option<u64> {
		self.total_supply
	}

	fn set_total_supply(&mut self, total_supply: u64) {
		self.total_supply = Option::from(total_supply);
	}

	fn decimals(&self) -> Option<u8> {
		self.decimals
	}

	fn set_decimals(&mut self, decimals: u8) {
		self.decimals = Option::from(decimals);
	}

	fn symbol(&self) -> Option<String> {
		self.symbol.clone()
	}

	fn set_symbol(&mut self, symbol: String) {
		self.symbol = Option::from(symbol);
	}

	async fn resolve_nns_text_record(&self, _name: &NNSName) -> Result<H160, ContractError> {
		// NFT contracts don't typically resolve NNS text records
		// Return an error indicating this operation is not supported
		Err(ContractError::UnsupportedOperation(
			"NNS text record resolution is not supported for NFT contracts".to_string(),
		))
	}
}

#[async_trait]
impl<'a, P: JsonRpcProvider> SmartContractTrait<'a> for NftContract<'a, P> {
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

#[async_trait]
impl<'a, P: JsonRpcProvider> NonFungibleTokenTrait<'a, P> for NftContract<'a, P> {
	const TOKEN_URI: &'static str = "tokenURI";

	async fn owner_of(&mut self, token_id: Bytes) -> Result<H160, ContractError> {
		self.throw_if_divisible_nft().await?;

		self.call_function_returning_script_hash(
			<NftContract<P> as NonFungibleTokenTrait<P>>::OWNER_OF,
			vec![token_id.into()],
		)
		.await
	}

	async fn token_uri(&mut self, token_id: Bytes) -> Result<String, ContractError> {
		self.throw_if_divisible_nft().await?;

		self.call_function_returning_string(
			Self::TOKEN_URI,
			vec![token_id.into()],
		)
		.await
	}

	async fn properties(&mut self, token_id: Bytes) -> Result<HashMap<String, StackItem>, ContractError> {
		self.custom_properties(token_id).await
	}

	async fn custom_properties(&mut self, token_id: Bytes) -> Result<HashMap<String, StackItem>, ContractError> {
		let invocation_result = self.call_invoke_function(
			<NftContract<P> as NonFungibleTokenTrait<P>>::PROPERTIES,
			vec![token_id.into()],
			vec![],
		).await?;
		self.throw_if_fault_state(&invocation_result)?;

		let stack_item = invocation_result
			.get_first_stack_item()
			.map_err(|e| ContractError::InvalidResponse(e.to_string()))?;
		let map = stack_item.as_map().ok_or_else(|| {
			ContractError::UnexpectedReturnType(stack_item.to_string() + StackItem::MAP_VALUE)
		})?;

		map.into_iter()
			.map(|(k, v)| {
				let key = k
					.as_string()
					.ok_or_else(|| ContractError::UnexpectedReturnType("String".to_string()))?;
				Ok((key, v.clone()))
			})
			.collect()
	}
}
