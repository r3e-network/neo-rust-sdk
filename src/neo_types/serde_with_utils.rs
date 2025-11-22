#![allow(unused_imports)]
#![allow(dead_code)]

use std::{
	collections::{HashMap, HashSet},
	convert::TryInto,
	fmt,
};

use elliptic_curve::sec1::ToEncodedPoint;
use hex;
use primitive_types::{H160, H256, U256};
use reqwest::Url;
use serde::{
	de::{self, SeqAccess, Visitor},
	ser::{SerializeMap, SerializeSeq},
	Deserialize, Deserializer, Serialize, Serializer,
};

use crate::{
	builder::WitnessScope,
	crypto::{Secp256r1PrivateKey, Secp256r1PublicKey},
	neo_protocol::HardForks,
	neo_types::{
		encode_string_h160, encode_string_h256, encode_string_u256, parse_address,
		parse_string_h160, parse_string_h256, parse_string_u256, parse_string_u64,
	},
	Address, AddressOrScriptHash, ContractParameter, ScriptHash, ScriptHashExtension,
};
pub fn serialize_h160_without_0x<S>(h160: &H160, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	let hex_str = format!("{:x}", h160);
	serializer.serialize_str(&hex_str)
}

pub fn serialize_h160<S>(item: &H160, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	serializer.serialize_str(&encode_string_h160(item))
}

pub fn deserialize_h160<'de, D>(deserializer: D) -> Result<H160, D::Error>
where
	D: Deserializer<'de>,
{
	let s: String = Deserialize::deserialize(deserializer)?;
	parse_string_h160(&s).map_err(|e| {
		serde::de::Error::custom(format!("Failed to parse H160 from string '{}': {}", s, e))
	})
}

pub fn serialize_scopes<S>(scopes: &[WitnessScope], serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	let scopes_str = scopes
		.iter()
		.map(ToString::to_string) // Using strum's ToString implementation
		.collect::<Vec<String>>()
		.join(",");
	serializer.serialize_str(&scopes_str)
}

pub fn deserialize_scopes<'de, D>(deserializer: D) -> Result<Vec<WitnessScope>, D::Error>
where
	D: Deserializer<'de>,
{
	let s: String = Deserialize::deserialize(deserializer)?;
	let scopes = s
		.split(",")
		.map(|x| {
			x.trim().parse().map_err(|e| {
				serde::de::Error::custom(format!("Failed to parse scope '{}': {}", x.trim(), e))
			})
		})
		.collect::<Result<Vec<WitnessScope>, _>>()?;

	Ok(scopes)
}

pub fn serialize_boolean_expression<S>(value: &bool, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	serializer.serialize_str(if *value { "true" } else { "false" })
}

pub fn deserialize_boolean_expression<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
	D: Deserializer<'de>,
{
	let s = String::deserialize(deserializer)?;
	let value = s == "true";
	Ok(value)
}

pub fn serialize_bytes<S>(item: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	let item_str = format!("0x{}", hex::encode(item));
	serializer.serialize_str(&item_str)
}

pub fn deserialize_bytes<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
	D: Deserializer<'de>,
{
	let s: String = Deserialize::deserialize(deserializer)?;
	let bytes = hex::decode(s.trim_start_matches("0x"))
		.map_err(|e| serde::de::Error::custom(format!("Failed to decode hex string: {}", e)))?;
	Ok(bytes)
}

pub fn serialize_url<S>(item: Url, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	// deserialize_script_hash
	let item_str = format!("{}", item);
	serializer.serialize_str(&item_str)
}

pub fn deserialize_pubkey<'de, D>(deserializer: D) -> Result<Secp256r1PublicKey, D::Error>
where
	D: Deserializer<'de>,
{
	let a: &[u8] = Deserialize::deserialize(deserializer)?;
	Secp256r1PublicKey::from_bytes(a).map_err(serde::de::Error::custom)
}

pub fn serialize_pubkey<S>(item: Secp256r1PublicKey, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	let item_str = format!("{:?}", item.get_encoded(true));
	serializer.serialize_str(&item_str)
}

pub fn deserialize_url<'de, D>(deserializer: D) -> Result<Url, D::Error>
where
	D: Deserializer<'de>,
{
	let s: String = Deserialize::deserialize(deserializer)?;
	let url = Url::parse(&s)
		.map_err(|e| serde::de::Error::custom(format!("Failed to parse URL '{}': {}", s, e)))?;
	Ok(url)
}

pub fn serialize_url_option<S>(item: &Option<Url>, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	match item {
		Some(url) => {
			let url_str = format!("{}", url);
			serializer.serialize_str(&url_str)
		},
		None => serializer.serialize_none(),
	}
}

