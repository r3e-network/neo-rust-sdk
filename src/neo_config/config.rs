use lazy_static::lazy_static;
use primitive_types::H160;
use serde::{Deserialize, Serialize};
use std::{
	collections::HashMap,
	hash::{Hash, Hasher},
	sync::{Arc, Mutex},
};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum NeoNetwork {
	MainNet = 0x334f454e,
	TestNet = 0x74746e41,
	PrivateNet = 0x4e454e,
}

impl NeoNetwork {
	pub fn to_magic(&self) -> u32 {
		match self {
			NeoNetwork::MainNet => 0x334f454e,
			NeoNetwork::TestNet => 0x74746e41,
			NeoNetwork::PrivateNet => 0x4e454e,
		}
	}

	pub fn from_magic(magic: u32) -> Option<NeoNetwork> {
		match magic {
			0x334f454e => Some(NeoNetwork::MainNet),
			0x74746e41 => Some(NeoNetwork::TestNet),
			0x4e454e => Some(NeoNetwork::PrivateNet),
			_ => None,
		}
	}
}

pub const DEFAULT_BLOCK_TIME: u64 = 15_000;
pub const DEFAULT_ADDRESS_VERSION: u8 = 0x35;
pub const MAX_VALID_UNTIL_BLOCK_INCREMENT_BASE: u64 = 86_400_000;

#[derive(Clone, Debug, Deserialize)]
pub struct NeoConfig {
	pub network: Option<u32>,
	pub address_version: u8,
	pub milliseconds_per_block: u32,
	pub max_transactions_per_block: u32,
	pub memory_pool_max_transactions: u32,
	pub max_traceable_blocks: u32,
	pub hardforks: HashMap<String, u32>,
	pub initial_gas_distribution: u64,
	pub validators_count: u32,
	pub standby_committee: Vec<String>,
	pub seed_list: Vec<String>,
	pub nns_resolver: H160,
	#[serde(skip)]
	pub allows_transmission_on_fault: bool,
}

lazy_static! {
	pub static ref NEOCONFIG: Mutex<NeoConfig> = Mutex::new(NeoConfig::default());
}

impl Hash for NeoConfig {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.network.hash(state);
		self.milliseconds_per_block.hash(state);
		self.max_transactions_per_block.hash(state);
		self.memory_pool_max_transactions.hash(state);
		self.max_traceable_blocks.hash(state);
		self.initial_gas_distribution.hash(state);
		self.validators_count.hash(state);
		self.nns_resolver.hash(state);
	}
}

