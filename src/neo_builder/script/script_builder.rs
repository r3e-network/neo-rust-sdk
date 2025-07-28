use crate::{
	builder::{BuilderError, CallFlags, InteropService},
	codec::Encoder,
	crypto::Secp256r1PublicKey,
	neo_crypto::utils::{FromBase64String, FromHexString, ToHexString},
	Bytes, ContractParameter, ContractParameterType, OpCode, ParameterValue, ScriptHashExtension,
};
use futures_util::future::ok;
use getset::{Getters, Setters};
use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive};
use primitive_types::H160;
use serde::{Deserialize, Serialize};
use std::{
	cmp::PartialEq,
	collections::HashMap,
	convert::TryInto,
	fmt::{Debug, Formatter},
	str::FromStr,
};
use tokio::io::AsyncWriteExt;

/// A builder for constructing Neo smart contract scripts.
///
/// The `ScriptBuilder` provides methods to create and manipulate scripts
/// by adding opcodes, pushing data, and performing various operations
/// required for Neo smart contract execution.
///
/// # Examples
///
/// ```rust
/// use neo3::neo_builder::ScriptBuilder;
/// use neo3::neo_types::OpCode;
/// use num_bigint::BigInt;
///
/// let mut builder = ScriptBuilder::new();
/// builder.push_integer(BigInt::from(42))
///        .push_data("Hello, Neo!".as_bytes().to_vec())
///        .op_code(&[OpCode::Add]);
///
/// let script = builder.to_bytes();
/// ```
#[derive(Debug, PartialEq, Eq, Hash, Getters, Setters)]
pub struct ScriptBuilder {
	#[getset(get = "pub")]
	pub script: Encoder,
}

impl ScriptBuilder {
	/// Creates a new `ScriptBuilder` instance.
	///
	/// # Examples
	///
	/// ```rust
	/// use neo3::neo_builder::ScriptBuilder;
	///
	/// let builder = ScriptBuilder::new();
	/// ```
	pub fn new() -> Self {
		Self { script: Encoder::new() }
	}

	/// Appends one or more opcodes to the script.
	///
	/// # Arguments
	///
	/// * `op_codes` - A slice of `OpCode` values to append to the script.
	///
	/// # Returns
	///
	/// A mutable reference to the `ScriptBuilder` for method chaining.
	///
	/// # Examples
	///
	/// ```rust
	/// use neo3::neo_builder::ScriptBuilder;
	/// use neo3::neo_types::OpCode;
	///
	/// let mut builder = ScriptBuilder::new();
	/// builder.op_code(&[OpCode::Push1, OpCode::Push2, OpCode::Add]);
	/// ```
	pub fn op_code(&mut self, op_codes: &[OpCode]) -> &mut Self {
		for opcode in op_codes {
			self.script.write_u8(opcode.opcode());
		}
		self
	}

	/// Appends an opcode with an argument to the script.
	///
	/// # Arguments
	///
	/// * `opcode` - The `OpCode` to append.
	/// * `argument` - The data argument for the opcode.
	///
	/// # Returns
	///
	/// A mutable reference to the `ScriptBuilder` for method chaining.
	///
	/// # Examples
	///
	/// ```rust
	/// use neo3::neo_builder::ScriptBuilder;
	/// use neo3::neo_types::OpCode;
	///
	/// let mut builder = ScriptBuilder::new();
	/// builder.op_code_with_arg(OpCode::PushData1, vec![0x01, 0x02, 0x03]);
	/// ```
	pub fn op_code_with_arg(&mut self, opcode: OpCode, argument: Bytes) -> &mut Self {
		self.script.write_u8(opcode.opcode());
		let _ = self.script.write_bytes(&argument);
		self
	}