pub fn deserialize_url_option<'de, D>(deserializer: D) -> Result<Option<Url>, D::Error>
where
	D: Deserializer<'de>,
{
	let s: Option<String> = Deserialize::deserialize(deserializer)?;
	match s {
		Some(s) => {
			let url = Url::parse(&s).map_err(|e| {
				serde::de::Error::custom(format!("Failed to parse URL '{}': {}", s, e))
			})?;
			Ok(Some(url))
		},
		None => Ok(None),
	}
}

// pub fn serialize_wildcard<S>(value: &Vec<String>, serializer: S) -> Result<S::Ok, S::Error>
// where
// 	S: Serializer,
// {
// 	if value == &vec!["*".to_string()] {
// 		serializer.serialize_str("*")
// 	} else {
// 		value.serialize(serializer)
// 	}
// }

// pub fn deserialize_wildcard<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
// where
// 	D: Deserializer<'de>,
// {
// 	let s: String = Deserialize::deserialize(deserializer)?;
// 	if s == "*" {
// 		Ok(vec!["*".to_string()])
// 	} else {
// 		Ok(vec![s])
// 	}
// }

pub fn serialize_u256<S>(item: &U256, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	let item_str = format!("{}", item);
	serializer.serialize_str(&item_str)
}

pub fn deserialize_u256<'de, D>(deserializer: D) -> Result<U256, D::Error>
where
	D: Deserializer<'de>,
{
	let s: String = Deserialize::deserialize(deserializer)?;
	parse_string_u256(&s).map_err(|e| {
		serde::de::Error::custom(format!("Failed to parse U256 from string '{}': {}", s, e))
	})
}

pub fn serialize_u256_option<S>(item: &Option<U256>, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	match item {
		Some(u256) => {
			let u256_str = encode_string_u256(u256);
			serializer.serialize_str(&u256_str)
		},
		None => serializer.serialize_none(),
	}
}

pub fn deserialize_u256_option<'de, D>(deserializer: D) -> Result<Option<U256>, D::Error>
where
	D: Deserializer<'de>,
{
	let s: Option<String> = Deserialize::deserialize(deserializer)?;
	match s {
		Some(s) => {
			let u256 = parse_string_u256(&s).map_err(|e| {
				serde::de::Error::custom(format!("Failed to parse U256 from string '{}': {}", s, e))
			})?;
			Ok(Some(u256))
		},
		None => Ok(None),
	}
}

pub fn serialize_u32<S>(item: &u32, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	let item_str = format!("0x{:x}", item);
	serializer.serialize_str(&item_str)
}

pub fn deserialize_u32<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
	D: Deserializer<'de>,
{
	let s: String = Deserialize::deserialize(deserializer)?;
	let v = if s.starts_with("0x") {
		let trimmed = s.trim_start_matches("0x");
		u32::from_str_radix(trimmed, 16).map_err(|e| {
			serde::de::Error::custom(format!("Failed to parse hex u32 '{}': {}", s, e))
		})?
	} else {
		s.parse::<u32>().map_err(|e| {
			serde::de::Error::custom(format!("Failed to parse decimal u32 '{}': {}", s, e))
		})?
	};
	Ok(v)
}

pub fn serialize_u64<S>(item: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	serializer.serialize_str(&item.to_string())
}

pub fn deserialize_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
	D: Deserializer<'de>,
{
	let s: String = Deserialize::deserialize(deserializer)?;
	parse_string_u64(&s).map_err(|e| {
		serde::de::Error::custom(format!("Failed to parse u64 from string '{}': {}", s, e))
	})
}

pub fn serialize_u64_option<S>(item: &Option<u64>, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	match item {
		Some(value) => serialize_u64(value, serializer),
		None => serializer.serialize_none(),
	}
}

pub fn deserialize_u64_option<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
	D: Deserializer<'de>,
{
	let opt: Option<String> = Option::deserialize(deserializer)?;
	match opt {
		Some(s) => parse_string_u64(&s).map(Some).map_err(|e| {
			serde::de::Error::custom(format!("Failed to parse u64 from '{}': {}", s, e))
		}),
		None => Ok(None),
	}
}

pub fn deserialize_script_hash<'de, D>(deserializer: D) -> Result<ScriptHash, D::Error>
where
	D: Deserializer<'de>,
{
	let s: String = Deserialize::deserialize(deserializer)?;
	parse_address(&s).map_err(|e| {
		serde::de::Error::custom(format!("Failed to parse address from string '{}': {}", s, e))
	})
}

