use std::fmt;

use p256::ecdsa::Signature;
use primitive_types::H256;
use serde_derive::{Deserialize, Serialize};
use signature::hazmat::{PrehashSigner, PrehashVerifier};

use crate::{
	neo_builder::{Transaction, TransactionError},
	neo_clients::JsonRpcProvider,
	neo_crypto::HashableForVec,
	neo_types::Address,
	neo_wallets::WalletError,
};

/// A Neo private-public key pair which can be used for signing messages.
///
/// # Examples
///
/// ## Signing and Verifying a message
///
/// The wallet can be used to produce ECDSA [`p256::NistP256`] objects, which can be
/// then verified.
///
/// ```ignore
/// // WalletSigner is used for transaction signing
/// // Example usage is demonstrated in integration tests
/// ```
///
/// [`p256::NistP256`]: p256::NistP256
#[derive(Clone, Serialize, Deserialize)]
pub struct WalletSigner<D: PrehashSigner<Signature>> {
	/// The WalletSigner's private Key
	pub(crate) signer: D,
	/// The wallet's address
	pub(crate) address: Address,
	pub(crate) network: Option<u64>,
}

impl<D: PrehashSigner<Signature>> WalletSigner<D> {
	/// Creates a new `WalletSigner` instance using an external `Signer` and associated Ethereum address.
	///
	/// # Arguments
	///
	/// * `signer` - An implementation of the `PrehashSigner` trait capable of signing messages.
	/// * `address` - The Ethereum address associated with the signer's public key.
	///
	/// # Returns
	///
	/// A new `WalletSigner` instance.
	pub fn new_with_signer(signer: D, address: Address) -> Self {
		WalletSigner { signer, address, network: None }
	}
}

impl<D: Sync + Send + PrehashSigner<Signature>> WalletSigner<D> {
	/// Signs a given `Transaction`, using the wallet's private key.
	///
	/// # Arguments
	///
	/// * `tx` - A reference to the transaction to be signed.
	///
	/// # Returns
	///
	/// A `Result` containing the `p256::NistP256` of the transaction, or a `WalletError` on failure.
	pub(crate) async fn sign_transaction<'a, P: JsonRpcProvider + 'static>(
		&self,
		tx: &Transaction<'a, P>,
	) -> Result<Signature, WalletError> {
		let tx_with_network = tx;
		if tx_with_network.network().is_none() {
			// in the case we don't have a network, let's use the signer chain id instead
			// tx_with_network.set_network(self.network.map(|n| n as u32));
		}
		let hash_data = tx_with_network.get_hash_data().await.map_err(|e| {
			WalletError::TransactionError(TransactionError::TransactionConfiguration(format!(
				"Failed to get transaction hash data: {}",
				e
			)))
		})?;

		self.signer.sign_prehash(&hash_data).map_err(|_| WalletError::SignHashError)
	}

	/// Signs a given hash directly, without performing any additional hashing.
	///
	/// # Arguments
	///
	/// * `hash` - A `H256` hash to be signed.
	///
	/// # Returns
	///
	/// A `Result` containing the `p256::NistP256` of the hash, or a `WalletError` on failure.
	pub fn sign_hash(&self, hash: H256) -> Result<Signature, WalletError> {
		self.signer.sign_prehash(hash.as_ref()).map_err(|_| WalletError::SignHashError)
	}

	/// Signs a given message, using the wallet's private key.
	/// The message will be hashed using the `Sha256` algorithm before being signed.
	///
	/// # Arguments
	///
	/// * `message` - The message to be signed.
	///
	/// # Returns
	///
	/// A `Result` containing the `p256::NistP256` of the message, or a `WalletError` on failure.
	pub async fn sign_message(&self, message: &[u8]) -> Result<Signature, WalletError> {
		let hash = message.hash256();
		self.sign_hash(H256::from_slice(hash.as_slice()))
	}

	/// Returns a reference to the wallet's signer.
	///
	/// # Returns
	///
	/// A reference to the `D`, the signer.
	pub fn signer(&self) -> &D {
		&self.signer
	}

	/// Returns the wallet's associated address.
	///
	/// # Returns
	///
	/// The `Address` of the wallet.
	pub(crate) fn address(&self) -> Address {
		self.address.clone()
	}

	/// Gets the wallet's chain id
	fn network(&self) -> Option<u64> {
		self.network
	}

	/// Associates the wallet with a specific network ID.
	///
	/// # Arguments
	///
	/// * `network` - The network ID to associate with the wallet.
	///
	/// # Returns
	///
	/// The `WalletSigner` instance with the network ID set.
	fn with_network<T: Into<u64>>(mut self, network: T) -> Self {
		self.network = Some(network.into());
		self
	}
}

// do not log the signer
impl<D: PrehashSigner<Signature> + PrehashVerifier<Signature>> fmt::Debug for WalletSigner<D> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("WalletSigner")
			.field("address", &self.address)
			.field("chain_Id", &self.network)
			.finish()
	}
}
