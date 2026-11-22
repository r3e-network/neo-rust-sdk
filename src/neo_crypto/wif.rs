use sha2::{Digest, Sha256};
// Re-export from elliptic_curve crate which is already a dependency
use p256::elliptic_curve::subtle::ConstantTimeEq;

use crate::crypto::{CryptoError, Secp256r1PrivateKey};

/// Converts a WIF (Wallet Import Format) string into a `Secp256r1PrivateKey`.
///
/// This function decodes a WIF string, verifies its format and checksum,
/// and then constructs a `Secp256r1PrivateKey` from it.
///
/// # Arguments
/// * `wif` - A string slice representing the WIF to be converted.
///
/// # Returns
/// A `Result` which is `Ok` with `Secp256r1PrivateKey` if the WIF string is valid,
/// or an `Err` with `CryptoError` if the WIF string is invalid.
///
/// # Errors
/// This function returns an error if:
/// * The WIF string is not properly base58 encoded.
/// * The decoded data does not have the correct length, prefix, or suffix expected for a WIF.
/// * The checksum of the WIF does not match the expected value.
pub fn private_key_from_wif(wif: &str) -> Result<Secp256r1PrivateKey, CryptoError> {
	let data = bs58::decode(wif)
		.into_vec()
		.map_err(|_| CryptoError::InvalidFormat("Incorrect WIF format.".to_string()))?;
	if data.len() != 38 || data[0] != 0x80 || data[33] != 0x01 {
		return Err(CryptoError::InvalidFormat("Incorrect WIF format.".to_string()));
	}

	// SECURITY: Use constant-time comparison for checksum verification
	// to prevent timing attacks that could leak information about the WIF format
	let checksum_calculated = Sha256::digest(Sha256::digest(&data[..34]));
	if checksum_calculated[..4].ct_eq(&data[34..]).unwrap_u8() != 1 {
		return Err(CryptoError::InvalidFormat("Incorrect WIF checksum.".to_string()));
	}

	Secp256r1PrivateKey::from_bytes(&data[1..33])
}

/// Converts a `Secp256r1PrivateKey` into a WIF (Wallet Import Format) string.
///
/// This function takes a `Secp256r1PrivateKey`, converts it to its raw byte representation,
/// adds the appropriate prefix and suffix, calculates the checksum, and then encodes it into WIF format.
///
/// # Arguments
/// * `private_key` - A reference to the `Secp256r1PrivateKey` to be converted.
///
/// # Returns
/// A `String` containing the WIF representation of the provided private key.
pub fn wif_from_private_key(private_key: &Secp256r1PrivateKey) -> String {
	let mut extended_key: Vec<u8> = vec![0x80];
	extended_key.extend(private_key.to_raw_bytes());
	extended_key.push(0x01);

	let hash = Sha256::digest(Sha256::digest(&extended_key));
	let checksum = &hash[0..4];
	extended_key.extend_from_slice(checksum);

	bs58::encode(extended_key).into_string()
}

#[cfg(test)]
mod tests {
	use crate::crypto::{
		PrivateKeyExtension, Secp256r1PrivateKey, private_key_from_wif, wif_from_private_key,
	};

	#[test]
	fn test_valid_wif_to_private_key() {
		let wif = "L25kgAQJXNHnhc7Sx9bomxxwVSMsZdkaNQ3m2VfHrnLzKWMLP13A";
		let expected_key =
			hex::decode("9117f4bf9be717c9a90994326897f4243503accd06712162267e77f18b49c3a3")
				.unwrap();

		let key = private_key_from_wif(wif).unwrap().to_raw_bytes().to_vec();

		assert_eq!(key, expected_key);
	}

	#[test]
	fn test_invalid_wif_sizes() {
		let too_long = "L25kgAQJXNHnhc7Sx9bomxxwVSMsZdkaNQ3m2VfHrnLzKWMLP13Ahc7S";
		let too_short = "L25kgAQJXNHnhc7Sx9bomxxwVSMsZdkaNQ3m2VfHrnLzKWML";

		assert!(private_key_from_wif(too_long).is_err());
		assert!(private_key_from_wif(too_short).is_err());
	}