pub fn serialize_script_hash<S>(item: &ScriptHash, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	// let item_str = encode_string_h160(item);
	let binding = encode_string_h160(item);
	let item_str = binding.trim_start_matches("0x");
	serializer.serialize_str(item_str)
}

pub fn deserialize_address_or_script_hash<'de, D>(
	deserializer: D,
) -> Result<AddressOrScriptHash, D::Error>
where
	D: Deserializer<'de>,
{
	let s: String = Deserialize::deserialize(deserializer)?;
	let addr = parse_address(&s).map_err(|e| {
		serde::de::Error::custom(format!("Failed to parse address from string '{}': {}", s, e))
	})?;
	Ok(AddressOrScriptHash::ScriptHash(addr))
}

pub fn serialize_address_or_script_hash<S>(
	item: &AddressOrScriptHash,
	serializer: S,
) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	let item_str = encode_string_h160(&item.script_hash());
	serializer.serialize_str(&item_str)
}

pub fn deserialize_vec_script_hash<'de, D>(deserializer: D) -> Result<Vec<ScriptHash>, D::Error>
where
	D: Deserializer<'de>,
{
	let string_seq = <Vec<ScriptHash>>::deserialize(deserializer)?;
	// let mut vec: Vec<Address> = Vec::new();
	// for v_str in string_seq {
	// 	let v = parse_address(&v_str);
	// 	vec.push(v);
	// }
	Ok(string_seq)
}

pub fn serialize_vec_script_hash<S>(
	item: &Vec<ScriptHash>,
	serializer: S,
) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	let mut seq = serializer.serialize_seq(Some(item.len()))?;
	for i in item {
		seq.serialize_element(&i.to_hex())?;
	}
	seq.end()
}

pub fn deserialize_vec_script_hash_option<'de, D>(
	deserializer: D,
) -> Result<Option<Vec<ScriptHash>>, D::Error>
where
	D: Deserializer<'de>,
{
	let string_seq = <Option<Vec<ScriptHash>>>::deserialize(deserializer)?;
	// let mut vec: Vec<Address> = Vec::new();
	// for v_str in string_seq {
	// 	let v = parse_address(&v_str);
	// 	vec.push(v);
	// }
	match string_seq {
		Some(s) => Ok(Some(s)),
		None => Ok(None),
	}
}

pub fn serialize_vec_script_hash_option<S>(
	item: &Option<Vec<ScriptHash>>,
	serializer: S,
) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	match item {
		Some(addr) => {
			let mut seq = serializer.serialize_seq(Some(addr.len()))?;
			for i in addr {
				seq.serialize_element(i.to_hex().trim_start_matches("0x"))?;
			}
			seq.end()
		},
		None => serializer.serialize_none(),
	}
}

pub fn serialize_script_hash_option<S>(
	item: &Option<ScriptHash>,
	serializer: S,
) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	match item {
		Some(addr) => {
			let addr_str = encode_string_h160(addr);
			serializer.serialize_str(addr_str.as_str())
		},
		None => serializer.serialize_none(),
	}
}

pub fn deserialize_script_hash_option<'de, D>(
	deserializer: D,
) -> Result<Option<ScriptHash>, D::Error>
where
	D: Deserializer<'de>,
{
	let s: Option<String> = Deserialize::deserialize(deserializer)?;
	match s {
		Some(s) => {
			let addr = parse_address(&s).map_err(|e| {
				serde::de::Error::custom(format!(
					"Failed to parse address from string '{}': {}",
					s, e
				))
			})?;
			Ok(Some(addr))
		},
		None => Ok(None),
	}
}

pub fn serialize_hash_map_h160_account<S, Account>(
	item: &HashMap<H160, Account>,
	serializer: S,
) -> Result<S::Ok, S::Error>
where
	S: Serializer,
	Account: Serialize,
{
	let mut map = serializer.serialize_map(Some(item.len()))?;
	for (k, v) in item {
		map.serialize_entry(&encode_string_h160(k), &v)?;
	}
	map.end()
}

pub fn deserialize_hash_map_h160_account<'de, D, Account>(
	deserializer: D,
) -> Result<HashMap<H160, Account>, D::Error>
where
	D: Deserializer<'de>,
	Account: Deserialize<'de>,
{
	let map = <HashMap<String, Account>>::deserialize(deserializer)?;
	let mut hashmap: HashMap<H160, Account> = HashMap::new();

	for (k, v) in map {
		let k_h160 = parse_address(&k).map_err(|e| {
			serde::de::Error::custom(format!("Failed to parse address from string '{}': {}", k, e))
		})?;
		hashmap.insert(k_h160, v);
	}
	Ok(hashmap)
}

// Secp256r1PrivateKey

