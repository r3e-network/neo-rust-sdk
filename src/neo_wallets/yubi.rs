//! Helpers for creating wallets backed by a YubiHSM2 device.
//!
//! The [`yubihsm`] crate's typed `ecdsa::Signer<C>` wrapper is coupled to the
//! crate's own elliptic-curve dependency version. To keep the SDK's `p256`
//! integration independent of that coupling, this module wraps the raw
//! YubiHSM2 client APIs (`sign_ecdsa_prehash_raw` / `get_public_key`) in a
//! thin [`YubiHsmSigner`] that plugs directly into [`WalletSigner`].
//!
//! [`WalletSigner`]: crate::neo_wallets::WalletSigner
#[cfg(feature = "yubi")]
use crate::{
	Address, neo_clients::public_key_to_address, neo_crypto::Secp256r1PublicKey,
	neo_error::WalletError, neo_wallets::WalletSigner,
};
#[cfg(feature = "yubi")]
use p256::{ecdsa::Signature, elliptic_curve::sec1::ToSec1Point as _};
#[cfg(feature = "yubi")]
use signature::hazmat::PrehashSigner;
#[cfg(feature = "yubi")]
use yubihsm::{Capability, Client, Connector, Credentials, Domain, object, object::Label};

/// A signer backed by an ECDSA (NIST P-256) asymmetric key stored on a
/// YubiHSM2 device.
///
/// Signing is performed on the device; the private key never leaves the HSM.
#[cfg(feature = "yubi")]
#[derive(Clone)]
pub struct YubiHsmSigner {
	client: yubihsm::Client,
	key_id: object::Id,
	public_key: p256::PublicKey,
}

#[cfg(feature = "yubi")]
impl YubiHsmSigner {
	/// Builds a signer from an open YubiHSM client and an existing EcP256 key.
	fn from_client(client: Client, key_id: object::Id) -> Result<Self, WalletError> {
		let hsm_key = client.get_public_key(key_id).map_err(|e| {
			WalletError::YubiHsmError(format!("Failed to get public key from YubiHSM: {e}"))
		})?;

		if hsm_key.algorithm != yubihsm::asymmetric::Algorithm::EcP256 {
			return Err(WalletError::YubiHsmError(format!(
				"YubiHSM key {key_id} is not an EcP256 (secp256r1) key"
			)));
		}

		// The HSM returns X || Y (64 bytes) without the SEC1 point tag, so the
		// uncompressed 0x04 prefix must be prepended for `from_sec1_bytes`.
		let mut encoded = Vec::with_capacity(65);
		encoded.push(0x04);
		encoded.extend_from_slice(&hsm_key.bytes);

		let public_key = p256::elliptic_curve::PublicKey::<p256::NistP256>::from_sec1_bytes(
			&encoded,
		)
		.map_err(|e| {
			WalletError::YubiHsmError(format!("YubiHSM returned an invalid public key: {e}"))
		})?;

		Ok(Self { client, key_id, public_key })
	}

	/// The [`p256::PublicKey`] corresponding to the key stored on the device.
	pub fn public_key(&self) -> &p256::PublicKey {
		&self.public_key
	}

	/// The object ID of the signing key on the device.
	pub fn key_id(&self) -> object::Id {
		self.key_id
	}

	/// The underlying YubiHSM client.
	pub fn client(&self) -> &Client {
		&self.client
	}
}

#[cfg(feature = "yubi")]
impl PrehashSigner<Signature> for YubiHsmSigner {
	fn sign_prehash(&self, prehash: &[u8]) -> Result<Signature, signature::Error> {
		let der = self
			.client
			.sign_ecdsa_prehash_raw(self.key_id, prehash.to_vec())
			.map_err(|_| signature::Error::new())?;

		Signature::from_der(&der).map_err(|_| signature::Error::new())
	}
}

#[cfg(feature = "yubi")]
impl WalletSigner<YubiHsmSigner> {
	/// Connects to a YubiHSM2 ECDSA (secp256r1) account at the provided object ID.
	pub fn connect(
		connector: Connector,
		credentials: Credentials,
		id: object::Id,
	) -> Result<Self, WalletError> {
		let client = Client::open(connector, credentials, true).map_err(|e| {
			WalletError::YubiHsmError(format!("Failed to open YubiHSM client: {e}"))
		})?;

		let signer = YubiHsmSigner::from_client(client, id)?;

		Ok(signer.into())
	}

	/// Creates a new random ECDSA keypair on the YubiHSM at the provided ID.
	pub fn new(
		connector: Connector,
		credentials: Credentials,
		id: object::Id,
		label: Label,
		domain: Domain,
	) -> Result<Self, WalletError> {
		let client = Client::open(connector, credentials, true).map_err(|e| {
			WalletError::YubiHsmError(format!("Failed to open YubiHSM client: {e}"))
		})?;

		let id = client
			.generate_asymmetric_key(
				id,
				label,
				domain,
				Capability::SIGN_ECDSA,
				yubihsm::asymmetric::Algorithm::EcP256,
			)
			.map_err(|e| {
				WalletError::YubiHsmError(format!("Failed to generate asymmetric key: {e}"))
			})?;

		let signer = YubiHsmSigner::from_client(client, id)?;

		Ok(signer.into())
	}