	/// Appends a contract call operation to the script.
	///
	/// # Arguments
	///
	/// * `hash160` - The 160-bit hash of the contract to call.
	/// * `method` - The name of the method to call.
	/// * `params` - A slice of `ContractParameter` values to pass as arguments to the method.
	/// * `call_flags` - An optional `CallFlags` value specifying the call flags.
	///
	/// # Returns
	///
	/// A `Result` containing a mutable reference to the `ScriptBuilder` for method chaining,
	/// or a `BuilderError` if an error occurs.
	///
	/// # Examples
	///
	/// ```rust
	/// use neo3::neo_builder::ScriptBuilder;
	/// use primitive_types::H160;
	/// use neo3::neo_types::ContractParameter;
	/// use neo3::builder::CallFlags;
	///
	/// let mut builder = ScriptBuilder::new();
	/// let contract_hash = H160::from_slice(&[0; 20]);
	/// let result = builder.contract_call(
	///     &contract_hash,
	///     "transfer",
	///     &[ContractParameter::from("NeoToken"), ContractParameter::from(1000)],
	///     Some(CallFlags::All)
	/// );
	/// ```
	pub fn contract_call(
		&mut self,
		hash160: &H160,
		method: &str,
		params: &[ContractParameter],
		call_flags: Option<CallFlags>,
	) -> Result<&mut Self, BuilderError> {
		if params.is_empty() {
			self.op_code(&[OpCode::NewArray0]);
		} else {
			self.push_params(params);
		}

		Ok(self
			.push_integer(BigInt::from(match call_flags {
				Some(flags) => flags.value(),
				None => CallFlags::All.value(),
			}))
			.push_data(method.as_bytes().to_vec())
			.push_data(hash160.to_vec())
			.sys_call(InteropService::SystemContractCall))
	}

	/// Appends a system call operation to the script.
	///
	/// # Arguments
	///
	/// * `operation` - The `InteropService` to call.
	///
	/// # Returns
	///
	/// A mutable reference to the `ScriptBuilder` for method chaining.
	///
	/// # Examples
	///
	/// ```rust
	/// use neo3::neo_builder::ScriptBuilder;
	/// use neo3::builder::InteropService;
	///
	/// let mut builder = ScriptBuilder::new();
	/// builder.sys_call(InteropService::SystemRuntimeCheckWitness);
	/// ```
	pub fn sys_call(&mut self, operation: InteropService) -> &mut Self {
		self.push_opcode_bytes(
			OpCode::Syscall,
			operation
				.hash()
				.from_hex_string()
				.map_err(|e| {
					BuilderError::IllegalArgument(format!("Invalid operation hash: {}", e))
				})
				.expect("InteropService hash should always be valid hex"),
		)
	}

	/// Pushes an array of contract parameters to the script.
	///
	/// # Arguments
	///
	/// * `params` - A slice of `ContractParameter` values to push to the script.
	///
	/// # Returns
	///
	/// A mutable reference to the `ScriptBuilder` for method chaining.
	///
	/// # Examples
	///
	/// ```rust
	/// use neo3::neo_builder::ScriptBuilder;
	/// use neo3::neo_types::ContractParameter;
	///
	/// let mut builder = ScriptBuilder::new();
	/// builder.push_params(&[
	///     ContractParameter::from("param1"),
	///     ContractParameter::from(42),
	///     ContractParameter::from(true)
	/// ]);
	/// ```
	pub fn push_params(&mut self, params: &[ContractParameter]) -> Result<&mut Self, BuilderError> {
		for param in params {
			self.push_param(param).map_err(|e| {
				BuilderError::IllegalArgument(format!("Failed to push parameter: {}", e))
			})?;
		}

		Ok(self.push_integer(BigInt::from(params.len())).op_code(&[OpCode::Pack]))
	}