pub fn deserialize_private_key<'de, D>(deserializer: D) -> Result<Secp256r1PrivateKey, D::Error>
where
	D: Deserializer<'de>,
{
	let s: String = Deserialize::deserialize(deserializer)?;
	let h256 = parse_string_h256(&s).map_err(|e| {
		serde::de::Error::custom(format!("Failed to parse H256 from string '{}': {}", s, e))
	})?;

	let key = Secp256r1PrivateKey::from_bytes(h256.as_bytes()).map_err(|e| {
		serde::de::Error::custom(format!("Failed to create private key from bytes: {}", e))
	})?;

	Ok(key)
}

pub fn serialize_private_key<S>(
	item: &Secp256r1PrivateKey,
	serializer: S,
) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	let item_str = encode_string_h256(&H256::from_slice(item.to_raw_bytes().as_ref()));
	serializer.serialize_str(&item_str)
}

// Secp256r1PublicKey
pub fn deserialize_public_key<'de, D>(deserializer: D) -> Result<Secp256r1PublicKey, D::Error>
where
	D: Deserializer<'de>,
{
	let s: String = Deserialize::deserialize(deserializer)?;
	let h256 = parse_string_h256(&s).map_err(|e| {
		serde::de::Error::custom(format!("Failed to parse H256 from string '{}': {}", s, e))
	})?;

	let key = Secp256r1PublicKey::from_bytes(h256.as_bytes()).map_err(|e| {
		serde::de::Error::custom(format!("Failed to create public key from bytes: {}", e))
	})?;

	Ok(key)
}

pub fn serialize_public_key<S>(item: &Secp256r1PublicKey, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	let item_str = encode_string_h256(&H256::from_slice(item.get_encoded(true).as_slice()));
	serializer.serialize_str(&item_str)
}

pub fn deserialize_vec_public_key<'de, D>(
	deserializer: D,
) -> Result<Vec<Secp256r1PublicKey>, D::Error>
where
	D: Deserializer<'de>,
{
	let string_seq = <Vec<String>>::deserialize(deserializer)?;
	let mut vec: Vec<Secp256r1PublicKey> = Vec::new();

	for v_str in string_seq {
		let v = parse_string_h256(&v_str).map_err(|e| {
			serde::de::Error::custom(format!("Failed to parse H256 from string '{}': {}", v_str, e))
		})?;

		let key = Secp256r1PublicKey::from_bytes(v.as_bytes()).map_err(|e| {
			serde::de::Error::custom(format!("Failed to create public key from bytes: {}", e))
		})?;

		vec.push(key);
	}

	Ok(vec)
}

pub fn deserialize_vec_public_key_option<'de, D>(
	deserializer: D,
) -> Result<Option<Vec<Secp256r1PublicKey>>, D::Error>
where
	D: Deserializer<'de>,
{
	let string_seq = <Vec<String>>::deserialize(deserializer)?;

	let mut vec: Vec<Secp256r1PublicKey> = Vec::new();
	for v_str in string_seq {
		let v = parse_string_h256(&v_str).map_err(|e| {
			serde::de::Error::custom(format!("Failed to parse H256 from string '{}': {}", v_str, e))
		})?;

		let key = Secp256r1PublicKey::from_bytes(v.as_bytes()).map_err(|e| {
			serde::de::Error::custom(format!("Failed to create public key from bytes: {}", e))
		})?;

		vec.push(key);
	}

	Ok(Some(vec))
}

pub fn serialize_vec_public_key<S>(
	item: &Vec<Secp256r1PublicKey>,
	serializer: S,
) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	let mut seq = serializer.serialize_seq(Some(item.len()))?;
	for i in item {
		seq.serialize_element(&i.get_encoded_compressed_hex())?;
	}
	seq.end()
}

pub fn serialize_vec_public_key_option<S>(
	item: &Option<Vec<Secp256r1PublicKey>>,
	serializer: S,
) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	match item {
		Some(key) => {
			let mut seq = serializer.serialize_seq(Some(key.len()))?;
			for i in key {
				seq.serialize_element(&i.get_encoded_compressed_hex())?;
			}
			seq.end()
		},
		None => serializer.serialize_none(),
	}
}

// impl serialize_public_key_option
pub fn serialize_public_key_option<S>(
	item: &Option<Secp256r1PublicKey>,
	serializer: S,
) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	match item {
		Some(key) => {
			let key_str = key.get_encoded_compressed_hex();
			serializer.serialize_str(&key_str)
		},
		None => serializer.serialize_none(),
	}
}

