//! # KeyPair
//!
//! `KeyPair` is a module that provides an implementation for Elliptic Curve Key Pairs using the `p256` crate.
//!
//! This structure can be used to manage and manipulate EC key pairs,
//! including generating new pairs, importing them from raw bytes,
//! and converting them to various formats.
//!
//! ## Security
//!
//! This type implements `ZeroizeOnDrop` to ensure that the private key is securely
//! erased from memory when the key pair is dropped.

use crate::{
	builder::VerificationScript,
	crypto::{
		CryptoError, PublicKeyExtension, Secp256r1PrivateKey, Secp256r1PublicKey,
		Secp256r1Signature, private_key_from_wif, wif_from_private_key,
	},
	neo_types::{ScriptHash, ScriptHashExtension},
};
use p256::elliptic_curve::zeroize::{Zeroize, ZeroizeOnDrop};

/// Represents an elliptic-curve key pair or a public-key-only identity.
///
/// The `Debug` impl is manually written to guarantee private key material can
/// never leak through debug formatting, even if the inner key type changes.
#[derive(Clone)]
pub struct KeyPair {
	/// The private key component of the key pair.
	private_key: Option<Secp256r1PrivateKey>,

	/// The public key component of the key pair.
	public_key: Secp256r1PublicKey,
}

impl std::fmt::Debug for KeyPair {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("KeyPair")
			.field("has_private_key", &self.private_key.is_some())
			.field("public_key", &self.public_key)
			.finish()
	}
}

impl KeyPair {
	fn missing_private_key_error() -> CryptoError {
		CryptoError::KeyError("Key pair does not contain a private key".to_string())
	}

	/// Creates a new `KeyPair` instance given a private key and its corresponding public key.
	///
	/// # Arguments
	///
	/// * `private_key` - A `Secp256r1PrivateKey` representing the private key.
	/// * `public_key` - A `Secp256r1PublicKey` representing the public key.
	pub fn new(private_key: Secp256r1PrivateKey, public_key: Secp256r1PublicKey) -> Self {
		Self { private_key: Some(private_key), public_key }
	}

	/// Returns a clone of the private key.
	///
	/// Returns an error if this is a public-key-only key pair created with
	/// [`KeyPair::from_public_key`].
	///
	/// PERFORMANCE NOTE: Consider using `private_key_ref()` to avoid cloning
	/// when you only need to read the key.
	pub fn private_key(&self) -> Result<Secp256r1PrivateKey, CryptoError> {
		self.private_key.as_ref().cloned().ok_or_else(Self::missing_private_key_error)
	}

	/// Returns a reference to the private key without cloning.
	///
	/// Returns an error if this is a public-key-only key pair created with
	/// [`KeyPair::from_public_key`].
	#[inline]
	pub fn private_key_ref(&self) -> Result<&Secp256r1PrivateKey, CryptoError> {
		self.private_key.as_ref().ok_or_else(Self::missing_private_key_error)
	}

	/// Returns a clone of the public key.
	///
	/// PERFORMANCE NOTE: Consider using `public_key_ref()` to avoid cloning
	/// when you only need to read the key.
	pub fn public_key(&self) -> Secp256r1PublicKey {
		self.public_key.clone()
	}

	/// Returns a reference to the public key without cloning.
	///
	/// Use this method when you only need to read the public key
	/// to avoid unnecessary memory allocation.
	#[inline]
	pub fn public_key_ref(&self) -> &Secp256r1PublicKey {
		&self.public_key
	}

	/// Returns true when this key pair contains a usable private key.
	#[inline]
	pub fn has_private_key(&self) -> bool {
		self.private_key.is_some()
	}

	/// Derives a new `KeyPair` instance from just a private key.
	/// The public key is derived from the given private key.
	///
	/// # Arguments
	///
	/// * `private_key` - A `Secp256r1PrivateKey` representing the private key.
	pub fn from_secret_key(private_key: &Secp256r1PrivateKey) -> Self {
		let public_key = private_key.clone().to_public_key();
		Self::new(private_key.clone(), public_key)
	}

