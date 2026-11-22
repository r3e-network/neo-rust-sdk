// use std::hash::Hasher;

use primitive_types::H160;
// use tokio::io::AsyncReadExt;

use crate::{
	TypeError,
	codec::{CodecError, Decoder, Encoder, NeoSerializable, VarSizeTrait},
	crypto::HashableForVec,
	neo_types::StringExt,
};
use neo3::prelude::{Bytes, ContractParameter, StackItem};
/*
┌───────────────────────────────────────────────────────────────────────┐
│                    NEO Executable Format 3 (NEF3)                     │
├──────────┬───────────────┬────────────────────────────────────────────┤
│  Field   │     Type      │                  Comment                   │
├──────────┼───────────────┼────────────────────────────────────────────┤
│ Magic    │ uint32        │ Magic header                               │
│ Compiler │ byte[64]      │ Compiler name and version                  │
├──────────┼───────────────┼────────────────────────────────────────────┤
│ Source   │ byte[]        │ The url of the source files, max 255 bytes │
│ Reserve  │ byte[2]       │ Reserved for future extensions. Must be 0. │
│ Tokens   │ MethodToken[] │ Method tokens                              │
│ Reserve  │ byte[2]       │ Reserved for future extensions. Must be 0. │
│ Script   │ byte[]        │ Var bytes for the payload                  │
├──────────┼───────────────┼────────────────────────────────────────────┤
│ Checksum │ uint32        │ First four bytes of double SHA256 hash     │
└──────────┴───────────────┴────────────────────────────────────────────┘
 */

#[derive(Debug, Clone)]
pub struct NefFile {
	pub(crate) compiler: Option<String>,
	source_url: String,
	method_tokens: Vec<MethodToken>,
	pub(crate) script: Bytes,
	pub(crate) checksum: Bytes,
}

impl From<NefFile> for ContractParameter {
	fn from(val: NefFile) -> ContractParameter {
		ContractParameter::try_from_nef_file(&val).unwrap_or_else(|err| {
			tracing::warn!(
				error = %err,
				"Failed to convert owned NEF file to contract parameter via safe path; falling back to legacy serializer"
			);
			ContractParameter::byte_array(val.to_array())
		})
	}
}

#[allow(dead_code)]
impl NefFile {
	const MAGIC: u32 = 0x3346454E;
	const MAGIC_SIZE: usize = 4;
	const COMPILER_SIZE: usize = 64;
	const MAX_SOURCE_URL_SIZE: usize = 256;
	const MAX_SCRIPT_LENGTH: usize = 512 * 1024;
	const CHECKSUM_SIZE: usize = 4;
	pub const HEADER_SIZE: usize = Self::MAGIC_SIZE + Self::COMPILER_SIZE;

	pub fn new(
		compiler: Option<String>,
		source_url: impl Into<String>,
		script: Bytes,
		checksum: Bytes,
	) -> Self {
		let mut file = Self {
			compiler,
			source_url: source_url.into(),
			method_tokens: Vec::new(),
			script,
			checksum,
		};

		if let Ok(computed_checksum) = file.compute_checksum_for_payload() {
			file.checksum = computed_checksum;
		}

		file
	}

	fn get_checksum_as_integer(bytes: &Bytes) -> Result<i32, TypeError> {
		let mut bytes = bytes.clone();
		bytes.reverse();
		bytes.try_into().map(i32::from_be_bytes).map_err(|_| {
			TypeError::InvalidEncoding("Failed to convert checksum bytes to i32".to_string())
		})
	}

	fn compute_checksum(file: &NefFile) -> Result<Bytes, TypeError> {
		file.compute_checksum_for_payload()
	}

	fn compute_checksum_from_bytes(bytes: Bytes) -> Result<Bytes, TypeError> {
		if bytes.len() < Self::CHECKSUM_SIZE {
			return Err(TypeError::InvalidEncoding("Invalid checksum".to_string()));
		}
		let mut file_bytes = bytes.clone();
		file_bytes.truncate(bytes.len() - Self::CHECKSUM_SIZE);
		Ok(file_bytes.hash256()[..Self::CHECKSUM_SIZE].to_vec())
	}