impl Default for NeoConfig {
	fn default() -> Self {
		let mut hardforks = HashMap::new();
		hardforks.insert("HF_Aspidochelone".to_string(), 1730000);
		hardforks.insert("HF_Basilisk".to_string(), 4120000);
		hardforks.insert("HF_Cockatrice".to_string(), 5450000);
		hardforks.insert("HF_Domovoi".to_string(), 5570000);

		NeoConfig {
			network: Some(860833102), // TestNet
			address_version: 53,
			milliseconds_per_block: 15000,
			max_transactions_per_block: 512,
			memory_pool_max_transactions: 50000,
			max_traceable_blocks: 2102400,
			hardforks,
			initial_gas_distribution: 5200000000000000,
			validators_count: 7,
			standby_committee: vec![
				"03b209fd4f53a7170ea4444e0cb0a6bb6a53c2bd016926989cf85f9b0fba17a70c".to_string(),
				"02df48f60e8f3e01c48ff40b9b7f1310d7a8b2a193188befe1c2e3df740e895093".to_string(),
				"03b8d9d5771d8f513aa0869b9cc8d50986403b78c6da36890638c3d46a5adce04a".to_string(),
				"02ca0e27697b9c248f6f16e085fd0061e26f44da85b58ee835c110caa5ec3ba554".to_string(),
				"024c7b7fb6c310fccf1ba33b082519d82964ea93868d676662d4a59ad548df0e7d".to_string(),
				"02aaec38470f6aad0042c6e877cfd8087d2676b0f516fddd362801b9bd3936399e".to_string(),
				"02486fd15702c4490a26703112a5cc1d0923fd697a33406bd5a1c00e0013b09a70".to_string(),
				"023a36c72844610b4d34d1968662424011bf783ca9d984efa19a20babf5582f3fe".to_string(),
				"03708b860c1de5d87f5b151a12c2a99feebd2e8b315ee8e7cf8aa19692a9e18379".to_string(),
				"03c6aa6e12638b36e88adc1ccdceac4db9929575c3e03576c617c49cce7114a050".to_string(),
				"03204223f8c86b8cd5c89ef12e4f0dbb314172e9241e30c9ef2293790793537cf0".to_string(),
				"02a62c915cf19c7f19a50ec217e79fac2439bbaad658493de0c7d8ffa92ab0aa62".to_string(),
				"03409f31f0d66bdc2f70a9730b66fe186658f84a8018204db01c106edc36553cd0".to_string(),
				"0288342b141c30dc8ffcde0204929bb46aed5756b41ef4a56778d15ada8f0c6654".to_string(),
				"020f2887f41474cfeb11fd262e982051c1541418137c02a0f4961af911045de639".to_string(),
				"0222038884bbd1d8ff109ed3bdef3542e768eef76c1247aea8bc8171f532928c30".to_string(),
				"03d281b42002647f0113f36c7b8efb30db66078dfaaa9ab3ff76d043a98d512fde".to_string(),
				"02504acbc1f4b3bdad1d86d6e1a08603771db135a73e61c9d565ae06a1938cd2ad".to_string(),
				"0226933336f1b75baa42d42b71d9091508b638046d19abd67f4e119bf64a7cfb4d".to_string(),
				"03cdcea66032b82f5c30450e381e5295cae85c5e6943af716cc6b646352a6067dc".to_string(),
				"02cd5a5547119e24feaa7c2a0f37b8c9366216bab7054de0065c9be42084003c8a".to_string(),
			],
			seed_list: vec![
				"seed1.neo.org:10333".to_string(),
				"seed2.neo.org:10333".to_string(),
				"seed3.neo.org:10333".to_string(),
				"seed4.neo.org:10333".to_string(),
				"seed5.neo.org:10333".to_string(),
			],
			nns_resolver: H160::from_slice(
				&hex::decode("50ac1c37690cc2cfc594472833cf57505d5f46de")
					.expect("Static hex string for NNS resolver should always be valid"),
			),
			allows_transmission_on_fault: false,
		}
	}
}

impl NeoConfig {
	pub fn new(json_config: &str) -> Result<Self, serde_json::Error> {
		let mut config: NeoConfig = serde_json::from_str(json_config)?;
		config.allows_transmission_on_fault = false;
		Ok(config)
	}

	pub fn set_network(&mut self, magic: u32) -> Result<(), &'static str> {
		// u32 can't exceed 0xFFFFFFFF, so this check is redundant
		// Keeping the function signature for API compatibility