// impl deserialize_public_key_option
pub fn deserialize_public_key_option<'de, D>(
	deserializer: D,
) -> Result<Option<Secp256r1PublicKey>, D::Error>
where
	D: Deserializer<'de>,
{
	let s: Option<String> = Deserialize::deserialize(deserializer)?;
	match s {
		Some(s) => {
			let pubkey_bytes = parse_string_h256(&s).map_err(|e| {
				serde::de::Error::custom(format!("Failed to parse H256 from string '{}': {}", s, e))
			})?;

			let key = Secp256r1PublicKey::from_bytes(pubkey_bytes.as_bytes()).map_err(|e| {
				serde::de::Error::custom(format!("Failed to create public key from bytes: {}", e))
			})?;

			Ok(Some(key))
		},
		None => Ok(None),
	}
}

// pub fn serialize_vec_methodtoken<S>(
// 	item: &Vec<MethodToken>,
// 	serializer: S,
// ) -> Result<S::Ok, S::Error>
// where
// 	S: Serializer,
// {
// 	let mut seq = serializer.serialize_seq(Some(item.len()))?;
// 	for i in item {
// 		seq.serialize_element(&i)?;
// 	}
// 	seq.end()
// }
//
// pub fn deserialize_vec_methodtoken<'de, D>(deserializer: D) -> Result<Vec<MethodToken>, D::Error>
// where
// 	D: Deserializer<'de>,
// {
// 	let string_seq = <Vec<MethodToken>>::deserialize(deserializer)?;
// 	let mut vec: Vec<MethodToken> = Vec::new();
// 	for v_str in string_seq {
// 		let v = v_str;
// 		vec.push(v);
// 	}
// 	Ok(vec)
// }

pub fn serialize_h256<S>(item: &H256, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	serializer.serialize_str(&encode_string_h256(item))
}

pub fn deserialize_h256<'de, D>(deserializer: D) -> Result<H256, D::Error>
where
	D: Deserializer<'de>,
{
	let s: String = Deserialize::deserialize(deserializer)?;
	parse_string_h256(&s).map_err(|e| {
		serde::de::Error::custom(format!("Failed to parse H256 from string '{}': {}", s, e))
	})
}

pub fn serialize_hashset_u256<S>(item: &HashSet<U256>, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	let mut seq = serializer.serialize_seq(Some(item.len()))?;
	for i in item {
		seq.serialize_element(&encode_string_u256(i))?;
	}
	seq.end()
}

pub fn deserialize_hashset_u256<'de, D>(deserializer: D) -> Result<HashSet<U256>, D::Error>
where
	D: Deserializer<'de>,
{
	let string_seq = <HashSet<String>>::deserialize(deserializer)?;
	let mut hashset: HashSet<U256> = HashSet::new();
	for v_str in string_seq {
		let v = parse_string_u256(&v_str).map_err(|e| {
			serde::de::Error::custom(format!("Failed to parse U256 from string '{}': {}", v_str, e))
		})?;
		hashset.insert(v);
	}
	Ok(hashset)
}

pub fn serialize_vec_h256<S>(item: &Vec<H256>, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	let mut seq = serializer.serialize_seq(Some(item.len()))?;
	for i in item {
		seq.serialize_element(&encode_string_h256(i))?;
	}
	seq.end()
}

pub fn deserialize_vec_h256<'de, D>(deserializer: D) -> Result<Vec<H256>, D::Error>
where
	D: Deserializer<'de>,
{
	let string_seq = <Vec<String>>::deserialize(deserializer)?;
	let mut vec: Vec<H256> = Vec::new();
	for v_str in string_seq {
		let v = parse_string_h256(&v_str).map_err(|e| {
			serde::de::Error::custom(format!("Failed to parse H256 from string '{}': {}", v_str, e))
		})?;
		vec.push(v);
	}
	Ok(vec)
}

pub fn serialize_vec_u256<S>(item: &Vec<U256>, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	let mut seq = serializer.serialize_seq(Some(item.len()))?;
	for i in item {
		seq.serialize_element(&encode_string_u256(i))?;
	}
	seq.end()
}

pub fn deserialize_vec_u256<'de, D>(deserializer: D) -> Result<Vec<U256>, D::Error>
where
	D: Deserializer<'de>,
{
	let string_seq = <Vec<String>>::deserialize(deserializer)?;
	let mut vec: Vec<U256> = Vec::new();
	for v_str in string_seq {
		let v = parse_string_u256(&v_str).map_err(|e| {
			serde::de::Error::custom(format!("Failed to parse U256 from string '{}': {}", v_str, e))
		})?;
		vec.push(v);
	}
	Ok(vec)
}

pub fn serialize_h256_option<S>(item: &Option<H256>, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	match item {
		Some(h256) => {
			let h256_str = encode_string_h256(h256);
			serializer.serialize_str(&h256_str)
		},
		None => serializer.serialize_none(),
	}
}