	fn encode_without_checksum(&self, writer: &mut Encoder) -> Result<(), TypeError> {
		if self.source_url.len() > Self::MAX_SOURCE_URL_SIZE {
			return Err(TypeError::InvalidEncoding(format!(
				"NEF source URL exceeds maximum length of {} bytes",
				Self::MAX_SOURCE_URL_SIZE
			)));
		}

		if self.script.is_empty() {
			return Err(TypeError::InvalidEncoding("Invalid script".to_string()));
		}

		if self.script.len() > Self::MAX_SCRIPT_LENGTH {
			return Err(TypeError::InvalidEncoding(format!(
				"NEF script exceeds maximum length of {} bytes",
				Self::MAX_SCRIPT_LENGTH
			)));
		}

		writer.write_u32(Self::MAGIC);
		writer.write_fixed_string(&self.compiler, Self::COMPILER_SIZE).map_err(|e| {
			TypeError::InvalidEncoding(format!("Failed to serialize NEF compiler: {}", e))
		})?;
		writer.write_var_string(&self.source_url);
		writer.write_u8(0);
		writer.write_var_int(self.method_tokens.len() as i64).map_err(|e| {
			TypeError::InvalidEncoding(format!("Failed to serialize NEF method token count: {}", e))
		})?;
		for method_token in &self.method_tokens {
			method_token.try_encode(writer).map_err(|e| {
				TypeError::InvalidEncoding(format!("Failed to serialize NEF method token: {}", e))
			})?;
		}
		writer.write_u16(0);
		writer.write_var_bytes(&self.script).map_err(|e| {
			TypeError::InvalidEncoding(format!("Failed to serialize NEF script: {}", e))
		})?;
		Ok(())
	}

	fn compute_checksum_for_payload(&self) -> Result<Bytes, TypeError> {
		let mut writer = Encoder::new();
		self.encode_without_checksum(&mut writer)?;
		let bytes = writer.to_bytes();
		Ok(bytes.hash256()[..Self::CHECKSUM_SIZE].to_vec())
	}

	fn encode_legacy(&self, writer: &mut Encoder) {
		writer.write_u32(Self::MAGIC);
		if let Err(err) = writer.write_fixed_string(&self.compiler, Self::COMPILER_SIZE) {
			tracing::warn!(error = %err, "Failed to serialize NEF compiler");
		}
		writer.write_var_string(&self.source_url);
		writer.write_u8(0);
		if let Err(err) = writer.write_serializable_variable_list(&self.method_tokens) {
			tracing::warn!(error = %err, "Failed to serialize NEF method tokens");
		}
		writer.write_u16(0);
		if let Err(err) = writer.write_var_bytes(&self.script) {
			tracing::warn!(error = %err, "Failed to serialize NEF script");
		}
		writer.write_bytes(&self.checksum);
	}

	fn encode_best_effort(&self, writer: &mut Encoder) {
		match self.compute_checksum_for_payload() {
			Ok(computed_checksum) => {
				if let Err(err) = self.encode_without_checksum(writer) {
					tracing::warn!(
						error = %err,
						"Failed to serialize NEF payload after checksum recompute; falling back to legacy encoder"
					);
					self.encode_legacy(writer);
					return;
				}
				writer.write_bytes(&computed_checksum);
			},
			Err(err) => {
				tracing::warn!(
					error = %err,
					"Failed to serialize NEF via safe payload path; falling back to legacy encoder"
				);
				self.encode_legacy(writer);
			},
		}
	}
	fn read_from_file(file: &str) -> Result<Self, TypeError> {
		let file_bytes = std::fs::read(file)
			.map_err(|e| TypeError::InvalidArgError(format!("Failed to read NEF file: {}", e)))?;

		if file_bytes.len() > 0x100000 {
			return Err(TypeError::InvalidArgError("NEF file is too large".to_string()));
		}

		let mut reader = Decoder::new(&file_bytes);
		reader.read_serializable().map_err(|e| {
			TypeError::InvalidEncoding(format!("Failed to deserialize NEF file: {}", e))
		})
	}