	/// Uploads the provided keypair on the YubiHSM at the provided ID.
	pub fn from_key(
		connector: Connector,
		credentials: Credentials,
		id: object::Id,
		label: Label,
		domain: Domain,
		key: impl Into<Vec<u8>>,
	) -> Result<Self, WalletError> {
		let client = Client::open(connector, credentials, true).map_err(|e| {
			WalletError::YubiHsmError(format!("Failed to open YubiHSM client: {e}"))
		})?;

		let id = client
			.put_asymmetric_key(
				id,
				label,
				domain,
				Capability::SIGN_ECDSA,
				yubihsm::asymmetric::Algorithm::EcP256,
				key,
			)
			.map_err(|e| WalletError::YubiHsmError(format!("Failed to put asymmetric key: {e}")))?;

		let signer = YubiHsmSigner::from_client(client, id)?;

		Ok(signer.into())
	}
}

#[cfg(feature = "yubi")]
impl From<YubiHsmSigner> for WalletSigner<YubiHsmSigner> {
	fn from(signer: YubiHsmSigner) -> Self {
		let compressed = signer.public_key().to_sec1_point(true);
		let public_key_bytes = compressed.as_bytes();

		// The first byte can be either 0x02 or 0x03 for compressed public keys
		debug_assert!(public_key_bytes[0] == 0x02 || public_key_bytes[0] == 0x03);

		let address = match Secp256r1PublicKey::from_bytes(public_key_bytes) {
			Ok(secp_public_key) => public_key_to_address(&secp_public_key),
			Err(_) => {
				tracing::warn!(
					"Failed to convert YubiHSM public key to Secp256r1PublicKey, using zero address as fallback"
				);
				Address::default()
			},
		};

		Self { signer, address, network: None }
	}
}

#[cfg(test)]
mod tests {
	#[cfg(feature = "mock-hsm")]
	use std::str::FromStr;

	#[cfg(feature = "mock-hsm")]
	use super::*;
	#[cfg(feature = "mock-hsm")]
	use signature::Verifier;

	#[cfg(feature = "mock-hsm")]
	#[tokio::test]
	async fn from_key() {
		let key = hex::decode("2d8c44dc2dd2f0bea410e342885379192381e82d855b1b112f9b55544f1e0900")
			.expect("Should be able to decode valid hex");

		let connector = yubihsm::Connector::mockhsm();
		let wallet = WalletSigner::from_key(
			connector,
			Credentials::default(),
			0,
			Label::from_bytes(&[]).expect("Empty label should be valid"),
			Domain::at(1).expect("Domain 1 should be valid"),
			key,
		)
		.expect("Should be able to create wallet from key");

		let msg = "Some data";
		let sig = wallet
			.sign_message(msg.as_bytes())
			.await
			.expect("Should be able to sign message");

		let compressed = wallet.signer().public_key().to_sec1_point(true);
		let verify_key = p256::ecdsa::VerifyingKey::from_sec1_bytes(compressed.as_bytes())
			.expect("Should be able to create verifying key from public key");

		assert!(verify_key.verify(msg.as_bytes(), &sig).is_ok());

		assert_eq!(
			wallet.address(),
			Address::from_str("NPZyWCdSCWghLM7hcxT5kgc7cC2V2RGeHZ")
				.expect("Should be able to parse valid address")
		);
	}

	#[cfg(feature = "mock-hsm")]
	#[tokio::test]
	async fn new_key() {
		let connector = yubihsm::Connector::mockhsm();
		let wallet = WalletSigner::<YubiHsmSigner>::new(
			connector,
			Credentials::default(),
			0,
			Label::from_bytes(&[]).expect("Empty label should be valid"),
			Domain::at(1).expect("Domain 1 should be valid"),
		)
		.expect("Should be able to create new wallet");

		let msg = "Some data";
		let sig = wallet
			.sign_message(msg.as_bytes())
			.await
			.expect("Should be able to sign message");

		let compressed = wallet.signer().public_key().to_sec1_point(true);
		let verify_key = p256::ecdsa::VerifyingKey::from_sec1_bytes(compressed.as_bytes())
			.expect("Should be able to create verifying key from public key");

		assert!(verify_key.verify(msg.as_bytes(), &sig).is_ok());
	}

	// Non-mock tests can be added here for production hardware testing
	#[test]
	fn test_wallet_signer_creation() {
		// This test doesn't require mockhsm and can run in production builds
		// Add tests that don't require actual hardware here

		// Test that the WalletSigner type exists and has correct type parameters
		#[cfg(feature = "yubi")]
		{
			use crate::neo_wallets::WalletSigner;
			use std::marker::PhantomData;

			let _phantom: PhantomData<WalletSigner<YubiHsmSigner>> = PhantomData;
		}

		// Test that required error types exist
		use crate::neo_wallets::wallet::WalletError;
		let _error = WalletError::YubiHsmError("test".to_string());

		// Professional test validates type system without hardware dependency
	}
}