pub fn deserialize_h256_option<'de, D>(deserializer: D) -> Result<Option<H256>, D::Error>
where
	D: Deserializer<'de>,
{
	let s: Option<String> = Deserialize::deserialize(deserializer)?;
	match s {
		Some(s) => {
			let h256 = parse_string_h256(&s).map_err(|e| {
				serde::de::Error::custom(format!("Failed to parse H256 from string '{}': {}", s, e))
			})?;
			Ok(Some(h256))
		},
		None => Ok(None),
	}
}

pub fn serialize_hashmap_u256_hashset_u256<S>(
	item: &HashMap<U256, HashSet<U256>>,
	serializer: S,
) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	let mut map = serializer.serialize_map(Some(item.len()))?;
	for (k, v) in item {
		let value: HashSet<String> = v.iter().map(encode_string_u256).collect();
		map.serialize_entry(&encode_string_u256(k), &value)?;
	}
	map.end()
}

pub fn deserialize_hashmap_u256_hashset_u256<'de, D>(
	deserializer: D,
) -> Result<HashMap<U256, HashSet<U256>>, D::Error>
where
	D: Deserializer<'de>,
{
	let map = <HashMap<String, HashSet<String>>>::deserialize(deserializer)?;
	let mut hashmap: HashMap<U256, HashSet<U256>> = HashMap::new();

	for (k, v) in map {
		let k_u256 = parse_string_u256(&k).map_err(|e| {
			serde::de::Error::custom(format!("Failed to parse U256 from string '{}': {}", k, e))
		})?;

		let mut v_hashset_u256: HashSet<U256> = HashSet::new();
		for x in v.iter() {
			let parsed = parse_string_u256(x).map_err(|e| {
				serde::de::Error::custom(format!("Failed to parse U256 from string '{}': {}", x, e))
			})?;
			v_hashset_u256.insert(parsed);
		}

		hashmap.insert(k_u256, v_hashset_u256);
	}
	Ok(hashmap)
}

pub fn serialize_hashmap_address_u256<S>(
	item: &HashMap<Address, U256>,
	serializer: S,
) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	let mut map = serializer.serialize_map(Some(item.len()))?;
	for (k, v) in item {
		map.serialize_entry(k, &encode_string_u256(v))?;
	}
	map.end()
}

pub fn deserialize_hashmap_address_u256<'de, D>(
	deserializer: D,
) -> Result<HashMap<Address, U256>, D::Error>
where
	D: Deserializer<'de>,
{
	let map = <HashMap<String, String>>::deserialize(deserializer)?;
	let mut hashmap: HashMap<Address, U256> = HashMap::new();

	for (k, v) in map {
		// let k_h160 = parse_address(&k);
		let v_u256 = parse_string_u256(&v).map_err(|e| {
			serde::de::Error::custom(format!("Failed to parse U256 from string '{}': {}", v, e))
		})?;
		hashmap.insert(k, v_u256);
	}
	Ok(hashmap)
}

pub fn serialize_hashmap_u256_hashset_h256<S>(
	item: &HashMap<U256, HashSet<H256>>,
	serializer: S,
) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	let mut map = serializer.serialize_map(Some(item.len()))?;
	for (k, v) in item {
		let value: HashSet<String> = v.iter().map(encode_string_h256).collect();
		map.serialize_entry(&encode_string_u256(k), &value)?;
	}
	map.end()
}

pub fn deserialize_hashmap_u256_hashset_h256<'de, D>(
	deserializer: D,
) -> Result<HashMap<U256, HashSet<H256>>, D::Error>
where
	D: Deserializer<'de>,
{
	let map = <HashMap<String, HashSet<String>>>::deserialize(deserializer)?;
	let mut hashmap: HashMap<U256, HashSet<H256>> = HashMap::new();

	for (k, v) in map {
		let k_u256 = parse_string_u256(&k).map_err(|e| {
			serde::de::Error::custom(format!("Failed to parse U256 from string '{}': {}", k, e))
		})?;

		let mut v_hashset_h256: HashSet<H256> = HashSet::new();
		for x in v.iter() {
			let parsed = parse_string_h256(x).map_err(|e| {
				serde::de::Error::custom(format!("Failed to parse H256 from string '{}': {}", x, e))
			})?;
			v_hashset_h256.insert(parsed);
		}

		hashmap.insert(k_u256, v_hashset_h256);
	}
	Ok(hashmap)
}

