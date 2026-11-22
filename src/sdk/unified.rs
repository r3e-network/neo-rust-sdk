use alloy::primitives::{Address as AlloyAddress, U256};
use alloy::sol_types::SolCall;

use crate::neo_clients::HttpProvider as N3HttpProvider;
use crate::neo_error::unified::{ErrorRecovery, NeoError};
use crate::neo_types::ScriptHash;
use crate::neo_wallets::WalletTrait;
use crate::neo_wallets::wallet::Wallet as N3Wallet;
use crate::neo_x::bridge::bridge_contract::NeoXBridgeContract;
use crate::neo_x::bridge::evm_bridge::NeoXBridgeContractEVM;
use crate::neo_x::evm::provider::NeoXProvider;
use crate::neo_x::evm::transaction::NeoXTransaction;
use crate::neo_x::evm::wallet::{NeoXClient, NeoXWallet};
use crate::sdk::{DecimalAmount, Token};
use std::str::FromStr;

/// Build a [`NeoError::Validation`] for a malformed amount field.
fn amount_validation_error<E: std::fmt::Display>(amount: &str, err: E) -> NeoError {
	NeoError::Validation {
		message: err.to_string(),
		field: "amount".into(),
		value: Some(amount.to_string()),
		recovery: ErrorRecovery::new()
			.suggest("Provide a non-negative decimal value")
			.suggest("Check the token's decimals scale"),
	}
}

/// Gas price for Neo X EVM transactions (1 Gwei in Wei).
const NEOX_GAS_PRICE_WEI: u64 = 1_000_000_000;

/// Gas limit for simple Neo X native token transfers.
const NEOX_TRANSFER_GAS_LIMIT: u64 = 21_000;

/// Gas limit for Neo X bridge contract interactions.
const NEOX_BRIDGE_GAS_LIMIT: u64 = 200_000;

/// Number of decimals for the GAS token on Neo N3.
const GAS_DECIMALS: u8 = 8;

/// Number of decimals for the native token on the Neo X EVM side
/// (18, matching the Ethereum convention the EVM layer follows).
const NEOX_NATIVE_DECIMALS: u8 = 18;

/// Parse a human-readable decimal amount on the given scale into base units.
fn parse_amount_base_units(amount: &str, decimals: u8) -> Result<U256, NeoError> {
	let parsed =
		DecimalAmount::parse(amount, decimals).map_err(|e| amount_validation_error(amount, e))?;
	U256::from_str(parsed.raw()).map_err(|e| amount_validation_error(amount, e))
}

/// Unified Ecosystem Client that pairs a Provider with a Wallet for either Neo N3 or Neo X.
/// Uses the SDK's native Neo N3 APIs and Alloy-backed Neo X APIs behind one interface.
pub enum EcosystemClient<'a> {
	/// N3 Network Client
	N3 {
		/// High-level Neo N3 RPC + transaction client.
		provider: crate::sdk::Neo,
		/// Wallet holding the signing key for outbound N3 transactions.
		wallet: N3Wallet,
	},
	/// Neo X EVM Network Client
	NeoX {
		/// EVM client paired with a Neo X wallet.
		client: NeoXClient<'a, N3HttpProvider>,
	},
}

impl<'a> EcosystemClient<'a> {
	/// Creates a new N3 client
	pub fn new_n3(provider: crate::sdk::Neo, wallet: N3Wallet) -> Self {
		Self::N3 { provider, wallet }
	}

	/// Creates a new Neo X EVM client using standard RPC
	pub fn new_neox(wallet: NeoXWallet, provider: NeoXProvider<'a, N3HttpProvider>) -> Self {
		let client = NeoXClient::new(wallet, provider);
		Self::NeoX { client }
	}

	/// Connects to Neo X through the configured third-party protected RPC endpoint.
	///
	/// Protection characteristics are determined by the endpoint operator; routing through this
	/// endpoint does not guarantee prevention of front-running or sandwich attacks.
	pub fn new_neox_anti_mev(wallet: NeoXWallet) -> Self {
		let provider = NeoXProvider::new_anti_mev(None);
		let client = NeoXClient::new(wallet, provider);
		Self::NeoX { client }
	}