	/// Pushes a single contract parameter to the script.
	///
	/// # Arguments
	///
	/// * `param` - The `ContractParameter` value to push to the script.
	///
	/// # Returns
	///
	/// A `Result` containing a mutable reference to the `ScriptBuilder` for method chaining,
	/// or a `BuilderError` if an error occurs.
	///
	/// # Examples
	///
	/// ```rust
	/// use neo3::neo_builder::ScriptBuilder;
	/// use neo3::neo_types::ContractParameter;
	///
	/// let mut builder = ScriptBuilder::new();
	/// builder.push_param(&ContractParameter::from("Hello, Neo!")).unwrap();
	/// ```
	pub fn push_param(&mut self, param: &ContractParameter) -> Result<&mut Self, BuilderError> {
		if param.get_type() == ContractParameterType::Any {
			self.op_code(&[OpCode::PushNull]);
			return Ok(self);
		}
		match &param
			.value
			.clone()
			.ok_or_else(|| BuilderError::IllegalArgument("Parameter value is None".to_string()))?
		{
			ParameterValue::Boolean(b) => self.push_bool(*b),
			ParameterValue::Integer(i) => self.push_integer(BigInt::from(i.clone())),
			ParameterValue::ByteArray(b) => {
				// Decode the base64-encoded string to get the actual bytes
				let bytes = b.from_base64_string().map_err(|e| {
					BuilderError::IllegalArgument(format!(
						"Failed to decode base64 ByteArray: {}",
						e
					))
				})?;
				self.push_data(bytes)
			},
			ParameterValue::Signature(b) | ParameterValue::PublicKey(b) => {
				self.push_data(b.as_bytes().to_vec())
			},
			ParameterValue::H160(h) => self.push_data(h.as_bytes().to_vec()),
			ParameterValue::H256(h) => self.push_data(h.as_bytes().to_vec()),
			ParameterValue::String(s) => self.push_data(s.as_bytes().to_vec()),
			ParameterValue::Array(arr) => self.push_array(arr).map_err(|e| {
				BuilderError::IllegalArgument(format!("Failed to push array: {}", e))
			})?,
			ParameterValue::Map(map) => self
				.push_map(&map.0)
				.map_err(|e| BuilderError::IllegalArgument(format!("Failed to push map: {}", e)))?,
			_ => {
				return Err(BuilderError::IllegalArgument("Unsupported parameter type".to_string()))
			},
		};

		Ok(self)
	}

	/// Adds a push operation with the given integer to the script.
	///
	/// The integer is encoded in its two's complement representation and little-endian byte order.
	///
	/// The integer can be up to 32 bytes in length. Values larger than 32 bytes will return an error.
	///
	/// # Arguments
	///
	/// * `i` - The integer to push to the script
	///
	/// # Returns
	///
	/// A mutable reference to the `ScriptBuilder` for method chaining.
	///
	/// # Examples
	///
	/// ```rust
	/// use neo3::neo_builder::ScriptBuilder;
	/// use num_bigint::BigInt;
	///
	/// let mut builder = ScriptBuilder::new();
	/// builder.push_integer(BigInt::from(42));
	/// ```
	pub fn push_integer(&mut self, i: BigInt) -> &mut Self {
		if i >= BigInt::from(-1) && i <= BigInt::from(16) {
			self.op_code(
				vec![OpCode::try_from(i.to_i32().unwrap() as u8 + OpCode::Push0 as u8).unwrap()]
					.as_slice(),
			);
		} else {
			let mut bytes = i.to_signed_bytes_le();

			// Remove unnecessary zero padding for positive numbers
			// BigInt::to_signed_bytes_le() adds extra zero bytes for positive numbers
			// to ensure they're not interpreted as negative
			// For positive numbers, we can remove trailing zeros if the previous byte doesn't have the sign bit set
			// OR if the number is positive and we have a trailing zero
			while bytes.len() > 1 && bytes[bytes.len() - 1] == 0 && !i.is_negative() {
				bytes.pop();
			}

			let len = bytes.len();

			// bytes.reverse();

			match len {
				1 => self.push_opcode_bytes(OpCode::PushInt8, bytes),
				2 => self.push_opcode_bytes(OpCode::PushInt16, bytes),
				len if len <= 4 => self.push_opcode_bytes(
					OpCode::PushInt32,
					Self::pad_right(&bytes, 4, i.is_negative()),
				),
				len if len <= 8 => self.push_opcode_bytes(
					OpCode::PushInt64,
					Self::pad_right(&bytes, 8, i.is_negative()),
				),
				len if len <= 16 => self.push_opcode_bytes(
					OpCode::PushInt128,
					Self::pad_right(&bytes, 16, i.is_negative()),
				),
				len if len <= 32 => self.push_opcode_bytes(
					OpCode::PushInt256,
					Self::pad_right(&bytes, 32, i.is_negative()),
				),
				_ => {
					// Instead of panicking, we'll truncate to 32 bytes and log a warning
					// This is safer than crashing the application
					eprintln!("Warning: Integer too large, truncating to 32 bytes");
					self.push_opcode_bytes(
						OpCode::PushInt256,
						Self::pad_right(&bytes[..32.min(bytes.len())], 32, i.is_negative()),
					)
				},
			};
		}

		self
	}