pub fn serialize_hashmap_u256_vec_u256<S>(
	item: &HashMap<U256, Vec<U256>>,
	serializer: S,
) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	let mut map = serializer.serialize_map(Some(item.len()))?;
	for (k, v) in item {
		let value: Vec<String> = v.iter().map(encode_string_u256).collect();
		map.serialize_entry(&encode_string_u256(k), &value)?;
	}
	map.end()
}

pub fn deserialize_hashmap_u256_vec_u256<'de, D>(
	deserializer: D,
) -> Result<HashMap<U256, Vec<U256>>, D::Error>
where
	D: Deserializer<'de>,
{
	let map = <HashMap<String, Vec<String>>>::deserialize(deserializer)?;
	let mut hashmap: HashMap<U256, Vec<U256>> = HashMap::new();

	for (k, v) in map {
		let k_u256 = parse_string_u256(&k).map_err(|e| {
			serde::de::Error::custom(format!("Failed to parse U256 from string '{}': {}", k, e))
		})?;

		let mut v_vec_u256: Vec<U256> = Vec::new();
		for x in v.iter() {
			let parsed = parse_string_u256(x).map_err(|e| {
				serde::de::Error::custom(format!("Failed to parse U256 from string '{}': {}", x, e))
			})?;
			v_vec_u256.push(parsed);
		}

		hashmap.insert(k_u256, v_vec_u256);
	}
	Ok(hashmap)
}

pub fn serialize_map<S>(
	map: &HashMap<ContractParameter, ContractParameter>,
	serializer: S,
) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	let mut serializable_map: Vec<(String, &ContractParameter)> = Vec::new();

	for (k, v) in map.iter() {
		let key_str = serde_json::to_string(k).map_err(|e| {
			serde::ser::Error::custom(format!("Failed to serialize contract parameter: {}", e))
		})?;
		serializable_map.push((key_str, v));
	}

	serializable_map.serialize(serializer)
}

pub fn deserialize_map<'de, D>(
	deserializer: D,
) -> Result<HashMap<ContractParameter, ContractParameter>, D::Error>
where
	D: Deserializer<'de>,
{
	let deserialized_vector: Vec<(String, ContractParameter)> = Vec::deserialize(deserializer)?;
	let mut map: HashMap<ContractParameter, ContractParameter> = HashMap::new();

	for (k, v) in deserialized_vector {
		let key = serde_json::from_str::<ContractParameter>(&k).map_err(|e| {
			serde::de::Error::custom(format!(
				"Failed to deserialize contract parameter from '{}': {}",
				k, e
			))
		})?;
		map.insert(key, v);
	}

	Ok(map)
}

const WILDCARD_CHAR: &str = "*";

pub fn serialize_wildcard<S>(methods: &Vec<String>, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	if !methods.is_empty() && methods[0] == WILDCARD_CHAR {
		serializer.serialize_str(WILDCARD_CHAR)
	} else {
		let mut seq = serializer.serialize_seq(Some(methods.len()))?;
		for method in methods {
			seq.serialize_element(method)?;
		}
		seq.end()
	}
}

pub fn deserialize_wildcard<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
	D: Deserializer<'de>,
{
	struct StringOrVec;

	impl<'de> Visitor<'de> for StringOrVec {
		type Value = Vec<String>;

		fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
			formatter.write_str("a string or a sequence of strings")
		}

		fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
		where
			E: de::Error,
		{
			Ok(vec![value.to_owned()])
		}

		fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
		where
			A: SeqAccess<'de>,
		{
			let mut vec = Vec::new();
			while let Some(value) = seq.next_element()? {
				vec.push(value);
			}
			Ok(vec)
		}
	}

	deserializer.deserialize_any(StringOrVec)
}

// Custom deserializer function
pub fn deserialize_hardforks<'de, D>(deserializer: D) -> Result<Vec<HardForks>, D::Error>
where
	D: Deserializer<'de>,
{
	struct HardforksVisitor;

	impl<'de> Visitor<'de> for HardforksVisitor {
		type Value = Vec<HardForks>;

		fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
			formatter.write_str("a list or a single Hardforks value")
		}

		fn visit_seq<V>(self, mut seq: V) -> Result<Vec<HardForks>, V::Error>
		where
			V: serde::de::SeqAccess<'de>,
		{
			let mut values = Vec::new();
			while let Some(value) = seq.next_element()? {
				values.push(value);
			}
			Ok(values)
		}

		fn visit_map<V>(self, map: V) -> Result<Vec<HardForks>, V::Error>
		where
			V: serde::de::MapAccess<'de>,
		{
			let single_value: HardForks =
				Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))?;
			Ok(vec![single_value])
		}
	}

	deserializer.deserialize_any(HardforksVisitor)
}

#[cfg(test)]
mod test {
	use super::*;