	/// Gets the native gas/token balance of the configured wallet.
	///
	/// Returns a human-readable decimal string on both chains, using each
	/// chain's native scale (8 decimals on Neo N3, 18 on Neo X).
	pub async fn get_balance(&self) -> Result<String, NeoError> {
		match self {
			Self::N3 { provider, wallet } => {
				let address = wallet
					.default_account()
					.ok_or_else(crate::sdk::no_default_account_error)?
					.address_or_scripthash
					.address();
				let balance = provider.get_balance(&address).await?;
				Ok(balance.gas.to_string())
			},
			Self::NeoX { client } => {
				let bal = client
					.get_balance()
					.await
					.map_err(|e| NeoError::network("Neo X balance lookup", e))?;
				// Normalize raw Wei to the same human-decimal form the N3 arm
				// returns, instead of leaking base units to callers.
				let amount = DecimalAmount::try_from_raw(bal.to_string(), NEOX_NATIVE_DECIMALS)
					.map_err(|e| NeoError::Other {
						message: format!("Invalid Neo X balance response: {e}"),
						source: None,
						recovery: ErrorRecovery::new()
							.suggest("Inspect the raw balance returned by the EVM provider"),
					})?;
				Ok(amount.to_fixed_string())
			},
		}
	}

	/// Transfers the native asset (GAS) from the active wallet to the target address.
	///
	/// `amount` is a human-readable decimal string in the native token's scale
	/// on both chains: 8 decimals on Neo N3, 18 decimals on Neo X
	/// (e.g. `"1.5"` sends 1.5 GAS on either chain).
	pub async fn transfer(&self, to: &str, amount: &str) -> Result<String, NeoError> {
		match self {
			Self::N3 { provider, wallet } => {
				let parsed = DecimalAmount::parse(amount, GAS_DECIMALS)
					.map_err(|e| amount_validation_error(amount, e))?;
				let amount_u64 =
					parsed.raw().parse::<u64>().map_err(|e| amount_validation_error(amount, e))?;
				let tx_hash = provider.transfer(wallet, to, amount_u64, Token::GAS).await?;
				Ok(tx_hash)
			},
			Self::NeoX { client } => {
				let to_addr = primitive_types::H160::from_str(to).map_err(|e| {
					NeoError::validation("to", Some(to.to_string()), &e.to_string())
				})?;
				let value = parse_amount_base_units(amount, NEOX_NATIVE_DECIMALS)?;

				let request = NeoXTransaction::build_alloy_request(
					Some(to_addr),
					vec![],
					value,
					NEOX_TRANSFER_GAS_LIMIT,
					NEOX_GAS_PRICE_WEI,
				);
				let receipt = client
					.send_transaction_request(request)
					.await
					.map_err(|e| NeoError::transaction("Neo X transfer failed", e))?;
				Ok(format!("{:?}", receipt.transaction_hash))
			},
		}
	}