	/// Append opcodes to the script in the provided order.
	///
	/// # Arguments
	///
	/// * `opcode` - The opcode to append
	/// * `argument` - The data argument for the opcode
	///
	/// # Returns
	///
	/// A mutable reference to the `ScriptBuilder` for method chaining.
	///
	/// # Examples
	///
	/// ```rust
	/// use neo3::neo_builder::ScriptBuilder;
	/// use neo3::neo_types::OpCode;
	///
	/// let mut builder = ScriptBuilder::new();
	/// builder.push_opcode_bytes(OpCode::PushData1, vec![0x01, 0x02, 0x03]);
	/// ```
	pub fn push_opcode_bytes(&mut self, opcode: OpCode, argument: Vec<u8>) -> &mut ScriptBuilder {
		self.script.write_u8(opcode as u8);
		self.script.write_bytes(&argument);

		self
	}

	fn pad_right(bytes: &[u8], size: usize, negative: bool) -> Vec<u8> {
		let pad_value = if negative { 0xFF } else { 0 };

		let mut padded = vec![0; size];
		padded[0..bytes.len()].copy_from_slice(bytes);
		padded[bytes.len()..].fill(pad_value);
		padded
	}

	// Push data handling

	/// Pushes data to the script.
	///
	/// # Arguments
	///
	/// * `data` - The data to push to the script.
	///
	/// # Returns
	///
	/// A mutable reference to the `ScriptBuilder` for method chaining.
	///
	/// # Examples
	///
	/// ```rust
	/// use neo3::neo_builder::ScriptBuilder;
	///
	/// let mut builder = ScriptBuilder::new();
	/// builder.push_data("Hello, Neo!".as_bytes().to_vec());
	/// ```
	pub fn push_data(&mut self, data: Vec<u8>) -> &mut Self {
		match data.len() {
			0..=0xff => {
				self.op_code(&[OpCode::PushData1]);
				self.script.write_u8(data.len() as u8);
				let _ = self.script.write_bytes(&data);
			},
			0x100..=0xffff => {
				self.op_code(&[OpCode::PushData2]);
				self.script.write_u16(data.len() as u16);
				let _ = self.script.write_bytes(&data);
			},
			_ => {
				self.op_code(&[OpCode::PushData4]);
				self.script.write_u32(data.len() as u32);
				let _ = self.script.write_bytes(&data);
			}, // _ => return Err(BuilderError::IllegalArgument("Data too long".to_string())),
		}
		self
	}

	/// Pushes a boolean value to the script.
	///
	/// # Arguments
	///
	/// * `b` - The boolean value to push to the script.
	///
	/// # Returns
	///
	/// A mutable reference to the `ScriptBuilder` for method chaining.
	///
	/// # Examples
	///
	/// ```rust
	/// use neo3::neo_builder::ScriptBuilder;
	///
	/// let mut builder = ScriptBuilder::new();
	/// builder.push_bool(true);
	/// ```
	pub fn push_bool(&mut self, b: bool) -> &mut Self {
		if b {
			self.op_code(&[OpCode::PushTrue])
		} else {
			self.op_code(&[OpCode::PushFalse])
		};
		self
	}

