use async_trait::async_trait;
use primitive_types::H160;

use crate::{
	neo_clients::{JsonRpcProvider, RpcClient},
	neo_contract::{
		traits::{FungibleTokenTrait, SmartContractTrait, TokenTrait},
		ContractError,
	},
	neo_types::NNSName,
};

#[derive(Debug)]
pub struct FungibleTokenContract<'a, P: JsonRpcProvider> {
	script_hash: H160,
	total_supply: Option<u64>,
	decimals: Option<u8>,
	symbol: Option<String>,
	provider: Option<&'a RpcClient<P>>,
}

impl<'a, P: JsonRpcProvider> FungibleTokenContract<'a, P> {
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
impl<'a, P: JsonRpcProvider> TokenTrait<'a, P> for FungibleTokenContract<'a, P> {
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
		Err(ContractError::UnsupportedOperation(
							"NNS text record resolution requires comprehensive domain name integration for fungible tokens".to_string(),
		))
	}
}

#[async_trait]
impl<'a, P: JsonRpcProvider> SmartContractTrait<'a> for FungibleTokenContract<'a, P> {
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
impl<'a, P: JsonRpcProvider> FungibleTokenTrait<'a, P> for FungibleTokenContract<'a, P> {}