	#[test]
	fn test_invalid_wif_bytes() {
		let wif = "L25kgAQJXNHnhc7Sx9bomxxwVSMsZdkaNQ3m2VfHrnLzKWMLP13A";
		let _expected_key =
			hex::decode("9117f4bf9be717c9a90994326897f4243503accd06712162267e77f18b49c3a3")
				.unwrap();

		let mut decoded = bs58::decode(wif).into_vec().unwrap();
		decoded[0] = 0x81;
		let invalid_first = bs58::encode(&decoded).into_string();

		decoded[33] = 0;
		let invalid_33rd = bs58::encode(&decoded).into_string();

		assert!(private_key_from_wif(invalid_first.as_str()).is_err());
		assert!(private_key_from_wif(invalid_33rd.as_str()).is_err());
	}

	#[test]
	fn test_valid_private_key_to_wif() {
		let pk = hex::decode("9117f4bf9be717c9a90994326897f4243503accd06712162267e77f18b49c3a3")
			.unwrap();
		let expected_wif = "L25kgAQJXNHnhc7Sx9bomxxwVSMsZdkaNQ3m2VfHrnLzKWMLP13A";

		let wif = wif_from_private_key(&Secp256r1PrivateKey::from_slice(&pk).unwrap());

		assert_eq!(wif, expected_wif);
	}

	#[test]
	fn test_invalid_private_key_length() {
		let invalid_len =
			hex::decode("9117f4bf9be717c9a90994326897f4243503accd06712162267e77f18b49c3").unwrap();

		// wif_from_private_key(&
		assert!(Secp256r1PrivateKey::from_slice(&invalid_len).is_err());
	}

	// Edge case tests for WIF security
	#[test]
	fn test_wif_empty_string() {
		assert!(private_key_from_wif("").is_err());
	}

	#[test]
	fn test_wif_invalid_base58_characters() {
		// Base58 doesn't include '0', 'O', 'I', 'l'
		assert!(
			private_key_from_wif("0000000000000000000000000000000000000000000000000000").is_err()
		);
		assert!(
			private_key_from_wif("OOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOO").is_err()
		);
		assert!(
			private_key_from_wif("IIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIII").is_err()
		);
		assert!(
			private_key_from_wif("llllllllllllllllllllllllllllllllllllllllllllllllllll").is_err()
		);
	}

	#[test]
	fn test_wif_corrupted_checksum() {
		let valid_wif = "L25kgAQJXNHnhc7Sx9bomxxwVSMsZdkaNQ3m2VfHrnLzKWMLP13A";
		let mut decoded = bs58::decode(valid_wif).into_vec().unwrap();

		// Corrupt the last byte (part of checksum)
		decoded[37] ^= 0xFF;
		let corrupted = bs58::encode(&decoded).into_string();

		let result = private_key_from_wif(&corrupted);
		assert!(result.is_err());
		assert!(result.unwrap_err().to_string().contains("checksum"));
	}

	#[test]
	fn test_wif_roundtrip() {
		// Test that WIF encoding and decoding are inverse operations
		let original_key =
			hex::decode("9117f4bf9be717c9a90994326897f4243503accd06712162267e77f18b49c3a3")
				.unwrap();
		let private_key = Secp256r1PrivateKey::from_slice(&original_key).unwrap();

		let wif = wif_from_private_key(&private_key);
		let recovered_key = private_key_from_wif(&wif).unwrap();

		assert_eq!(private_key.to_raw_bytes(), recovered_key.to_raw_bytes());
	}

	#[test]
	fn test_wif_different_keys_produce_different_wifs() {
		let key1 = Secp256r1PrivateKey::from_slice(
			&hex::decode("9117f4bf9be717c9a90994326897f4243503accd06712162267e77f18b49c3a3")
				.unwrap(),
		)
		.unwrap();
		let key2 = Secp256r1PrivateKey::from_slice(
			&hex::decode("c7134d6fd8e73d819e82755c64c93788d8db0961929e025a53363c4cc02a6962")
				.unwrap(),
		)
		.unwrap();

		let wif1 = wif_from_private_key(&key1);
		let wif2 = wif_from_private_key(&key2);

		assert_ne!(wif1, wif2);
	}
}
