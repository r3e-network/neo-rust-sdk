use std::{fmt, str::FromStr, sync::Arc};

use crate::{
	builder::Transaction, neo_clients::JsonRpcProvider, neo_wallets::WalletError, Address,
	ScriptHashExtension,
};
use async_trait::async_trait;
use coins_ledger::{
	common::APDUCommand,
	transports::{Ledger, LedgerAsync},
};
use p256::NistP256;
use primitive_types::{H160, H256};
use sha2::Digest;
use signature::hazmat::{PrehashSigner, PrehashVerifier};

/// Neo N3 APDU commands for Ledger devices.
pub mod apdu {
	use coins_ledger::common::APDUCommand;

	/// APDU command to get the Neo N3 address for a given derivation path.
	pub fn get_address(derivation_path: &[u32], display: bool) -> APDUCommand {
		let mut data = Vec::new();
		data.push(derivation_path.len() as u8);

		for item in derivation_path.iter() {
			data.push((*item >> 24) as u8);
			data.push((*item >> 16) as u8);
			data.push((*item >> 8) as u8);
			data.push(*item as u8);
		}

		APDUCommand {
			cla: 0x80,
			ins: 0x02,
			p1: if display { 0x01 } else { 0x00 },
			p2: 0x00,
			data: data.into(),
			response_len: Some(65),
		}
	}

	/// APDU command to sign a Neo N3 transaction.
	pub fn sign_tx(derivation_path: &[u32], tx_hash: &[u8]) -> APDUCommand {
		let mut data = Vec::new();
		data.push(derivation_path.len() as u8);

		for item in derivation_path.iter() {
			data.push((*item >> 24) as u8);
			data.push((*item >> 16) as u8);
			data.push((*item >> 8) as u8);
			data.push(*item as u8);
		}

		data.extend_from_slice(tx_hash);

		APDUCommand {
			cla: 0x80,
			ins: 0x04,
			p1: 0x00,
			p2: 0x00,
			data: data.into(),
			response_len: Some(64),
		}
	}

	/// APDU command to sign a Neo N3 message.
	pub fn sign_message(derivation_path: &[u32], message_hash: &[u8]) -> APDUCommand {
		let mut data = Vec::new();
		data.push(derivation_path.len() as u8);

		for item in derivation_path.iter() {
			data.push((*item >> 24) as u8);
			data.push((*item >> 16) as u8);
			data.push((*item >> 8) as u8);
			data.push(*item as u8);
		}

		data.extend_from_slice(message_hash);

		APDUCommand {
			cla: 0x80,
			ins: 0x08,
			p1: 0x00,
			p2: 0x00,
			data: data.into(),
			response_len: Some(64),
		}
	}
}

/// Represents a hierarchical deterministic path for Neo N3 accounts.
#[derive(Debug, Clone)]
pub enum HDPath {
	/// Ledger Live-style derivation path: m/44'/888'/0'/0/0
	LedgerLive(u32),
	/// Legacy derivation path: m/44'/888'/0'/0
	Legacy(u32),
	/// Custom derivation path
	Custom(Vec<u32>),
}

impl HDPath {
	/// Converts the HD path to a vector of integers.
	pub fn to_vec(&self) -> Vec<u32> {
		match self {
			HDPath::LedgerLive(index) => {
				vec![44 + 0x80000000, 888 + 0x80000000, 0 + 0x80000000, 0, *index]
			},
			HDPath::Legacy(index) => {
				vec![44 + 0x80000000, 888 + 0x80000000, 0 + 0x80000000, *index]
			},
			HDPath::Custom(path) => path.clone(),
		}
	}
}

/// A Ledger hardware wallet signer for Neo N3.
pub struct LedgerWallet<T: LedgerAsync> {
	/// The Ledger device
	pub(crate) ledger: Arc<T>,
	/// The derivation path
	pub(crate) derivation_path: HDPath,
	/// The wallet's address
	pub(crate) address: Option<Address>,
	/// The network ID
	pub(crate) network: Option<u64>,
}

impl<T: LedgerAsync> LedgerWallet<T> {
	/// Creates a new Ledger wallet with the specified derivation path and account index.
	pub async fn new(ledger: T, derivation_path: HDPath) -> Result<Self, WalletError> {
		let ledger = Arc::new(ledger);
		let mut wallet = Self { ledger, derivation_path, address: None, network: None };

		// Derive the address
		wallet.address = Some(wallet.get_address().await?);

		Ok(wallet)
	}