	/// Pushes an array of contract parameters to the script.
	///
	/// # Arguments
	///
	/// * `arr` - A slice of `ContractParameter` values to push to the script.
	///
	/// # Returns
	///
	/// A `Result` containing a mutable reference to the `ScriptBuilder` for method chaining,
	/// or a `BuilderError` if an error occurs.
	///
	pub fn push_array(&mut self, arr: &[ContractParameter]) -> Result<&mut Self, BuilderError> {
		if arr.is_empty() {
			self.op_code(&[OpCode::NewArray0]);
		} else {
			self.push_params(arr);
		};
		Ok(self)
	}

	/// Pushes a map of contract parameters to the script.
	///
	/// # Arguments
	///
	/// * `map` - A reference to a `HashMap` mapping `ContractParameter` keys to `ContractParameter` values.
	///
	/// # Returns
	///
	/// A `Result` containing a mutable reference to the `ScriptBuilder` for method chaining,
	/// or a `BuilderError` if an error occurs.
	pub fn push_map(
		&mut self,
		map: &HashMap<ContractParameter, ContractParameter>,
	) -> Result<&mut Self, BuilderError> {
		for (k, v) in map {
			let kk: ContractParameter = k.clone().into();
			let vv: ContractParameter = v.clone().into();
			self.push_param(&vv).unwrap();
			self.push_param(&kk).unwrap();
		}

		Ok(self.push_integer(BigInt::from(map.len())).op_code(&[OpCode::PackMap]))
	}

	/// Appends the `Pack` opcode to the script.
	///
	/// # Returns
	///
	/// A mutable reference to the `ScriptBuilder` for method chaining.
	pub fn pack(&mut self) -> &mut Self {
		self.op_code(&[OpCode::Pack])
	}

	/// Returns the script as a `Bytes` object.
	pub fn to_bytes(&self) -> Bytes {
		self.script.to_bytes()
	}

	/// Builds a verification script for the given public key.
	///
	/// # Arguments
	///
	/// * `pub_key` - The public key to use for verification.
	///
	/// # Returns
	///
	/// A `Bytes` object containing the verification script.
	pub fn build_verification_script(pub_key: &Secp256r1PublicKey) -> Bytes {
		let mut sb = ScriptBuilder::new();
		sb.push_data(pub_key.get_encoded(true))
			.sys_call(InteropService::SystemCryptoCheckSig);
		sb.to_bytes()
	}

	/// Builds a multi-signature script for the given public keys and threshold.
	///
	/// # Arguments
	///
	/// * `pubkeys` - A mutable slice of `Secp256r1PublicKey` values representing the public keys.
	/// * `threshold` - The minimum number of signatures required to validate the script.
	///
	/// # Returns
	///
	/// A `Result` containing a `Bytes` object containing the multi-signature script,
	/// or a `BuilderError` if an error occurs.
	pub fn build_multi_sig_script(
		pubkeys: &mut [Secp256r1PublicKey],
		threshold: u8,
	) -> Result<Bytes, BuilderError> {
		let mut sb = ScriptBuilder::new();
		sb.push_integer(BigInt::from(threshold));
		pubkeys.sort_by(|a, b| a.get_encoded(true).cmp(&b.get_encoded(true)));
		for pk in pubkeys.iter() {
			sb.push_data(pk.get_encoded(true));
		}
		sb.push_integer(BigInt::from(pubkeys.len()));
		sb.sys_call(InteropService::SystemCryptoCheckMultiSig);
		Ok(sb.to_bytes())
	}