	#[derive(Clone, Default, Debug, Serialize, Deserialize)]
	struct TestStruct {
		#[serde(serialize_with = "serialize_hashset_u256")]
		#[serde(deserialize_with = "deserialize_hashset_u256")]
		value: HashSet<U256>,
	}

	#[derive(Clone, Default, Debug, Serialize)]
	struct TestStruct2 {
		#[serde(serialize_with = "serialize_hashmap_u256_hashset_u256")]
		value2: HashMap<U256, HashSet<U256>>,
	}

	#[test]
	fn test_serialize_hashset_u256() {
		let mut v: HashSet<U256> = HashSet::new();
		v.insert(10.into());
		v.insert(0x10000.into());
		let _copy = v.clone();
		let test_struct = TestStruct { value: v };
		let json_string = serde_json::to_string_pretty(&test_struct).unwrap();
		println!("{json_string}");
		let v_copy: TestStruct = serde_json::from_str(&json_string).unwrap();
		assert_eq!(test_struct.value, v_copy.value);
	}

	#[test]
	fn test_serialize_hashmap_u256_hashset_u256() {
		let mut v: HashMap<U256, HashSet<U256>> = HashMap::new();
		let mut v2: HashSet<U256> = HashSet::new();
		v2.insert(10.into());
		v2.insert(0x10000.into());
		v.insert(20.into(), v2);
		let test_struct = TestStruct2 { value2: v };
		let json_string = serde_json::to_string_pretty(&test_struct).unwrap();
		println!("{json_string}");
	}

	#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq)]
	struct OptionalHeights {
		#[serde(serialize_with = "serialize_u64", deserialize_with = "deserialize_u64")]
		current: u64,
		#[serde(
			serialize_with = "serialize_u64_option",
			deserialize_with = "deserialize_u64_option",
			default,
			skip_serializing_if = "Option::is_none"
		)]
		highest: Option<u64>,
	}

	#[test]
	fn test_u64_hex_and_option_roundtrip() {
		let payload = OptionalHeights { current: 0x10, highest: Some(0x20) };
		let json = serde_json::to_string(&payload).unwrap();
		assert!(json.contains("16")); // decimal output

		let parsed: OptionalHeights = serde_json::from_str(&json).unwrap();
		assert_eq!(parsed, payload);

		// hex inputs should also deserialize
		let hex_json = r#"{"current":"0x10","highest":"0x20"}"#;
		let parsed_hex: OptionalHeights = serde_json::from_str(hex_json).unwrap();
		assert_eq!(parsed_hex.current, 16);
		assert_eq!(parsed_hex.highest, Some(32));
	}

	#[test]
	fn test_serialize_bytes() {
		#[derive(Clone, Default, Debug, Serialize, Deserialize)]
		struct TestStruct {
			#[serde(serialize_with = "serialize_bytes")]
			#[serde(deserialize_with = "deserialize_bytes")]
			value: Vec<u8>,
		}

		let v = TestStruct { value: vec![23, 253, 255, 255, 0, 123] };
		let json_string = serde_json::to_string_pretty(&v).unwrap();
		println!("{json_string}");
		let v_copy: TestStruct = serde_json::from_str(&json_string).unwrap();
		assert_eq!(v.value, v_copy.value);
	}

	#[test]
	fn test_serialize_u32() {
		#[derive(Clone, Default, Debug, Serialize, Deserialize)]
		struct TestStruct {
			#[serde(serialize_with = "serialize_u32")]
			#[serde(deserialize_with = "deserialize_u32")]
			value: u32,
		}

		let v = TestStruct { value: 20 };
		let json_string = serde_json::to_string_pretty(&v).unwrap();
		println!("{json_string}");
		let v_copy: TestStruct = serde_json::from_str(&json_string).unwrap();
		assert_eq!(v.value, v_copy.value);
	}

	#[test]
	fn test_serialize_vec_h256() {
		#[derive(Clone, Default, Debug, Serialize, Deserialize)]
		struct TestStruct {
			#[serde(serialize_with = "serialize_vec_h256")]
			#[serde(deserialize_with = "deserialize_vec_h256")]
			value: Vec<H256>,
		}

		let v = TestStruct {
			value: vec![parse_string_h256(
				"0x95ff99bcdac06fad4a141f06c5f9f1c65e71b188ff5978116a110c4170fd7355",
			)
			.expect("Failed to parse H256 string")],
		};
		let json_string = serde_json::to_string_pretty(&v).unwrap();
		println!("{json_string}");
		let v_copy: TestStruct = serde_json::from_str(&json_string).unwrap();
		assert_eq!(v.value, v_copy.value);
	}
}