	/// Returns the 32-byte representation of the private key.
	pub fn private_key_bytes(&self) -> Result<[u8; 32], CryptoError> {
		Ok(self.private_key_ref()?.to_raw_bytes())
	}

	/// Returns the 64-byte uncompressed representation of the public key.
	pub fn public_key_bytes(&self) -> [u8; 64] {
		let mut buf = [0u8; 64];
		let uncompressed = self.public_key.get_encoded(false);
		debug_assert_eq!(
			uncompressed.len(),
			65,
			"Expected an uncompressed secp256r1 public key to be 65 bytes (0x04 || X || Y)"
		);
		if uncompressed.len() == 65 {
			buf.copy_from_slice(&uncompressed[1..]);
		} else if uncompressed.len() == 64 {
			buf.copy_from_slice(&uncompressed);
		}

		buf
	}

	/// Signs a message using this key pair's private key.
	///
	/// Returns an error if this key pair was created via `from_public_key()`
	/// and does not contain a real private key.
	pub fn sign(&self, message: &[u8]) -> Result<Secp256r1Signature, CryptoError> {
		self.private_key_ref()?.sign_tx(message)
	}

	/// Verifies a signature against a message using this key pair's public key.
	pub fn verify(
		&self,
		message: &[u8],
		signature: &Secp256r1Signature,
	) -> Result<bool, CryptoError> {
		self.public_key.verify(message, signature).map(|_| true)
	}
}

impl KeyPair {
	/// Generates a new random `KeyPair`.
	pub fn new_random() -> Self {
		let secret_key = Secp256r1PrivateKey::new_random();
		Self::from_secret_key(&secret_key)
	}

	/// Creates an `KeyPair` from a given 32-byte private key.
	///
	/// # Arguments
	///
	/// * `private_key` - A 32-byte slice representing the private key.
	pub fn from_private_key(private_key: &[u8; 32]) -> Result<Self, CryptoError> {
		let secret_key = Secp256r1PrivateKey::from_bytes(private_key)?;
		Ok(Self::from_secret_key(&secret_key))
	}

	/// Creates an `KeyPair` from a given Wallet Import Format (WIF) string.
	/// This will use the private key encoded in the WIF to generate the key pair.
	///
	///  # Arguments
	///
	/// * `wif` - A Wallet Import Format (WIF) string.
	///
	/// The WIF string should be in the format `Kx...` or `Lx...`.
	/// The key pair will be generated from the private key encoded in the WIF.
	/// The public key will be derived from the private key.
	pub fn from_wif(wif: &str) -> Result<Self, CryptoError> {
		let private_key = private_key_from_wif(wif)?;
		Ok(Self::from_secret_key(&private_key))
	}

	/// WARNING: Creates a KeyPair with a PLACEHOLDER private key.
	/// This KeyPair CANNOT be used for signing. It exists only for
	/// public-key-only operations like address derivation.
	///
	/// # Arguments
	///
	/// * `public_key` - The 64-byte uncompressed public key (x || y, without the 0x04 prefix).
	pub fn from_public_key(public_key: &[u8; 64]) -> Result<Self, CryptoError> {
		let public_key = Secp256r1PublicKey::from_slice(public_key)?;
		Ok(Self { private_key: None, public_key })
	}

	/// Exports the key pair as a Wallet Import Format (WIF) string.
	///
	/// Returns an error when this is a public-key-only key pair created with
	/// [`KeyPair::from_public_key`].
	pub fn export_as_wif(&self) -> Result<String, CryptoError> {
		Ok(wif_from_private_key(self.private_key_ref()?))
	}

	pub fn get_script_hash(&self) -> ScriptHash {
		let vs = VerificationScript::from_public_key(self.public_key_ref());
		vs.hash()
	}

