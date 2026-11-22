pub struct NeoConstants {}
impl NeoConstants {
	// Network Magic Numbers
	pub const MAGIC_NUMBER_MAINNET: u32 = 860833102;
	pub const MAGIC_NUMBER_TESTNET: u32 = 894710606;

	// Accounts, Addresses, Keys
	pub const MAX_PUBLIC_KEYS_PER_MULTI_SIG: u32 = 1024;
	pub const HASH160_SIZE: u32 = 20;
	pub const HASH256_SIZE: u32 = 32;
	pub const PRIVATE_KEY_SIZE: u32 = 32;
	pub const PUBLIC_KEY_SIZE_COMPRESSED: u32 = 33;
	pub const SIGNATURE_SIZE: u32 = 64;
	pub const VERIFICATION_SCRIPT_SIZE: u32 = 40;
	pub const MAX_ITERATOR_ITEMS_DEFAULT: u32 = 100;

	pub const MAX_SUBITEMS: u32 = 16;
	pub const MAX_NESTING_DEPTH: u8 = 2;

	// Transactions & Contracts
	pub const CURRENT_TX_VERSION: u8 = 0;
	pub const MAX_TRANSACTION_SIZE: u32 = 102400;
	pub const MAX_TRANSACTION_ATTRIBUTES: u32 = 16;
	pub const MAX_SIGNER_SUBITEMS: u32 = 16;
	pub const MAX_MANIFEST_SIZE: u32 = 0xFFFF;

	// RPC / Transport
	// Upper bound for a single JSON-RPC message payload (bytes).
	//
	// This helps prevent unbounded memory growth when interacting with an
	// untrusted or misbehaving node.
	pub const MAX_RPC_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

	/// Returns the maximum JSON-RPC message size to accept (bytes).
	///
	/// By default, this is [`Self::MAX_RPC_MESSAGE_SIZE`]. On non-SGX builds you can override it
	/// at runtime by setting the `NEO3_MAX_RPC_MESSAGE_SIZE` environment variable to a positive
	/// integer.
	#[cfg(not(all(feature = "sgx", target_env = "sgx")))]
	pub fn max_rpc_message_size() -> usize {
		std::env::var("NEO3_MAX_RPC_MESSAGE_SIZE")
			.ok()
			.and_then(|v| v.parse::<usize>().ok())
			.filter(|&v| v > 0)
			.unwrap_or(Self::MAX_RPC_MESSAGE_SIZE)
	}

	#[cfg(all(feature = "sgx", target_env = "sgx"))]
	pub fn max_rpc_message_size() -> usize {
		// SGX/no_std builds don't have a reliable notion of process environment variables.
		Self::MAX_RPC_MESSAGE_SIZE
	}

	/// Returns an optional per-request timeout for JSON-RPC operations.
	///
	/// When set, transports may abort requests that take longer than the configured duration.
	/// By default this returns `None` (no timeout). On non-SGX builds you can override it at
	/// runtime by setting the `NEO3_RPC_TIMEOUT_SECS` environment variable to a positive integer.
	#[cfg(not(all(feature = "sgx", target_env = "sgx")))]
	pub fn rpc_request_timeout() -> Option<core::time::Duration> {
		std::env::var("NEO3_RPC_TIMEOUT_SECS")
			.ok()
			.and_then(|v| v.parse::<u64>().ok())
			.filter(|&v| v > 0)
			.map(core::time::Duration::from_secs)
	}

	#[cfg(all(feature = "sgx", target_env = "sgx"))]
	pub fn rpc_request_timeout() -> Option<core::time::Duration> {
		// SGX/no_std builds don't have a reliable notion of process environment variables.
		None
	}

	// pub const DEFAULT_SCRYPT_PARAMS: Params = Params::new(14, 8, 8, 32).unwrap();

	pub const SEED_1: &'static str = "http://seed1.neo.org:10332";
	pub const SEED_2: &'static str = "http://seed2.neo.org:10332";
	pub const SEED_3: &'static str = "http://seed3.neo.org:10332";
	pub const SEED_4: &'static str = "http://seed4.neo.org:10332";
	pub const SEED_5: &'static str = "http://seed5.neo.org:10332";

	pub const SCRYPT_N: usize = 16384;
	pub const SCRYPT_R: u32 = 8;
	pub const SCRYPT_P: u32 = 8;
	pub const SCRYPT_LOG_N: u8 = 14;
	pub const SCRYPT_DK_LEN: usize = 64;

	pub const NEP_HEADER_1: u8 = 0x01;
	pub const NEP_HEADER_2: u8 = 0x42;
	pub const NEP_FLAG: u8 = 0xe0;

	pub fn new() -> Self {
		Self {}
	}
}

impl Default for NeoConstants {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::NeoConstants;
	use serial_test::serial;

	fn with_env_var(key: &str, value: Option<&str>, f: impl FnOnce()) {
		let prev = std::env::var(key).ok();
		// SAFETY: test-only helper. Environment mutation happens exclusively
		// under `#[serial]`-guarded tests, so no other thread reads the
		// environment concurrently.
		match value {
			Some(v) => unsafe { std::env::set_var(key, v) },
			None => unsafe { std::env::remove_var(key) },
		}
		f();
		match prev {
			Some(v) => unsafe { std::env::set_var(key, v) },
			None => unsafe { std::env::remove_var(key) },
		}
	}

	#[test]
	#[serial]
	fn rpc_request_timeout_parses_env_var() {
		with_env_var("NEO3_RPC_TIMEOUT_SECS", None, || {
			assert_eq!(NeoConstants::rpc_request_timeout(), None);
		});

		with_env_var("NEO3_RPC_TIMEOUT_SECS", Some("5"), || {
			assert_eq!(
				NeoConstants::rpc_request_timeout(),
				Some(core::time::Duration::from_secs(5))
			);
		});

		with_env_var("NEO3_RPC_TIMEOUT_SECS", Some("0"), || {
			assert_eq!(NeoConstants::rpc_request_timeout(), None);
		});

		with_env_var("NEO3_RPC_TIMEOUT_SECS", Some("not-a-number"), || {
			assert_eq!(NeoConstants::rpc_request_timeout(), None);
		});
	}
}
