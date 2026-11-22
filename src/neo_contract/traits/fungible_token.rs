use crate::{
	Bytes, ContractParameter, NNSName, ScriptHash,
	builder::{AccountSigner, TransactionBuilder},
	neo_clients::JsonRpcProvider,
	neo_contract::{ContractError, FungibleTokenContract, TokenTrait},
	neo_protocol::{Account, AccountTrait},
	neo_wallets::Wallet,
};
use async_trait::async_trait;
use primitive_types::H160;

#[async_trait]
pub trait FungibleTokenTrait<'a, P: JsonRpcProvider>: TokenTrait<'a, P> {
	const BALANCE_OF: &'static str = "balanceOf";
	const TRANSFER: &'static str = "transfer";

	async fn get_balance_of(&self, script_hash: &ScriptHash) -> Result<i64, ContractError> {
		self.get_balance_of_hash160(script_hash).await
	}

	async fn get_balance_of_hash160(&self, script_hash: &H160) -> Result<i64, ContractError> {
		self.call_function_returning_int(Self::BALANCE_OF, vec![script_hash.into()])
			.await
	}

	async fn get_total_balance(&self, wallet: &Wallet) -> Result<i64, ContractError> {
		let mut sum = 0;
		for account in wallet.accounts.values() {
			sum += self.get_balance_of(&account.address_or_scripthash().script_hash()).await?;
		}
		Ok(sum)
	}

	async fn transfer_from_account(
		&self,
		from: &Account,
		to: &ScriptHash,
		amount: i64,
		data: Option<ContractParameter>,
	) -> Result<TransactionBuilder<P>, ContractError> {
		let mut builder = self
			.transfer_from_hash160(&from.address_or_scripthash().script_hash(), to, amount, data)
			.await?;
		let signer = AccountSigner::called_by_entry(from)
			.map_err(|err| ContractError::RuntimeError(err.to_string()))?;
		builder
			.set_signers(vec![signer.into()])
			.map_err(|err| ContractError::RuntimeError(err.to_string()))?;

		Ok(builder)
	}

	async fn transfer_from_hash160(
		&self,
		from: &ScriptHash,
		to: &ScriptHash,
		amount: i64,
		data: Option<ContractParameter>,
	) -> Result<TransactionBuilder<P>, ContractError> {
		if amount < 0 {
			return Err(ContractError::InvalidArgError(
				"The amount must be greater than or equal to 0.".to_string(),
			));
		}

		let transfer_script = self.build_transfer_script(from, to, amount, data).await?;
		let mut builder = TransactionBuilder::new();
		builder.set_script(Some(transfer_script));
		Ok(builder)
	}

	async fn build_transfer_script(
		&self,
		from: &ScriptHash,
		to: &ScriptHash,
		amount: i64,
		data: Option<ContractParameter>,
	) -> Result<Bytes, ContractError> {
		self.build_invoke_function_script(
			<FungibleTokenContract<P> as FungibleTokenTrait<P>>::TRANSFER,
			vec![
				from.into(),
				to.into(),
				amount.into(),
				data.unwrap_or_else(ContractParameter::any),
			],
		)
		.await
	}

	// MARK: Transfer using NNS

	async fn transfer_from_account_to_nns(
		&self,
		from: &Account,
		to: &NNSName,
		amount: i64,
		data: Option<ContractParameter>,
	) -> Result<TransactionBuilder<P>, ContractError> {
		let mut builder = self
			.transfer_from_hash160_to_nns(&from.get_script_hash(), to, amount, data)
			.await?;
		let signer = AccountSigner::called_by_entry(from)
			.map_err(|err| ContractError::RuntimeError(err.to_string()))?;
		builder
			.set_signers(vec![signer.into()])
			.map_err(|err| ContractError::RuntimeError(err.to_string()))?;

		Ok(builder)
	}

	async fn transfer_from_hash160_to_nns(
		&self,
		from: &ScriptHash,
		to: &NNSName,
		amount: i64,
		data: Option<ContractParameter>,
	) -> Result<TransactionBuilder<P>, ContractError> {
		let script_hash = self.resolve_nns_text_record(to).await?;
		self.transfer_from_hash160(from, &script_hash, amount, data).await
	}
}