	/// Deserializes a NEF file from a byte array
	///
	/// # Arguments
	///
	/// * `bytes` - The byte array to deserialize
	///
	/// # Returns
	///
	/// A `Result` containing the deserialized NEF file or a `TypeError`
	pub fn deserialize(bytes: &[u8]) -> Result<Self, TypeError> {
		if bytes.len() > 0x100000 {
			return Err(TypeError::InvalidArgError("NEF file is too large".to_string()));
		}

		let mut reader = Decoder::new(bytes);
		reader.read_serializable().map_err(|e| {
			TypeError::InvalidEncoding(format!("Failed to deserialize NEF file: {}", e))
		})
	}

	fn read_from_stack_item(item: StackItem) -> Result<Self, TypeError> {
		if let StackItem::ByteString { value: bytes } = item {
			let mut reader = Decoder::new(bytes.as_bytes());
			reader.read_serializable().map_err(|e| {
				TypeError::InvalidEncoding(format!(
					"Failed to deserialize NEF from stack item: {}",
					e
				))
			})
		} else {
			let item_str = serde_json::to_string(&item).map_err(|e| {
				TypeError::InvalidFormat(format!("Failed to serialize stack item: {}", e))
			})?;

			Err(TypeError::UnexpectedReturnType(item_str + StackItem::BYTE_STRING_VALUE))
		}
	}

	pub fn try_encode(&self, writer: &mut Encoder) -> Result<(), TypeError> {
		let computed_checksum = self.compute_checksum_for_payload()?;

		if self.checksum.len() != Self::CHECKSUM_SIZE {
			return Err(TypeError::InvalidEncoding("NEF checksum length is invalid".to_string()));
		}

		if self.checksum != computed_checksum {
			return Err(TypeError::InvalidEncoding("Invalid checksum".to_string()));
		}

		self.encode_without_checksum(writer)?;
		writer.write_bytes(&computed_checksum);
		Ok(())
	}

	pub fn try_to_array(&self) -> Result<Vec<u8>, TypeError> {
		let mut writer = Encoder::new();
		self.try_encode(&mut writer)?;
		Ok(writer.to_bytes())
	}
}

impl NeoSerializable for NefFile {
	type Error = TypeError;

	fn size(&self) -> usize {
		let mut size = Self::HEADER_SIZE;
		size += self.source_url.var_size() + self.source_url.len();
		size += 1;
		size += self.method_tokens.var_size();
		size += 2;
		size += self.script.var_size();
		size += Self::CHECKSUM_SIZE;

		size
	}

	fn encode(&self, writer: &mut Encoder) {
		self.encode_best_effort(writer);
	}

