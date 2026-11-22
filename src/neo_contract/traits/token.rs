use async_trait::async_trait;
use primitive_types::H160;

use crate::{
	neo_clients::JsonRpcProvider,
	neo_contract::{ContractError, SmartContractTrait, checked_vm_integer},
	neo_types::NNSName,
};

#[async_trait]
pub trait TokenTrait<'a, P: JsonRpcProvider>: SmartContractTrait<'a, P = P> {
	const TOTAL_SUPPLY: &'static str = "totalSupply";
	const SYMBOL: &'static str = "symbol";
	const DECIMALS: &'static str = "decimals";

	fn total_supply(&self) -> Option<u64>;

	fn set_total_supply(&mut self, total_supply: u64);

	fn decimals(&self) -> Option<u8>;

	fn set_decimals(&mut self, decimals: u8);

	fn symbol(&self) -> Option<String>;

	fn set_symbol(&mut self, symbol: String);

	async fn get_total_supply(&mut self) -> Result<u64, ContractError> {
		if let Some(supply) = &self.total_supply() {
			return Ok(*supply);
		}

		let supply = checked_vm_integer(
			self.call_function_returning_int(Self::TOTAL_SUPPLY, vec![]).await?,
			"token total supply",
		)?;

		self.set_total_supply(supply);
		Ok(supply)
	}

	async fn get_decimals(&mut self) -> Result<u8, ContractError> {
		if let Some(decimals) = &self.decimals() {
			return Ok(*decimals);
		}

		let decimals = checked_vm_integer(
			self.call_function_returning_int(Self::DECIMALS, vec![]).await?,
			"token decimals",
		)?;

		self.set_decimals(decimals);
		Ok(decimals)
	}

	// Other methods

	async fn get_symbol(&mut self) -> Result<String, ContractError> {
		if let Some(symbol) = &self.symbol() {
			return Ok(symbol.clone());
		}

		let symbol = self.call_function_returning_string(Self::SYMBOL, vec![]).await?;

		self.set_symbol(symbol.clone());
		Ok(symbol)
	}

	fn to_fractions(&self, amount: u64, decimals: u32) -> Result<i64, ContractError> {
		let multiplier = 10_u64
			.checked_pow(decimals)
			.ok_or_else(|| ContractError::RuntimeError("Decimals exponent overflow".to_string()))?;

		let scaled = (amount as u128)
			.checked_mul(multiplier as u128)
			.filter(|value| *value <= i64::MAX as u128)
			.ok_or_else(|| {
				ContractError::RuntimeError(
					"Amount is too large to fit into i64 fractions".to_string(),
				)
			})?;

		i64::try_from(scaled).map_err(|_| {
			ContractError::RuntimeError("Amount is too large to fit into i64 fractions".to_string())
		})
	}

	/// Resolves an NNS name to a script hash.
	///
	/// The default implementation returns an error indicating NNS resolution
	/// requires a configured NNS contract. Override this in implementations
	/// that support NNS-based address resolution.
	async fn resolve_nns_text_record(&self, name: &NNSName) -> Result<H160, ContractError> {
		Err(ContractError::RuntimeError(format!(
			"NNS resolution for '{}' is not supported by this token contract. \
			 Use NameService directly to resolve NNS names.",
			name.name()
		)))
	}
}