		self.network = Some(magic);
		Ok(())
	}

	pub fn get_max_valid_until_block_increment(&self) -> u32 {
		(MAX_VALID_UNTIL_BLOCK_INCREMENT_BASE / self.milliseconds_per_block as u64) as u32
	}

	pub fn mainnet() -> Self {
		let mut hardforks = HashMap::new();
		hardforks.insert("HF_Aspidochelone".to_string(), 1730000);
		hardforks.insert("HF_Basilisk".to_string(), 4120000);
		hardforks.insert("HF_Cockatrice".to_string(), 5450000);
		hardforks.insert("HF_Domovoi".to_string(), 5570000);

		NeoConfig {
			network: Some(860833102),
			address_version: 53,
			milliseconds_per_block: 15000,
			max_transactions_per_block: 512,
			memory_pool_max_transactions: 50000,
			max_traceable_blocks: 2102400,
			hardforks,
			initial_gas_distribution: 5200000000000000,
			validators_count: 7,
			standby_committee: vec![
				"03b209fd4f53a7170ea4444e0cb0a6bb6a53c2bd016926989cf85f9b0fba17a70c".to_string(),
				"02df48f60e8f3e01c48ff40b9b7f1310d7a8b2a193188befe1c2e3df740e895093".to_string(),
				"03b8d9d5771d8f513aa0869b9cc8d50986403b78c6da36890638c3d46a5adce04a".to_string(),
				"02ca0e27697b9c248f6f16e085fd0061e26f44da85b58ee835c110caa5ec3ba554".to_string(),
				"024c7b7fb6c310fccf1ba33b082519d82964ea93868d676662d4a59ad548df0e7d".to_string(),
				"02aaec38470f6aad0042c6e877cfd8087d2676b0f516fddd362801b9bd3936399e".to_string(),
				"02486fd15702c4490a26703112a5cc1d0923fd697a33406bd5a1c00e0013b09a70".to_string(),
				"023a36c72844610b4d34d1968662424011bf783ca9d984efa19a20babf5582f3fe".to_string(),
				"03708b860c1de5d87f5b151a12c2a99feebd2e8b315ee8e7cf8aa19692a9e18379".to_string(),
				"03c6aa6e12638b36e88adc1ccdceac4db9929575c3e03576c617c49cce7114a050".to_string(),
				"03204223f8c86b8cd5c89ef12e4f0dbb314172e9241e30c9ef2293790793537cf0".to_string(),
				"02a62c915cf19c7f19a50ec217e79fac2439bbaad658493de0c7d8ffa92ab0aa62".to_string(),
				"03409f31f0d66bdc2f70a9730b66fe186658f84a8018204db01c106edc36553cd0".to_string(),
				"0288342b141c30dc8ffcde0204929bb46aed5756b41ef4a56778d15ada8f0c6654".to_string(),
				"020f2887f41474cfeb11fd262e982051c1541418137c02a0f4961af911045de639".to_string(),
				"0222038884bbd1d8ff109ed3bdef3542e768eef76c1247aea8bc8171f532928c30".to_string(),
				"03d281b42002647f0113f36c7b8efb30db66078dfaaa9ab3ff76d043a98d512fde".to_string(),
				"02504acbc1f4b3bdad1d86d6e1a08603771db135a73e61c9d565ae06a1938cd2ad".to_string(),
				"0226933336f1b75baa42d42b71d9091508b638046d19abd67f4e119bf64a7cfb4d".to_string(),
				"03cdcea66032b82f5c30450e381e5295cae85c5e6943af716cc6b646352a6067dc".to_string(),
				"02cd5a5547119e24feaa7c2a0f37b8c9366216bab7054de0065c9be42084003c8a".to_string(),
			],
			seed_list: vec![
				"seed1.neo.org:10333".to_string(),
				"seed2.neo.org:10333".to_string(),
				"seed3.neo.org:10333".to_string(),
				"seed4.neo.org:10333".to_string(),
				"seed5.neo.org:10333".to_string(),
			],
			nns_resolver: H160::from_slice(
				&hex::decode("50ac1c37690cc2cfc594472833cf57505d5f46de")
					.expect("Static hex string for NNS resolver should always be valid"),
			),
			allows_transmission_on_fault: false,
		}
	}
}

#[derive(Clone, Debug)]
pub struct Counter {
	count: Arc<Mutex<u32>>,
}

impl Hash for Counter {
	fn hash<H: Hasher>(&self, state: &mut H) {
		if let Ok(count) = self.count.lock() {
			count.hash(state);
		} else {
			// If we can't lock the mutex, hash a default value
			0u32.hash(state);
		}
	}
}

impl PartialEq for Counter {
	fn eq(&self, other: &Self) -> bool {
		match (self.count.lock(), other.count.lock()) {
			(Ok(self_count), Ok(other_count)) => *self_count == *other_count,
			_ => false, // If we can't lock either mutex, consider them not equal
		}
	}
}

impl Counter {
	pub fn new() -> Self {
		Counter { count: Arc::new(Mutex::new(1)) }
	}

	pub fn get_and_increment(&self) -> u32 {
		if let Ok(mut count) = self.count.lock() {
			let v: u32 = *count;
			*count += 1;
			v
		} else {
			// If we can't lock the mutex, return a default value
			// This is a fallback to avoid panicking
			1
		}
	}
}
