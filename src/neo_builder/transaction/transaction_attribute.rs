use std::{
	fmt::Debug,
	hash::{Hash, Hasher},
};

use crate::neo_crypto::utils::FromBase64String;
use primitive_types::H256;
use serde::{Deserialize, Serialize};

use crate::{
	builder::TransactionError,
	codec::{Decoder, Encoder, NeoSerializable},
	prelude::Base64Encode,
};

use super::oracle_response_code::OracleResponseCode;

#[derive(Serialize, Deserialize, PartialEq, Hash, Debug, Clone)]
#[serde(tag = "type")]
pub enum TransactionAttribute {
	#[serde(rename = "HighPriority")]
	HighPriority,

	#[serde(rename = "OracleResponse")]
	OracleResponse(OracleResponse),

	#[serde(rename = "NotValidBefore")]
	NotValidBefore {
		height: u32,
	},

	Conflicts {
		hash: H256,
	},
}

#[derive(Serialize, Deserialize, PartialEq, Hash, Debug, Clone)]
pub struct OracleResponse {
	pub id: u32,
	pub response_code: OracleResponseCode,
	pub result: String,
}

impl TransactionAttribute {
	pub const MAX_RESULT_SIZE: usize = 0xffff;

	pub fn to_bytes(&self) -> Vec<u8> {
		let mut bytes = vec![];

		match self {
			TransactionAttribute::HighPriority => {
				bytes.push(0x01);
			},
			TransactionAttribute::OracleResponse(OracleResponse { id, response_code, result }) => {
				bytes.push(0x11);
				bytes.extend(&id.to_be_bytes());
				bytes.push(*response_code as u8);
				bytes.extend(result.as_bytes());
			},
			_ => {},
		}

		bytes
	}

	pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
		match bytes[0] {
			0x01 => Ok(TransactionAttribute::HighPriority),
			0x11 => {
				if bytes.len() < 9 {
					return Err("Not enough bytes for OracleResponse");
				}
				let mut array = [0; 8];
				let slice_len = bytes[1..9].len();
				array[8 - slice_len..].copy_from_slice(&bytes[1..9]);
				let id = u64::from_be_bytes(array);
				let response_code = OracleResponseCode::try_from(bytes[9]).unwrap();
				let result =
					String::from_utf8(bytes[10..].to_vec()).map_err(|_| "Invalid UTF-8").unwrap();

				Ok(TransactionAttribute::OracleResponse(OracleResponse {
					id: id as u32,
					response_code,
					result,
				}))
			},
			_ => Err("Invalid attribute type byte"),
		}
	}

	pub fn to_json(&self) -> String {
		serde_json::to_string(self).unwrap()
	}

	// Get the height for NotValidBefore attribute
	pub fn get_height(&self) -> Option<&u32> {
		match self {
			TransactionAttribute::NotValidBefore { height } => Some(height),
			_ => None,
		}
	}

	// Get the height for NotValidBefore attribute
	pub fn get_hash(&self) -> Option<&H256> {
		match self {
			TransactionAttribute::Conflicts { hash } => Some(hash),
			_ => None,
		}
	}
}

impl NeoSerializable for TransactionAttribute {
	type Error = TransactionError;

	fn size(&self) -> usize {
		match self {
			TransactionAttribute::HighPriority => 1,
			TransactionAttribute::OracleResponse(OracleResponse {
				id: _,
				response_code: _,
				result,
			}) => 1 + 9 + result.len(),
			TransactionAttribute::NotValidBefore { height: _ } => 1 + 4, // 1 byte type + 4 bytes height
			TransactionAttribute::Conflicts { hash: _ } => 1 + 32,       // 1 byte type + 32 bytes hash
		}
	}

	fn encode(&self, writer: &mut Encoder) {
		match self {
			TransactionAttribute::HighPriority => {
				writer.write_u8(0x01);
			},
			TransactionAttribute::OracleResponse(OracleResponse { id, response_code, result }) => {
				writer.write_u8(0x11);
				let mut v = id.to_be_bytes();
				v.reverse();
				writer.write(&v);
				writer.write_u8(*response_code as u8);
				writer
					.write_var_bytes(result.from_base64_string().unwrap().as_slice())
					.expect("Failed to encode oracle response");
			},
			_ => {},
		}
	}

	fn decode(reader: &mut Decoder) -> Result<Self, Self::Error> {
		match reader.read_u8() {
			0x01 => Ok(TransactionAttribute::HighPriority),
			0x11 => {
				let id = reader.read_u32().map_err(|e| {
					TransactionError::TransactionConfiguration(format!(
						"Failed to read oracle response ID: {}",
						e
					))
				})?;
				let response_code =
					OracleResponseCode::try_from(reader.read_u8()).map_err(|_| {
						TransactionError::TransactionConfiguration(
							"Invalid oracle response code".to_string(),
						)
					})?;
				let result = reader
					.read_var_bytes()
					.map_err(|e| {
						TransactionError::TransactionConfiguration(format!(
							"Failed to read oracle response result: {}",
							e
						))
					})?
					.to_base64();

				Ok(TransactionAttribute::OracleResponse(OracleResponse {
					id,
					response_code,
					result,
				}))
			},
			_ => Err(TransactionError::InvalidTransaction),
		}
	}

	fn to_array(&self) -> Vec<u8> {
		let mut writer = Encoder::new();
		self.encode(&mut writer);
		writer.to_bytes()
	}
}
