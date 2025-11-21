use hmac::{Hmac, Mac};
use ripemd::{Digest as RipemdDigest, Ripemd160};
use sha2::{Sha256, Sha512};

pub trait HashableForVec {
	fn hash256(&self) -> Vec<u8>;
	fn ripemd160(&self) -> Vec<u8>;
	fn sha256_ripemd160(&self) -> Vec<u8>;
	fn hmac_sha512(&self, key: &[u8]) -> Vec<u8>;
}

impl HashableForVec for [u8] {
	fn hash256(&self) -> Vec<u8> {
		let mut hasher = Sha256::new();
		hasher.update(self);
		hasher.finalize().to_vec()
	}

	fn ripemd160(&self) -> Vec<u8> {
		let mut hasher = Ripemd160::new();
		hasher.update(self);
		hasher.finalize().to_vec()
	}

	fn sha256_ripemd160(&self) -> Vec<u8> {
		let mut sha256 = Sha256::new();
		sha256.update(self);
		let sha_result = sha256.finalize();

		let mut ripemd = Ripemd160::new();
		ripemd.update(sha_result);
		ripemd.finalize().to_vec()
	}

	fn hmac_sha512(&self, key: &[u8]) -> Vec<u8> {
		type HmacSha512 = Hmac<Sha512>;
		let mut mac = HmacSha512::new_from_slice(key).expect("HMAC can take key of any size");
		mac.update(self);
		mac.finalize().into_bytes().to_vec()
	}
}

impl HashableForVec for Vec<u8> {
	fn hash256(&self) -> Vec<u8> {
		self.as_slice().hash256()
	}

	fn ripemd160(&self) -> Vec<u8> {
		self.as_slice().ripemd160()
	}

	fn sha256_ripemd160(&self) -> Vec<u8> {
		self.as_slice().sha256_ripemd160()
	}

	fn hmac_sha512(&self, key: &[u8]) -> Vec<u8> {
		self.as_slice().hmac_sha512(key)
	}
}

fn hex_encode(bytes: &[u8]) -> String {
	hex::encode(bytes)
}

pub trait HashableForString {
	fn hash256(&self) -> String;
	fn ripemd160(&self) -> String;
	fn sha256_ripemd160(&self) -> String;
	fn hmac_sha512(&self, key: &str) -> String;
	fn hash160(&self) -> String;
}

impl HashableForString for String {
	fn hash256(&self) -> String {
		hex_encode(&self.as_bytes().hash256())
	}

	fn ripemd160(&self) -> String {
		hex_encode(&self.as_bytes().ripemd160())
	}

	fn sha256_ripemd160(&self) -> String {
		hex_encode(&self.as_bytes().sha256_ripemd160())
	}

	fn hmac_sha512(&self, key: &str) -> String {
		hex_encode(&self.as_bytes().hmac_sha512(key.as_bytes()))
	}

	fn hash160(&self) -> String {
		let hash = self.as_bytes().sha256_ripemd160();
		bs58::encode(&hash[..]).into_string()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_hash256_for_bytes() {
		let data = b"hello world";
		let expected = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
		let result = data.hash256();
		assert_eq!(hex_encode(&result), expected);
	}

	#[test]
	fn test_hash256_for_string() {
		let data = String::from("hello world");
		let expected = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
		assert_eq!(data.hash256(), expected);
	}

	#[test]
	fn test_ripemd160_for_bytes() {
		let data = b"hello world";
		// Use the expected hash value for "hello world" using RIPEMD160
		let expected = "98c615784ccb5fe5936fbc0cbe9dfdb408d92f0f";
		let result = data.ripemd160();
		assert_eq!(hex_encode(&result), expected);
	}

	#[test]
	fn test_ripemd160_for_string() {
		let data = String::from("hello world");
		let expected = "98c615784ccb5fe5936fbc0cbe9dfdb408d92f0f";
		assert_eq!(data.ripemd160(), expected);
	}

	#[test]
	fn test_sha256_ripemd160_for_bytes() {
		let data = b"hello world";
		// Use the expected hash value for "hello world" using SHA256 followed by RIPEMD160
		let expected = "d7d5ee7824ff93f94c3055af9382c86c68b5ca92";
		let result = data.sha256_ripemd160();
		assert_eq!(hex_encode(&result), expected);
	}

	#[test]
	fn test_sha256_ripemd160_for_string() {
		let data = String::from("hello world");
		let expected = "d7d5ee7824ff93f94c3055af9382c86c68b5ca92"; // fill this in
		assert_eq!(data.sha256_ripemd160(), expected);
	}

	#[test]
	fn test_hmac_sha512_for_bytes() {
		let data = b"hello world";
		let key = b"secret";
		// Use the expected HMAC-SHA512 value for "hello world" with key "secret"
		let expected = "6d32239b01dd1750557211629313d95e4f4fcb8ee517e443990ac1afc7562bfd74ffa6118387efd9e168ff86d1da5cef4a55edc63cc4ba289c4c3a8b4f7bdfc2";
		let result = data.hmac_sha512(key);
		assert_eq!(hex_encode(&result), expected);
	}

	#[test]
	fn test_hmac_sha512_for_string() {
		let data = String::from("hello world");
		let key = "secret";
		let expected = "6d32239b01dd1750557211629313d95e4f4fcb8ee517e443990ac1afc7562bfd74ffa6118387efd9e168ff86d1da5cef4a55edc63cc4ba289c4c3a8b4f7bdfc2";
		assert_eq!(data.hmac_sha512(key), expected);
	}

	#[test]
	fn test_hash160_for_string() {
		let data = String::from("hello world");
		// Use the expected hash value for "hello world" using SHA256 followed by RIPEMD160 and then base58 encoded
		let expected = "41QPk1SP3NZmiQxd5jY6HWh1tRcD";
		assert_eq!(data.hash160(), expected);
	}

	#[test]
	fn test_ripemd160_test_vectors() {
		let test_vectors: &[(&str, &str)] = &[
			("", "9c1185a5c5e9fc54612808977ee8f548b2258d31"),
			("a", "0bdc9d2d256b3ee9daae347be6f4dc835a467ffe"),
			("abc", "8eb208f7e05d987a9b044a8e98c6b087f15a0bfc"),
			("message digest", "5d0689ef49d2fae572b881b123a85ffa21595f36"),
			("abcdefghijklmnopqrstuvwxyz", "f71c27109c692c1b56bbdceb5b9d2865b3708dbc"),
			(
				"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
				"12a053384a9c0c88e405a06c27dcf49ada62eb2b",
			),
			(
				"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
				"b0e20b6e3116640286ed3a87a5713079b21f5189",
			),
			// For the large repeating strings, directly include them in the test
			(
				"12345678901234567890123456789012345678901234567890123456789012345678901234567890",
				"9b752e45573d4b39f4dbd3323cab82bf63326bfb",
			),
			(&"a".repeat(1_000_000), "52783243c1697bdbe16d37f97f68f08325dc1528"),
		];

		for &(input, expected_hash) in test_vectors {
			let hash = input.as_bytes().ripemd160();
			let hex_string = to_hex_string(&hash);
			assert_eq!(hex_string, expected_hash);
		}
	}

	// Helper function to convert bytes to hex string
	// Define this or replace it with your actual hex string conversion function
	fn to_hex_string(bytes: &[u8]) -> String {
		bytes.iter().map(|byte| format!("{byte:02x}")).collect()
	}
}