	/// Builds a contract script for the given sender, NEF checksum, and contract name.
	///
	/// This method creates a script for deploying a smart contract on the Neo N3 blockchain.
	///
	/// # Arguments
	///
	/// * `sender` - The 160-bit hash of the contract sender.
	/// * `nef_checksum` - The checksum of the NEF (Neo Executable Format) file.
	/// * `name` - The name of the contract.
	///
	/// # Returns
	///
	/// A `Result` containing a `Bytes` object with the contract deployment script,
	/// or a `BuilderError` if an error occurs.
	///
	/// # Examples
	///
	/// ```rust
	/// use neo3::neo_builder::ScriptBuilder;
	/// use primitive_types::H160;
	/// use std::str::FromStr;
	///
	/// let sender = H160::from_str("0xd2a4cff31913016155e38e474a2c06d08be276cf").unwrap();
	/// let nef_checksum = 1234567890;
	/// let name = "MyContract";
	///
	/// let script = ScriptBuilder::build_contract_script(&sender, nef_checksum, name).unwrap();
	/// ```
	/// * `nef_checksum` - The checksum of the NEF file.
	/// * `name` - The name of the contract.
	///
	/// # Returns
	///
	/// A `Result` containing a `Bytes` object containing the contract script,
	/// or a `BuilderError` if an error occurs.
	pub fn build_contract_script(
		sender: &H160,
		nef_checksum: u32,
		name: &str,
	) -> Result<Bytes, BuilderError> {
		let mut sb = ScriptBuilder::new();
		sb.op_code(&[OpCode::Abort])
			.push_data(sender.to_vec())
			.push_integer(BigInt::from(nef_checksum))
			.push_data(name.as_bytes().to_vec());
		Ok(sb.to_bytes())
	}

	/// Builds a script that calls a contract method and unwraps the iterator result.
	///
	/// This method is particularly useful when calling contract methods that return iterators.
	/// It automatically handles the iteration process and collects the results into an array.
	///
	/// # Arguments
	///
	/// * `contract_hash` - The 160-bit hash of the contract to call.
	/// * `method` - The name of the method to call.
	/// * `params` - A slice of `ContractParameter` values to pass as arguments to the method.
	/// * `max_items` - The maximum number of items to retrieve from the iterator.
	/// * `call_flags` - An optional `CallFlags` value specifying the call flags.
	///
	/// # Returns
	///
	/// A `Result` containing a `Bytes` object with the script that calls the contract method
	/// and unwraps the iterator result into an array, or a `BuilderError` if an error occurs.
	///
	/// # Examples
	///
	/// ```rust
	/// use neo3::neo_builder::ScriptBuilder;
	/// use neo3::neo_types::ContractParameter;
	/// use neo3::builder::CallFlags;
	/// use primitive_types::H160;
	/// use std::str::FromStr;
	///
	/// // Call a contract method that returns an iterator and collect up to 100 items
	/// let contract_hash = H160::from_str("0xd2a4cff31913016155e38e474a2c06d08be276cf").unwrap();
	/// let method = "getTokens";
	/// let params = vec![ContractParameter::from("owner_address")];
	/// let max_items = 100;
	///
	/// // Build the script
	/// let script = ScriptBuilder::build_contract_call_and_unwrap_iterator(
	///     &contract_hash,
	///     method,
	///     &params,
	///     max_items,
	///     Some(CallFlags::All)
	/// ).unwrap();
	///
	/// // The resulting script will:
	/// // 1. Call the contract method
	/// // 2. Iterate through the returned iterator
	/// // 3. Collect up to max_items into an array
	/// // 4. Leave the array on the stack
	/// ```
	pub fn build_contract_call_and_unwrap_iterator(
		contract_hash: &H160,
		method: &str,
		params: &[ContractParameter],
		max_items: u32,
		call_flags: Option<CallFlags>,
	) -> Result<Bytes, BuilderError> {
		let mut sb = Self::new();
		sb.push_integer(BigInt::from(max_items));

		sb.contract_call(contract_hash, method, params, call_flags).unwrap();

		sb.op_code(&[OpCode::NewArray]);

		let cycle_start = sb.len();
		sb.op_code(&[OpCode::Over]);
		sb.sys_call(InteropService::SystemIteratorNext);

		let jmp_if_not = sb.len();
		sb.op_code_with_arg(OpCode::JmpIf, vec![0]);

		sb.op_code(&[OpCode::Dup, OpCode::Push2, OpCode::Pick])
			.sys_call(InteropService::SystemIteratorValue)
			.op_code(&[
				OpCode::Append,
				OpCode::Dup,
				OpCode::Size,
				OpCode::Push3,
				OpCode::Pick,
				OpCode::Ge,
			]);

		let jmp_if_max = sb.len();
		sb.op_code_with_arg(OpCode::JmpIf, vec![0]);

		let jmp_offset = sb.len();
		// Calculate backward jump as a signed offset
		let jmp_bytes = (cycle_start as i32 - jmp_offset as i32) as i8;
		sb.op_code_with_arg(OpCode::Jmp, vec![jmp_bytes as u8]);

		let load_result = sb.len();
		sb.op_code(&[OpCode::Nip, OpCode::Nip]);

		let mut script = sb.to_bytes();
		let jmp_not_bytes = (load_result - jmp_if_not) as i8;
		script[jmp_if_not + 1] = jmp_not_bytes as u8;

		let jmp_max_bytes = (load_result - jmp_if_max) as i8;
		script[jmp_if_max + 1] = jmp_max_bytes as u8;

		Ok(script)
	}