	fn decode(reader: &mut Decoder) -> Result<Self, Self::Error> {
		let magic = reader
			.read_u32()
			.map_err(|e| TypeError::InvalidEncoding(format!("Failed to read magic: {}", e)))?;

		if magic != Self::MAGIC {
			return Err(TypeError::InvalidEncoding("Invalid magic".to_string()));
		}

		let compiler_bytes = reader.read_bytes(Self::COMPILER_SIZE)?;
		let compiler = String::from_utf8(compiler_bytes.to_vec())
			.map_err(|_| CodecError::InvalidEncoding("Invalid compiler".to_string()))?;

		let source_url = reader.read_var_string_bounded(Self::MAX_SOURCE_URL_SIZE)?;

		if reader.read_u8_safe()? != 0 {
			return Err(TypeError::InvalidEncoding("Invalid reserve bytes".to_string()));
		}

		// Avoid pathological `tokens` lengths by bounding based on the remaining buffer and the
		// minimum possible serialized size of a `MethodToken`.
		let min_trailing_bytes = 2 + 1 + 1 + Self::CHECKSUM_SIZE;
		let max_tokens = reader.available().saturating_sub(min_trailing_bytes)
			/ MethodToken::MIN_SERIALIZED_SIZE;
		let method_tokens = reader.read_serializable_list_bounded::<MethodToken>(max_tokens)?;

		if reader.read_u16().map_err(|e| {
			TypeError::InvalidEncoding(format!("Failed to read reserve bytes: {}", e))
		})? != 0
		{
			return Err(TypeError::InvalidEncoding("Invalid reserve bytes".to_string()));
		}

		let script = reader.read_var_bytes_bounded(Self::MAX_SCRIPT_LENGTH)?;
		if script.is_empty() {
			return Err(TypeError::InvalidEncoding("Invalid script".to_string()));
		}

		let checksum = reader.read_bytes(Self::CHECKSUM_SIZE)?;
		let file = Self {
			compiler: Some(compiler),
			source_url,
			method_tokens,
			script,
			checksum: checksum.clone(),
		};

		let computed_checksum = Self::compute_checksum(&file)?;
		if checksum != computed_checksum {
			return Err(TypeError::InvalidEncoding("Invalid checksum".to_string()));
		}

		Ok(file)
	}

	fn to_array(&self) -> Vec<u8> {
		self.try_to_array().unwrap_or_else(|err| {
			tracing::warn!(
				error = %err,
				"Failed to serialize NEF via safe path; falling back to legacy encoder"
			);
			let mut writer = Encoder::new();
			self.encode(&mut writer);
			writer.to_bytes()
		})
	}
}

#[derive(Debug, Clone)]
pub struct MethodToken {
	hash: H160,
	method: String,
	params_count: u16,
	has_return_value: bool,
	call_flags: u8,
}

impl MethodToken {
	const PARAMS_COUNT_SIZE: usize = 2;
	const HAS_RETURN_VALUE_SIZE: usize = 1;
	const CALL_FLAGS_SIZE: usize = 1;
	const MAX_METHOD_NAME_SIZE: usize = 256;
	const MIN_SERIALIZED_SIZE: usize =
		20 + 1 + Self::PARAMS_COUNT_SIZE + Self::HAS_RETURN_VALUE_SIZE + Self::CALL_FLAGS_SIZE;

	pub fn try_encode(&self, writer: &mut Encoder) -> Result<(), TypeError> {
		if self.method.len() > Self::MAX_METHOD_NAME_SIZE {
			return Err(TypeError::InvalidEncoding(format!(
				"MethodToken method exceeds maximum length of {} bytes",
				Self::MAX_METHOD_NAME_SIZE
			)));
		}

		writer.write_serializable_fixed(&self.hash);
		writer.write_var_string(&self.method);
		writer.write_u16(self.params_count);
		writer.write_bool(self.has_return_value);
		writer.write_u8(self.call_flags);
		Ok(())
	}

	pub fn try_to_array(&self) -> Result<Vec<u8>, TypeError> {
		let mut writer = Encoder::new();
		self.try_encode(&mut writer)?;
		Ok(writer.to_bytes())
	}
}

impl NeoSerializable for MethodToken {
	type Error = TypeError;

	fn size(&self) -> usize {
		let mut size = H160::len_bytes();
		size += self.method.var_size() + self.method.len();
		size += MethodToken::PARAMS_COUNT_SIZE;
		size += MethodToken::HAS_RETURN_VALUE_SIZE;
		size += MethodToken::CALL_FLAGS_SIZE;

		size
	}

	fn encode(&self, writer: &mut Encoder) {
		if let Err(err) = self.try_encode(writer) {
			tracing::warn!(
				error = %err,
				"Failed to serialize MethodToken via safe path; falling back to legacy encoder"
			);
			writer.write_serializable_fixed(&self.hash);
			writer.write_var_string(&self.method);
			writer.write_u16(self.params_count);
			writer.write_bool(self.has_return_value);
			writer.write_u8(self.call_flags);
		}
	}

