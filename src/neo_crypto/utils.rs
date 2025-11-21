use crate::crypto::{
	CryptoError, PrivateKeyExtension, PublicKeyExtension, Secp256r1PrivateKey, Secp256r1PublicKey,
};
use base64::{self, Engine};
use hex;

/// Convert a private key to a public key.
pub fn private_key_to_public_key(private_key: &Secp256r1PrivateKey) -> Secp256r1PublicKey {
	private_key.to_public_key()
}

/// Converts a private key to its hexadecimal string representation.
///
/// # Arguments
///
/// * `private_key` - The private key to convert
///
/// # Returns
///
/// A hexadecimal string representation of the private key
pub fn private_key_to_hex_string(private_key: &Secp256r1PrivateKey) -> String {
	hex::encode(private_key.to_raw_bytes())
}

/// Convert a private key in hex format to a Secp256r1PrivateKey.
///
/// # Errors
///
/// Will return an error if the hex decoding fails
pub fn private_key_from_hex(hex: &str) -> Result<Secp256r1PrivateKey, CryptoError> {
	let bytes = hex::decode(hex)?;
	let secret_key = Secp256r1PrivateKey::from_slice(&bytes)?;
	Ok(secret_key)
}

/// Converts a public key to its hexadecimal string representation.
///
/// # Arguments
///
/// * `public_key` - The public key bytes to convert
///
/// # Returns
///
/// A hexadecimal string representation of the public key
pub fn public_key_to_hex_string(public_key: &[u8]) -> String {
	hex::encode(public_key)
}

/// Convert a public key in hex format to a Secp256r1PublicKey.
///
/// # Errors
///
/// Will return an error if hex decoding fails
pub fn public_key_from_hex(hex: &str) -> Result<Secp256r1PublicKey, CryptoError> {
	let bytes = hex::decode(hex)?;
	let public_key = Secp256r1PublicKey::from_slice(&bytes)?;
	Ok(public_key)
}

pub trait ToArray32 {
	fn to_array32(&self) -> Result<[u8; 32], CryptoError>;
}

macro_rules! impl_to_array32 {
	($type:ty) => {
		impl ToArray32 for $type {
			fn to_array32(&self) -> Result<[u8; 32], CryptoError> {
				if self.len() != 32 {
					return Err(CryptoError::InvalidFormat(
						"Vector does not contain exactly 32 elements".to_string(),
					));
				}

				let mut array = [0u8; 32];
				let bytes = &self[..array.len()]; // Take a slice of the vec
				array.copy_from_slice(bytes); // Copy the slice into the array
				Ok(array)
			}
		}
	};
}

impl_to_array32!(Vec<u8>);
impl_to_array32!(&[u8]);

/// Trait to add hex encoding functionality to byte arrays and vectors
pub trait ToHexString {
	fn to_hex_string(&self) -> String;
}

impl ToHexString for [u8] {
	fn to_hex_string(&self) -> String {
		hex::encode(self)
	}
}

impl ToHexString for Vec<u8> {
	fn to_hex_string(&self) -> String {
		hex::encode(self)
	}
}

impl<const N: usize> ToHexString for [u8; N] {
	fn to_hex_string(&self) -> String {
		hex::encode(self)
	}
}

/// Trait to add hex decoding functionality to strings
pub trait FromHexString {
	fn from_hex_string(&self) -> Result<Vec<u8>, hex::FromHexError>;
}

impl FromHexString for str {
	fn from_hex_string(&self) -> Result<Vec<u8>, hex::FromHexError> {
		hex::decode(self)
	}
}

impl FromHexString for String {
	fn from_hex_string(&self) -> Result<Vec<u8>, hex::FromHexError> {
		hex::decode(self)
	}
}

/// Trait to add base64 decoding functionality to strings
pub trait FromBase64String {
	fn from_base64_string(&self) -> Result<Vec<u8>, base64::DecodeError>;
}

impl FromBase64String for str {
	fn from_base64_string(&self) -> Result<Vec<u8>, base64::DecodeError> {
		base64::engine::general_purpose::STANDARD.decode(self)
	}
}

impl FromBase64String for String {
	fn from_base64_string(&self) -> Result<Vec<u8>, base64::DecodeError> {
		base64::engine::general_purpose::STANDARD.decode(self)
	}
}

/// Trait to add base64 encoding functionality to byte slices
pub trait ToBase64String {
	fn to_base64_string(&self) -> String;
}

impl ToBase64String for [u8] {
	fn to_base64_string(&self) -> String {
		base64::engine::general_purpose::STANDARD.encode(self)
	}
}