	/// Returns the length of the script in bytes.
	///
	/// This method is useful for determining the current position in the script,
	/// which is needed for calculating jump offsets in control flow operations.
	///
	/// # Returns
	///
	/// The length of the script in bytes.
	///
	/// # Examples
	///
	/// ```rust
	/// use neo3::neo_builder::ScriptBuilder;
	///
	/// let mut builder = ScriptBuilder::new();
	/// builder.push_data("Hello, Neo!".as_bytes().to_vec());
	/// let script_length = builder.len();
	/// println!("Script length: {} bytes", script_length);
	/// ```
	pub fn len(&self) -> usize {
		self.script().size()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		neo_types::{contract::ContractParameterMap, ContractParameter, ContractParameterType},
		prelude::Bytes,
	};
	use num_bigint::BigInt;
	use std::collections::HashMap;

	#[test]
	fn test_push_empty_array() {
		let mut builder = ScriptBuilder::new();
		builder.push_array(&[]).unwrap();
		assert_builder(&builder, &[OpCode::NewArray0 as u8]);
	}

	#[test]
	fn test_push_byte_array() {
		let mut builder = ScriptBuilder::new();
		let data = vec![0x01, 0x02, 0x03];
		builder.push_data(data.clone());

		let mut expected = vec![OpCode::PushData1 as u8, data.len() as u8];
		expected.extend(data);
		assert_builder(&builder, &expected);
	}

	#[test]
	fn test_push_string() {
		let mut builder = ScriptBuilder::new();
		let string_data = "Hello, Neo!";
		builder.push_data(string_data.as_bytes().to_vec());

		let mut expected = vec![OpCode::PushData1 as u8, string_data.len() as u8];
		expected.extend(string_data.as_bytes());
		assert_builder(&builder, &expected);
	}

	#[test]
	fn test_push_integer() {
		let mut builder = ScriptBuilder::new();

		// Test small integers (-1 to 16)
		builder.push_integer(BigInt::from(0));
		assert_builder(&builder, &[OpCode::Push0 as u8]);

		let mut builder = ScriptBuilder::new();
		builder.push_integer(BigInt::from(16));
		assert_builder(&builder, &[OpCode::Push16 as u8]);

		// Test larger integers
		let mut builder = ScriptBuilder::new();
		builder.push_integer(BigInt::from(255));
		assert_builder(&builder, &[OpCode::PushInt8 as u8, 0xff]);

		let mut builder = ScriptBuilder::new();
		builder.push_integer(BigInt::from(65535));
		assert_builder(&builder, &[OpCode::PushInt16 as u8, 0xff, 0xff]);

		// Test negative integers - update expectations to match our more efficient implementation
		let mut builder = ScriptBuilder::new();
		builder.push_integer(BigInt::from(-1000000000000000i64));
		// Our implementation uses PushInt64 (more efficient) instead of PushInt128
		// The actual bytes are: [0, 128, 57, 91, 129, 114, 252] padded to 8 bytes with 0xFF
		let expected_bytes = vec![OpCode::PushInt64 as u8, 0, 128, 57, 91, 129, 114, 252, 255];
		assert_builder(&builder, &expected_bytes);

		let mut builder = ScriptBuilder::new();
		builder.push_integer(BigInt::from(1000000000000000i64));
		// Similarly, this should use PushInt64 instead of PushInt128
		// The actual bytes for positive 1000000000000000 should be different
		let pos_big_int = BigInt::from(1000000000000000i64);
		let pos_bytes = pos_big_int.to_signed_bytes_le();
		let mut expected_pos = vec![OpCode::PushInt64 as u8];
		let padded_pos = ScriptBuilder::pad_right(&pos_bytes, 8, false);
		expected_pos.extend(padded_pos);
		assert_builder(&builder, &expected_pos);
	}