	/// Gets the Neo N3 address for the current derivation path.
	pub async fn get_address(&self) -> Result<Address, WalletError> {
		let path = self.derivation_path.to_vec();
		let command = apdu::get_address(&path, false);

		let response = self
			.ledger
			.exchange(&command)
			.await
			.map_err(|e| WalletError::LedgerError(format!("Failed to get address: {e}")))?;

		if response.retcode() != 0x9000 {
			return Err(WalletError::LedgerError(format!(
				"Ledger error: {:x}",
				response.retcode()
			)));
		}

		// The response data contains the public key and the Neo N3 address
		// Extract the public key (first 65 bytes) and derive the address
		let data = response
			.data()
			.ok_or_else(|| WalletError::LedgerError("No data in response".to_string()))?;
		if data.len() < 65 {
			return Err(WalletError::LedgerError("Invalid response data length".to_string()));
		}

		let public_key = &data[0..65];
		// Convert the public key to a Neo N3 address
		let address =
			Address::from_str(&format!("0x{}", H160::from_slice(&public_key[1..21]).to_hex()))
				.map_err(|e| WalletError::LedgerError(format!("Failed to derive address: {e}")))?;

		Ok(address)
	}

	/// Signs a transaction using the Ledger device.
	pub async fn sign_transaction<'a, P: JsonRpcProvider + 'static>(
		&self,
		tx: &Transaction<'a, P>,
	) -> Result<k256::ecdsa::Signature, WalletError> {
		let path = self.derivation_path.to_vec();

		// Get the transaction hash
		let tx_hash = tx.get_hash_data().await?;

		// Create the APDU command
		let command = apdu::sign_tx(&path, &tx_hash);

		// Send the command to the Ledger device
		let response =
			self.ledger.exchange(&command).await.map_err(|e| {
				WalletError::LedgerError(format!("Failed to sign transaction: {e}"))
			})?;

		if response.retcode() != 0x9000 {
			return Err(WalletError::LedgerError(format!(
				"Ledger error: {:x}",
				response.retcode()
			)));
		}

		// Parse the signature from the response
		let data = response
			.data()
			.ok_or_else(|| WalletError::LedgerError("No data in response".to_string()))?;
		if data.len() != 64 {
			return Err(WalletError::LedgerError("Invalid signature length".to_string()));
		}

		// Convert the signature to a Signature
		let r = H256::from_slice(&data[0..32]);
		let s = H256::from_slice(&data[32..64]);

		// Create a Signature from r and s
		let r_bytes: [u8; 32] = r.into();
		let s_bytes: [u8; 32] = s.into();
		let signature = k256::ecdsa::Signature::from_scalars(r_bytes, s_bytes)
			.map_err(|e| WalletError::LedgerError(format!("Failed to create signature: {e}")))?;

		Ok(signature)
	}

	/// Signs a message using the Ledger device.
	pub async fn sign_message(
		&self,
		message: &[u8],
	) -> Result<k256::ecdsa::Signature, WalletError> {
		let path = self.derivation_path.to_vec();

		// Hash the message using SHA-256
		let message_hash = sha2::Sha256::digest(message);

		// Create the APDU command
		let command = apdu::sign_message(&path, &message_hash);

		// Send the command to the Ledger device
		let response = self
			.ledger
			.exchange(&command)
			.await
			.map_err(|e| WalletError::LedgerError(format!("Failed to sign message: {e}")))?;

		if response.retcode() != 0x9000 {
			return Err(WalletError::LedgerError(format!(
				"Ledger error: {:x}",
				response.retcode()
			)));
		}

		// Parse the signature from the response
		let data = response
			.data()
			.ok_or_else(|| WalletError::LedgerError("No data in response".to_string()))?;
		if data.len() != 64 {
			return Err(WalletError::LedgerError("Invalid signature length".to_string()));
		}

		// Convert the signature to a Signature
		let r = H256::from_slice(&data[0..32]);
		let s = H256::from_slice(&data[32..64]);

		// Create a Signature from r and s
		let r_bytes: [u8; 32] = r.into();
		let s_bytes: [u8; 32] = s.into();
		let signature = k256::ecdsa::Signature::from_scalars(r_bytes, s_bytes)
			.map_err(|e| WalletError::LedgerError(format!("Failed to create signature: {e}")))?;

		Ok(signature)
	}
}

// Implement Debug for LedgerWallet
impl<T: LedgerAsync> fmt::Debug for LedgerWallet<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("LedgerWallet")
			.field("derivation_path", &self.derivation_path)
			.field("address", &self.address)
			.field("network", &self.network)
			.finish()
	}
}