	/// Bridges tokens from the current chain to the other.
	/// If currently on N3, bridges to Neo X.
	/// If currently on Neo X, bridges to N3.
	///
	/// `amount` is a human-readable decimal string in the native token's scale
	/// on both chains (8 decimals on Neo N3, 18 on Neo X), matching
	/// [`Self::transfer`].
	pub async fn bridge_to_other_chain(
		&self,
		destination_address: &str,
		amount: &str,
	) -> Result<String, NeoError> {
		match self {
			Self::N3 { provider, wallet } => {
				let parsed = DecimalAmount::parse(amount, GAS_DECIMALS)
					.map_err(|e| amount_validation_error(amount, e))?;
				let amount_i64 = parsed
					.raw_i64()
					.ok_or_else(|| amount_validation_error(amount, "amount overflows i64"))?;

				let rpc_client = provider.client();
				let bridge = NeoXBridgeContract::new(Some(rpc_client)).map_err(|e| {
					NeoError::contract(
						"Failed to bind NeoX bridge contract",
						Some("NeoXBridge".into()),
						None,
						e,
					)
				})?;

				let account =
					wallet.default_account().ok_or_else(crate::sdk::no_default_account_error)?;

				let gas_token = ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")
					.map_err(|e| {
						NeoError::validation("gas_token", None::<String>, &e.to_string())
					})?;

				let mut builder = bridge
					.deposit(&gas_token, amount_i64, destination_address, account)
					.await
					.map_err(|e| NeoError::transaction("NeoX bridge deposit", e))?;

				let mut signed_tx = builder
					.sign()
					.await
					.map_err(|e| NeoError::transaction("Failed to sign bridge tx", e))?;
				let tx_response = signed_tx
					.send_tx()
					.await
					.map_err(|e| NeoError::transaction("Failed to send bridge tx", e))?;
				Ok(format!("N3 -> Neo X Bridge Transaction Sent: {:?}", tx_response.hash))
			},
			Self::NeoX { client } => {
				let amount_wei = parse_amount_base_units(amount, NEOX_NATIVE_DECIMALS)?;
				let token_addr = AlloyAddress::ZERO;

				let evm =
					client.provider.evm_provider().ok_or_else(|| NeoError::Configuration {
						message: "No EVM provider configured".into(),
						field: Some("evm_provider".into()),
						recovery: ErrorRecovery::new()
							.suggest("Configure the EVM provider when constructing NeoXProvider"),
					})?;
				let bridge = NeoXBridgeContractEVM::default_bridge(evm.clone()).map_err(|e| {
					NeoError::contract(
						"Failed to bind NeoX EVM bridge",
						Some("NeoXBridgeEVM".into()),
						Some("default_bridge".into()),
						e,
					)
				})?;

				// ABI-encode the withdraw calldata without broadcasting, then wrap it
				// in a NeoXTransaction addressed to the bridge contract.
				let withdraw_call = crate::neo_x::bridge::evm_bridge::NeoXBridgeEVM::withdrawCall {
					token: token_addr,
					amount: amount_wei,
					destination: destination_address.to_string(),
				};
				let data = withdraw_call.abi_encode();
				let to_addr = primitive_types::H160::from(bridge.address().into_array());

				let tx = NeoXTransaction::new(
					Some(to_addr),
					data,
					0,
					NEOX_BRIDGE_GAS_LIMIT,
					NEOX_GAS_PRICE_WEI,
				);

				let receipt = client
					.send_transaction(tx)
					.await
					.map_err(|e| NeoError::transaction("Failed to send Neo X bridge tx", e))?;
				Ok(format!("Neo X -> N3 Bridge Transaction Sent: {:?}", receipt.transaction_hash))
			},
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_amount_uses_chain_specific_scale() {
		// Neo X native token: 18 decimals — "1.5" means 1.5 GAS, not 1.5 Wei.
		let neox = parse_amount_base_units("1.5", NEOX_NATIVE_DECIMALS).unwrap();
		assert_eq!(neox, U256::from(1_500_000_000_000_000_000u64));

		// Whole numbers and zero parse cleanly.
		assert_eq!(parse_amount_base_units("0", NEOX_NATIVE_DECIMALS).unwrap(), U256::ZERO);
	}

	#[test]
	fn parse_amount_rejects_invalid_and_over_precise_input() {
		// More than 18 fractional digits cannot be represented exactly.
		assert!(parse_amount_base_units("0.1234567890123456789", NEOX_NATIVE_DECIMALS).is_err());
		// Negative and non-numeric inputs are rejected.
		assert!(parse_amount_base_units("-1", NEOX_NATIVE_DECIMALS).is_err());
		assert!(parse_amount_base_units("abc", NEOX_NATIVE_DECIMALS).is_err());
		assert!(parse_amount_base_units("", NEOX_NATIVE_DECIMALS).is_err());
	}

	#[test]
	fn neox_balance_formats_as_human_decimal() {
		// The NeoX get_balance arm converts raw Wei into a fixed decimal string.
		let bal = U256::from(1_500_000_000_000_000_000u64);
		let amount = DecimalAmount::try_from_raw(bal.to_string(), NEOX_NATIVE_DECIMALS).unwrap();
		assert_eq!(amount.to_fixed_string(), "1.500000000000000000");
	}
}