	fn decode(reader: &mut Decoder) -> Result<Self, Self::Error>
	where
		Self: Sized,
	{
		let hash = reader.read_serializable()?;
		let method = reader.read_var_string_bounded(Self::MAX_METHOD_NAME_SIZE)?;
		let params_count = reader.read_u16().map_err(|e| {
			TypeError::InvalidEncoding(format!("Failed to read params_count: {}", e))
		})?;
		let has_return_value = reader.read_bool_safe()?;
		let call_flags = reader.read_u8_safe()?;

		Ok(Self { hash, method, params_count, has_return_value, call_flags })
	}

	fn to_array(&self) -> Vec<u8> {
		self.try_to_array().unwrap_or_else(|err| {
			tracing::warn!(
				error = %err,
				"Failed to serialize MethodToken via safe path; falling back to legacy encoder"
			);
			let mut writer = Encoder::new();
			writer.write_serializable_fixed(&self.hash);
			writer.write_var_string(&self.method);
			writer.write_u16(self.params_count);
			writer.write_bool(self.has_return_value);
			writer.write_u8(self.call_flags);
			writer.to_bytes()
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{codec::Encoder, crypto::HashableForVec};

	fn create_valid_serialized_nef_bytes() -> Vec<u8> {
		let script = vec![0x11, 0x40];
		let mut writer = Encoder::new();
		writer.write_u32(NefFile::MAGIC);
		writer
			.write_fixed_string(&Some("test-compiler".to_string()), NefFile::COMPILER_SIZE)
			.unwrap();
		writer.write_var_string("");
		writer.write_u8(0);
		writer.write_var_int(0).unwrap();
		writer.write_u16(0);
		writer.write_var_bytes(&script).unwrap();
		let mut bytes = writer.to_bytes();
		let checksum = bytes.hash256();
		bytes.extend_from_slice(&checksum[..NefFile::CHECKSUM_SIZE]);
		bytes
	}

	fn create_encodable_nef() -> NefFile {
		NefFile::new(Some("test-compiler".to_string()), String::new(), vec![0x11, 0x40], vec![0; 4])
	}

	#[test]
	fn test_deserialize_preserves_checksum_and_roundtrips() {
		let bytes = create_valid_serialized_nef_bytes();
		let nef = NefFile::deserialize(&bytes).unwrap();

		assert_eq!(nef.checksum.len(), NefFile::CHECKSUM_SIZE);
		assert_eq!(nef.try_to_array().unwrap(), bytes);
	}

	#[test]
	fn test_to_array_repairs_checksum_for_encodable_nef() {
		let legacy_nef = NefFile {
			compiler: Some("test-compiler".to_string()),
			source_url: String::new(),
			method_tokens: vec![],
			script: vec![0x11, 0x40],
			checksum: vec![0; NefFile::CHECKSUM_SIZE],
		};

		let canonical_nef = NefFile::new(
			Some("test-compiler".to_string()),
			String::new(),
			vec![0x11, 0x40],
			vec![0; 4],
		);

		assert_eq!(legacy_nef.to_array(), canonical_nef.to_array());
		assert!(NefFile::deserialize(&legacy_nef.to_array()).is_ok());
	}

	#[test]
	fn test_try_to_array_rejects_invalid_checksum_length() {
		let nef = NefFile {
			compiler: Some("test-compiler".to_string()),
			source_url: String::new(),
			method_tokens: vec![],
			script: vec![1, 2, 3],
			checksum: vec![0; NefFile::CHECKSUM_SIZE - 1],
		};

		assert!(matches!(
			nef.try_to_array(),
			Err(TypeError::InvalidEncoding(message)) if message.contains("checksum")
		));
	}

	#[test]
	fn test_try_to_array_rejects_mismatched_checksum() {
		let nef = NefFile {
			compiler: Some("test-compiler".to_string()),
			source_url: String::new(),
			method_tokens: vec![],
			script: vec![1, 2, 3],
			checksum: vec![0; NefFile::CHECKSUM_SIZE],
		};

		assert!(matches!(
			nef.try_to_array(),
			Err(TypeError::InvalidEncoding(message)) if message.contains("checksum")
		));
	}

	#[test]
	fn test_method_token_try_to_array_rejects_method_name_longer_than_max() {
		let token = MethodToken {
			hash: H160::zero(),
			method: "x".repeat(MethodToken::MAX_METHOD_NAME_SIZE + 1),
			params_count: 0,
			has_return_value: false,
			call_flags: 0,
		};

		assert!(matches!(
			token.try_to_array(),
			Err(TypeError::InvalidEncoding(message)) if message.contains("method")
		));
	}

	#[test]
	fn test_try_to_array_rejects_method_token_with_long_method_name() {
		let nef = NefFile {
			compiler: Some("test-compiler".to_string()),
			source_url: String::new(),
			method_tokens: vec![MethodToken {
				hash: H160::zero(),
				method: "x".repeat(MethodToken::MAX_METHOD_NAME_SIZE + 1),
				params_count: 0,
				has_return_value: false,
				call_flags: 0,
			}],
			script: vec![1, 2, 3],
			checksum: vec![0; 4],
		};

		assert!(matches!(
			nef.try_to_array(),
			Err(TypeError::InvalidEncoding(message)) if message.contains("method token")
		));
	}

	#[test]
	fn test_nef_size_matches_serialized_length() {
		let nef = create_encodable_nef();
		assert_eq!(nef.size(), nef.try_to_array().unwrap().len());
	}

	#[test]
	fn test_method_token_size_matches_serialized_length() {
		let token = MethodToken {
			hash: H160::zero(),
			method: "transfer".to_string(),
			params_count: 2,
			has_return_value: true,
			call_flags: 0x11,
		};

		assert_eq!(token.size(), token.to_array().len());
	}

	#[test]
	fn test_try_to_array_rejects_source_url_longer_than_max() {
		let nef = NefFile {
			compiler: Some("test-compiler".to_string()),
			source_url: "x".repeat(NefFile::MAX_SOURCE_URL_SIZE + 1),
			method_tokens: vec![],
			script: vec![1, 2, 3],
			checksum: vec![0; 4],
		};

		assert!(matches!(
			nef.try_to_array(),
			Err(TypeError::InvalidEncoding(message)) if message.contains("source")
		));
	}

	#[test]
	fn test_try_to_array_rejects_empty_script() {
		let nef = NefFile {
			compiler: Some("test-compiler".to_string()),
			source_url: String::new(),
			method_tokens: vec![],
			script: vec![],
			checksum: vec![0; 4],
		};

		assert!(matches!(
			nef.try_to_array(),
			Err(TypeError::InvalidEncoding(message)) if message.contains("script")
		));
	}

	#[test]
	fn test_try_to_array_rejects_script_longer_than_max() {
		let nef = NefFile {
			compiler: Some("test-compiler".to_string()),
			source_url: String::new(),
			method_tokens: vec![],
			script: vec![0; NefFile::MAX_SCRIPT_LENGTH + 1],
			checksum: vec![0; 4],
		};

		assert!(matches!(
			nef.try_to_array(),
			Err(TypeError::InvalidEncoding(message)) if message.contains("script")
		));
	}

	#[test]
	fn test_try_to_array_rejects_compiler_longer_than_fixed_width() {
		let nef = NefFile {
			compiler: Some("x".repeat(NefFile::COMPILER_SIZE + 1)),
			source_url: String::new(),
			method_tokens: vec![],
			script: vec![1, 2, 3],
			checksum: vec![0; 4],
		};

		assert!(matches!(
			nef.try_to_array(),
			Err(TypeError::InvalidEncoding(message)) if message.contains("compiler")
		));
	}

	#[test]
	fn test_try_to_array_matches_legacy_for_encodable_nef() {
		let nef = create_encodable_nef();
		assert_eq!(nef.try_to_array().unwrap(), nef.to_array());
	}
}