	pub fn get_address(&self) -> String {
		self.get_script_hash().to_address()
	}
}

impl PartialEq for KeyPair {
	fn eq(&self, other: &Self) -> bool {
		self.private_key == other.private_key && self.public_key == other.public_key
	}
}

// Implement Zeroize for manual zeroization if needed
impl Zeroize for KeyPair {
	fn zeroize(&mut self) {
		if let Some(private_key) = self.private_key.as_mut() {
			private_key.zeroize();
		}
		// Note: public_key is not sensitive, so we don't zeroize it
	}
}

// Automatically zeroize private key when KeyPair is dropped
impl ZeroizeOnDrop for KeyPair {}

#[cfg(test)]
mod tests {
	use crate::{
		ScriptHash, ScriptHashExtension,
		config::TestConstants,
		crypto::{CryptoError, KeyPair},
	};
	use hex;

	#[test]
	fn test_public_key_wif() {
		let private_key =
			hex::decode("c7134d6fd8e73d819e82755c64c93788d8db0961929e025a53363c4cc02a6962")
				.unwrap();
		let private_key_arr: &[u8; 32] = private_key.as_slice().try_into().unwrap();
		let key_pair = KeyPair::from_private_key(private_key_arr).unwrap();
		assert_eq!(
			key_pair.export_as_wif().unwrap(),
			"L3tgppXLgdaeqSGSFw1Go3skBiy8vQAM7YMXvTHsKQtE16PBncSU"
		);
	}

	#[test]
	fn test_public_key_only_wif_export_fails() {
		let key_pair = KeyPair::new_random();
		let public_key = key_pair.public_key_bytes();
		let watch_only = KeyPair::from_public_key(&public_key).unwrap();

		assert!(!watch_only.has_private_key());
		assert!(matches!(watch_only.private_key_ref(), Err(CryptoError::KeyError(_))));
		assert!(matches!(watch_only.private_key_bytes(), Err(CryptoError::KeyError(_))));
		assert!(matches!(watch_only.sign(b"message"), Err(CryptoError::KeyError(_))));
		assert!(matches!(watch_only.export_as_wif(), Err(CryptoError::KeyError(_))));
	}

	#[test]
	fn test_address() {
		let private_key = hex::decode(TestConstants::DEFAULT_ACCOUNT_PRIVATE_KEY).unwrap();
		let private_key_arr: &[u8; 32] = private_key.as_slice().try_into().unwrap();
		let key_pair = KeyPair::from_private_key(private_key_arr).unwrap();
		assert_eq!(key_pair.get_address(), TestConstants::DEFAULT_ACCOUNT_ADDRESS);
	}

	#[test]
	fn test_script_hash() {
		let private_key = hex::decode(TestConstants::DEFAULT_ACCOUNT_PRIVATE_KEY).unwrap();
		let private_key_arr: &[u8; 32] = private_key.as_slice().try_into().unwrap();
		let key_pair = KeyPair::from_private_key(private_key_arr).unwrap();
		assert_eq!(
			key_pair.get_script_hash(),
			ScriptHash::from_hex(TestConstants::DEFAULT_ACCOUNT_SCRIPT_HASH).unwrap()
		);
	}

	#[test]
	fn test_public_key_bytes_uncompressed_xy() {
		let private_key = hex::decode(TestConstants::DEFAULT_ACCOUNT_PRIVATE_KEY).unwrap();
		let private_key_arr: &[u8; 32] = private_key.as_slice().try_into().unwrap();
		let key_pair = KeyPair::from_private_key(private_key_arr).unwrap();

		let bytes = key_pair.public_key_bytes();
		let uncompressed = key_pair.public_key().get_encoded(false);
		assert_eq!(uncompressed.len(), 65);
		assert_eq!(&bytes[..], &uncompressed[1..]);
	}
}