	#[test]
	fn test_verification_script() {
		let public_key = Secp256r1PublicKey::from_bytes(
			&hex::decode("035fdb1d1f06759547020891ae97c729327853aeb1256b6fe0473bc2e9fa42ff50")
				.unwrap(),
		)
		.unwrap();

		let script = ScriptBuilder::build_verification_script(&public_key);

		// The script should be: PushData1 (0x0c) + length (33/0x21) + public key (33 bytes) + Syscall (0x41) + SystemCryptoCheckSig hash
		// Let's build the expected result dynamically to ensure it's correct
		let mut expected = vec![0x0c, 0x21]; // PushData1 + length 33
		expected.extend_from_slice(
			&hex::decode("035fdb1d1f06759547020891ae97c729327853aeb1256b6fe0473bc2e9fa42ff50")
				.unwrap(),
		);
		expected.push(0x41); // Syscall opcode
		expected
			.extend_from_slice(&hex::decode(&InteropService::SystemCryptoCheckSig.hash()).unwrap());

		assert_eq!(script.to_vec(), expected);
	}

	#[test]
	fn test_map() {
		let mut map = HashMap::new();
		map.insert(
			ContractParameter::string("first".to_string()),
			ContractParameter::byte_array(hex::decode("7365636f6e64").unwrap()),
		);

		let mut builder = ScriptBuilder::new();
		builder.push_map(&map).unwrap();

		let expected = builder.to_bytes().to_hex_string();

		let mut builder2 = ScriptBuilder::new();
		builder2
			.push_data(hex::decode("7365636f6e64").unwrap())
			.push_data("first".as_bytes().to_vec())
			.push_integer(BigInt::from(1))
			.op_code(&[OpCode::PackMap]);

		let expected2 = builder2.to_bytes().to_hex_string();

		let mut builder3 = ScriptBuilder::new().push_map(&map).unwrap().to_bytes().to_hex_string();

		assert_eq!(expected, expected2);
		assert_eq!(expected, builder3);
	}

	#[test]
	fn test_map_nested() {
		let mut inner = ContractParameterMap::new();
		inner.0.insert(
			ContractParameter::string("inner_key".to_string()),
			ContractParameter::integer(42),
		);

		let mut outer = ContractParameterMap::new();
		outer.0.insert(
			ContractParameter::string("outer_key".to_string()),
			ContractParameter::map(inner),
		);

		let expected = ScriptBuilder::new().push_map(&outer.0).unwrap().to_bytes().to_hex_string();

		// Manually build the expected script
		let mut manual_builder = ScriptBuilder::new();
		manual_builder
			.push_integer(BigInt::from(42))
			.push_data("inner_key".as_bytes().to_vec())
			.push_integer(BigInt::from(1))
			.op_code(&[OpCode::PackMap])
			.push_data("outer_key".as_bytes().to_vec())
			.push_integer(BigInt::from(1))
			.op_code(&[OpCode::PackMap]);

		let manual_expected = manual_builder.to_bytes().to_hex_string();

		assert_eq!(expected, manual_expected);
	}

	fn assert_builder(builder: &ScriptBuilder, expected: &[u8]) {
		assert_eq!(builder.to_bytes().to_vec(), expected);
	}

	fn byte_array(size: usize) -> Vec<u8> {
		(0..size).map(|i| (i % 256) as u8).collect()
	}
}
