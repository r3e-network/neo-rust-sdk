use crate::{
	OpCode,
	codec::{CodecError, NeoSerializable},
};
/// This module provides a binary decoder that can read various types of data from a byte slice.
///
/// # Examples
///
/// ```rust
///
/// use neo3::neo_codec::Decoder;
/// let data = [0x01, 0x02, 0x03, 0x04];
/// let mut decoder = Decoder::new(&data);
///
/// assert_eq!(decoder.read_bool(), true);
/// assert_eq!(decoder.read_u8(), 2);
/// // Note: These examples are simplified - actual methods return Results
/// // assert_eq!(decoder.read_u16().unwrap(), 0x0403);
/// // assert_eq!(decoder.read_i16().unwrap(), 0x0403);
/// ```
use getset::{Getters, Setters};
use num_bigint::{BigInt, Sign};
use serde::Deserialize;
use serde::Serialize;

/// A binary decoder that can read various types of data from a byte slice.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, Getters, Setters)]
pub struct Decoder<'a> {
	data: &'a [u8],
	#[getset(get = "pub")]
	pointer: usize,
	marker: usize,
}

impl<'a> Iterator for Decoder<'a> {
	type Item = u8;

	/// Returns the next byte in the byte slice, or None if the end of the slice has been reached.
	fn next(&mut self) -> Option<Self::Item> {
		if self.pointer < self.data.len() {
			let val = self.data[self.pointer];
			self.pointer += 1;
			Some(val)
		} else {
			None
		}
	}
}

impl<'a> Decoder<'a> {
	/// Creates a new binary decoder that reads from the given byte slice.
	pub fn new(data: &'a [u8]) -> Self {
		Self { data, pointer: 0, marker: 0 }
	}

	/// Reads an unsigned 8-bit integer from the byte slice.
	///
	/// Unlike [`Self::read_u8`], this method returns an error instead of panicking when the
	/// decoder is at EOF.
	pub fn read_u8_safe(&mut self) -> Result<u8, CodecError> {
		if self.pointer >= self.data.len() {
			return Err(CodecError::IndexOutOfBounds("Read beyond end of buffer".to_string()));
		}
		let val = self.data[self.pointer];
		self.pointer += 1;
		Ok(val)
	}

	/// Reads a boolean value from the byte slice.
	///
	/// Unlike [`Self::read_bool`], this method returns an error instead of panicking when the
	/// decoder is at EOF.
	pub fn read_bool_safe(&mut self) -> Result<bool, CodecError> {
		Ok(self.read_u8_safe()? == 1)
	}

	/// Reads a boolean value from the byte slice.
	///
	/// # Panics
	///
	/// Panics if the buffer has no more bytes to read.
	/// For a fallible alternative, use [`Self::read_bool_safe`].
	pub fn read_bool(&mut self) -> bool {
		assert!(self.pointer < self.data.len(), "read_bool: buffer underflow");
		let val = self.data[self.pointer] == 1;
		self.pointer += 1;
		val
	}

	/// Reads an unsigned 8-bit integer from the byte slice.
	///
	/// # Panics
	///
	/// Panics if the buffer has no more bytes to read.
	/// For a fallible alternative, use [`Self::read_u8_safe`].
	pub fn read_u8(&mut self) -> u8 {
		assert!(self.pointer < self.data.len(), "read_u8: buffer underflow");
		let val = self.data[self.pointer];
		self.pointer += 1;
		val
	}

	/// Reads an unsigned 16-bit integer from the byte slice.
	pub fn read_u16(&mut self) -> Result<u16, CodecError> {
		let bytes = self.read_bytes(2)?;
		bytes
			.try_into()
			.map(u16::from_le_bytes)
			.map_err(|_| CodecError::InvalidEncoding("Failed to convert bytes to u16".to_string()))
	}

	/// Reads a signed 16-bit integer from the byte slice.
	pub fn read_i16(&mut self) -> Result<i16, CodecError> {
		let bytes = self.read_bytes(2)?;
		bytes
			.try_into()
			.map(i16::from_le_bytes)
			.map_err(|_| CodecError::InvalidEncoding("Failed to convert bytes to i16".to_string()))
	}

	/// Reads an unsigned 32-bit integer from the byte slice.
	pub fn read_u32(&mut self) -> Result<u32, CodecError> {
		let bytes = self.read_bytes(4)?;
		bytes
			.try_into()
			.map(u32::from_le_bytes)
			.map_err(|_| CodecError::InvalidEncoding("Failed to convert bytes to u32".to_string()))
	}

	/// Reads a signed 32-bit integer from the byte slice.
	pub fn read_i32(&mut self) -> Result<i32, CodecError> {
		let bytes = self.read_bytes(4)?;
		bytes
			.try_into()
			.map(i32::from_le_bytes)
			.map_err(|_| CodecError::InvalidEncoding("Failed to convert bytes to i32".to_string()))
	}

	/// Reads an unsigned 64-bit integer from the byte slice.
	pub fn read_u64(&mut self) -> Result<u64, CodecError> {
		let bytes = self.read_bytes(8)?;
		bytes
			.try_into()
			.map(u64::from_le_bytes)
			.map_err(|_| CodecError::InvalidEncoding("Failed to convert bytes to u64".to_string()))
	}

	/// Reads a signed 64-bit integer from the byte slice.
	pub fn read_i64(&mut self) -> Result<i64, CodecError> {
		let bytes = self.read_bytes(8)?;
		bytes
			.try_into()
			.map(i64::from_le_bytes)
			.map_err(|_| CodecError::InvalidEncoding("Failed to convert bytes to i64".to_string()))
	}

	pub fn read_bigint(&mut self) -> Result<BigInt, CodecError> {
		let byte = self.read_u8_safe()?;

		let negative = byte & 0x80 != 0;
		let len = match byte {
			0..=0x4b => 1,
			0x4c => self.read_u8_safe()? as usize,
			0x4d => self.read_u16()? as usize,
			0x4e => self.read_u32()? as usize,
			_ => return Err(CodecError::InvalidFormat),
		};

		let mut bytes = self.read_bytes(len)?;
		if negative {
			// Flip sign bit
			if let Some(byte) = bytes.last_mut() {
				*byte ^= 0x80;
			} else if len == 0 {
				return Err(CodecError::InvalidFormat);
			}
		}
		// Note: NEO uses little-endian byte order for BigIntegers
		// The sign is determined by the 'negative' flag above
		let sign = if negative { Sign::Minus } else { Sign::Plus };
		Ok(BigInt::from_bytes_le(sign, &bytes))
	}

	/// Reads an encoded EC point (33 bytes: 1-byte prefix + 32-byte X coordinate) from the byte slice.
	pub fn read_encoded_ec_point(&mut self) -> Result<Vec<u8>, CodecError> {
		let prefix = self.read_u8_safe()?;
		match prefix {
			0x02 | 0x03 => {
				let coord = self.read_bytes(32)?;
				let mut point = Vec::with_capacity(33);
				point.push(prefix);
				point.extend_from_slice(&coord);
				Ok(point)
			},
			_ => Err(CodecError::InvalidEncoding("Invalid encoded EC point".to_string())),
		}
	}

	/// Reads a byte slice of the given length from the byte slice.
	pub fn read_bytes(&mut self, length: usize) -> Result<Vec<u8>, CodecError> {
		let end = self
			.pointer
			.checked_add(length)
			.ok_or_else(|| CodecError::IndexOutOfBounds("Read beyond end of buffer".to_string()))?;
		if end > self.data.len() {
			return Err(CodecError::IndexOutOfBounds("Read beyond end of buffer".to_string()));
		}
		let result = self.data[self.pointer..end].to_vec();
		self.pointer = end;
		Ok(result)
	}

	/// Reads a variable-length byte slice from the byte slice.
	pub fn read_var_bytes(&mut self) -> Result<Vec<u8>, CodecError> {
		let len = self
			.read_var_int()?
			.try_into()
			.map_err(|_| CodecError::InvalidEncoding("Invalid length".into()))?;
		self.read_bytes(len)
	}

	/// Reads a variable-length byte slice from the byte slice, enforcing a maximum length.
	pub fn read_var_bytes_bounded(&mut self, max_len: usize) -> Result<Vec<u8>, CodecError> {
		let len: usize = self
			.read_var_int()?
			.try_into()
			.map_err(|_| CodecError::InvalidEncoding("Invalid length".into()))?;
		if len > max_len {
			return Err(CodecError::InvalidEncoding(format!(
				"VarBytes length {len} exceeds maximum {max_len}"
			)));
		}
		self.read_bytes(len)
	}

	/// Reads a variable-length integer from the byte slice.
	///
	/// Validates canonical (minimal) encoding: multi-byte forms must not be used
	/// for values that fit in a shorter form.
	pub fn read_var_int(&mut self) -> Result<i64, CodecError> {
		let first = self.read_u8_safe()?;
		match first {
			0xfd => {
				let v = self.read_u16()?;
				if v < 0xFD {
					return Err(CodecError::InvalidEncoding(
						"Non-canonical VarInt: 3-byte form used for value < 0xFD".to_string(),
					));
				}
				Ok(v as i64)
			},
			0xfe => {
				let v = self.read_u32()?;
				if v < 0x10000 {
					return Err(CodecError::InvalidEncoding(
						"Non-canonical VarInt: 5-byte form used for value < 0x10000".to_string(),
					));
				}
				Ok(v as i64)
			},
			0xff => {
				let v = self.read_u64()?;
				if v < 0x100000000 {
					return Err(CodecError::InvalidEncoding(
						"Non-canonical VarInt: 9-byte form used for value < 0x100000000"
							.to_string(),
					));
				}
				i64::try_from(v).map_err(|_| {
					CodecError::InvalidEncoding("VarInt value too large for i64".to_string())
				})
			},
			_ => Ok(first as i64),
		}
	}

	pub fn read_var_string(&mut self) -> Result<String, CodecError> {
		let bytes = self.read_var_bytes()?;

		let string =
			String::from_utf8(bytes).map_err(|e| CodecError::InvalidEncoding(e.to_string()))?;

		// Trim null bytes from end
		let string = string.trim_end_matches(char::from(0));

		Ok(string.to_string())
	}

	pub fn read_var_string_bounded(&mut self, max_len: usize) -> Result<String, CodecError> {
		let bytes = self.read_var_bytes_bounded(max_len)?;

		let string =
			String::from_utf8(bytes).map_err(|e| CodecError::InvalidEncoding(e.to_string()))?;

		// Trim null bytes from end
		let string = string.trim_end_matches(char::from(0));

		Ok(string.to_string())
	}

	/// Reads a push byte slice from the byte slice.
	pub fn read_push_bytes(&mut self) -> Result<Vec<u8>, CodecError> {
		let opcode = self.read_u8_safe()?;
		let len =
			match OpCode::try_from(opcode)? {
				OpCode::PushData1 => self.read_u8_safe()? as usize,
				OpCode::PushData2 => self.read_u16().map_err(|e| {
					CodecError::InvalidEncoding(format!("Failed to read u16: {}", e))
				})? as usize,
				OpCode::PushData4 => self.read_u32().map_err(|e| {
					CodecError::InvalidEncoding(format!("Failed to read u32: {}", e))
				})? as usize,
				_ => return Err(CodecError::InvalidOpCode),
			};

		self.read_bytes(len)
	}

	/// Reads a push integer from the byte slice.
	pub fn read_push_int(&mut self) -> Result<BigInt, CodecError> {
		let byte = self.read_u8_safe()?;

		if (OpCode::PushM1 as u8..=OpCode::Push16 as u8).contains(&byte) {
			return Ok(BigInt::from(byte as i8 - OpCode::Push0 as i8));
		}

		let count = match OpCode::try_from(byte)? {
			OpCode::PushInt8 => 1,
			OpCode::PushInt16 => 2,
			OpCode::PushInt32 => 4,
			OpCode::PushInt64 => 8,
			OpCode::PushInt128 => 16,
			OpCode::PushInt256 => 32,
			_ => {
				return Err(CodecError::InvalidEncoding(
					"Couldn't parse PUSHINT OpCode".to_string(),
				));
			},
		};

		let bytes = self.read_bytes(count)?;
		Ok(BigInt::from_signed_bytes_le(&bytes))
	}

	/// Reads a push string from the byte slice.
	pub fn read_push_string(&mut self) -> Result<String, CodecError> {
		let bytes = self.read_push_bytes()?;
		String::from_utf8(bytes)
			.map_err(|_| CodecError::InvalidEncoding("Invalid UTF-8".to_string()))
	}

	/// Reads a deserializable value from the byte slice.
	pub fn read_serializable<T: NeoSerializable>(&mut self) -> Result<T, CodecError> {
		T::decode(self).map_err(|_e| CodecError::InvalidFormat)
	}

	fn read_serializable_list_len(&mut self) -> Result<usize, CodecError> {
		let len = self.read_var_int()?;
		len.try_into()
			.map_err(|_| CodecError::InvalidEncoding("Invalid list length".into()))
	}

	/// Reads a list of deserializable values from the byte slice, enforcing a maximum length.
	pub fn read_serializable_list_bounded<T: NeoSerializable>(
		&mut self,
		max_len: usize,
	) -> Result<Vec<T>, CodecError> {
		let len = self.read_serializable_list_len()?;
		if len > max_len {
			return Err(CodecError::InvalidEncoding(format!(
				"List length {len} exceeds maximum {max_len}"
			)));
		}
		if len > self.available() {
			return Err(CodecError::InvalidEncoding(
				"List length exceeds remaining bytes".to_string(),
			));
		}

		let mut list = Vec::with_capacity(len);
		for _ in 0..len {
			list.push(T::decode(self).map_err(|_e| CodecError::InvalidFormat)?);
		}
		Ok(list)
	}

	/// Reads a list of deserializable values from the byte slice.
	pub fn read_serializable_list<T: NeoSerializable>(&mut self) -> Result<Vec<T>, CodecError> {
		let len = self.read_serializable_list_len()?;
		if len > self.available() {
			return Err(CodecError::InvalidEncoding(
				"List length exceeds remaining bytes".to_string(),
			));
		}

		let mut list = Vec::with_capacity(len);
		for _ in 0..len {
			list.push(T::decode(self).map_err(|_e| CodecError::InvalidFormat)?);
		}
		Ok(list)
	}

	pub fn read_serializable_list_var_bytes<T: NeoSerializable>(
		&mut self,
	) -> Result<Vec<T>, CodecError> {
		let len = self.read_serializable_list_len()?;
		if len > self.available() {
			return Err(CodecError::InvalidEncoding(
				"List length exceeds remaining bytes".to_string(),
			));
		}

		let start = self.pointer;
		let end = start
			.checked_add(len)
			.ok_or_else(|| CodecError::InvalidEncoding("List length overflow".into()))?;

		let mut list = Vec::with_capacity(len);
		while self.pointer < end {
			let before = self.pointer;
			list.push(T::decode(self).map_err(|_e| CodecError::InvalidFormat)?);
			if self.pointer == before {
				return Err(CodecError::InvalidFormat);
			}
		}

		if self.pointer != end {
			return Err(CodecError::InvalidFormat);
		}

		Ok(list)
	}

	pub fn mark(&mut self) {
		self.marker = self.pointer;
	}

	pub fn reset(&mut self) {
		self.pointer = self.marker;
	}

	pub fn available(&self) -> usize {
		self.data.len().saturating_sub(self.pointer)
	}
}

#[cfg(test)]
mod tests {
	use crate::codec::Decoder;
	use num_bigint::BigInt;

	#[test]
	fn test_read_u16_is_little_endian() {
		let bytes = [0x00_u8, 0x01_u8];
		assert_eq!(
			Decoder::new(&bytes)
				.read_u16()
				.expect("read_u16 should decode [0x00, 0x01] as 256"),
			256
		);

		let bytes = [0x01_u8, 0x00_u8];
		assert_eq!(
			Decoder::new(&bytes)
				.read_u16()
				.expect("read_u16 should decode [0x01, 0x00] as 1"),
			1
		);
	}

	#[test]
	fn test_read_var_int_u16_is_little_endian() {
		// 256 encoded as VarInt: 0xfd 0x00 0x01 (u16 LE)
		let bytes = [0xfd_u8, 0x00_u8, 0x01_u8];
		assert_eq!(
			Decoder::new(&bytes)
				.read_var_int()
				.expect("read_var_int should decode VarInt 256 (0xfd 0x00 0x01)"),
			256
		);
	}

	#[test]
	fn test_read_var_bytes_bounded_rejects_excess_length() {
		let bytes = [3_u8, b'a', b'b', b'c'];
		let err = Decoder::new(&bytes).read_var_bytes_bounded(2).unwrap_err();
		assert!(matches!(err, crate::codec::CodecError::InvalidEncoding(_)));
	}

	#[test]
	fn test_read_var_string_bounded_rejects_excess_length() {
		let bytes = [3_u8, b'a', b'b', b'c'];
		let err = Decoder::new(&bytes).read_var_string_bounded(2).unwrap_err();
		assert!(matches!(err, crate::codec::CodecError::InvalidEncoding(_)));
	}

	#[test]
	fn test_read_push_data_bytes() {
		let prefix_count_map = [
			(hex::decode("0c01").expect("hex decode should succeed for prefix 0x0c01 (1 byte)"), 1),
			(
				hex::decode("0cff")
					.expect("hex decode should succeed for prefix 0x0cff (255 bytes)"),
				255,
			),
			(
				hex::decode("0d0001")
					.expect("hex decode should succeed for prefix 0x0d0001 (256 bytes)"),
				256,
			),
			(
				hex::decode("0d0010")
					.expect("hex decode should succeed for prefix 0x0d0010 (4096 bytes)"),
				4096,
			),
			(
				hex::decode("0e00000100")
					.expect("hex decode should succeed for prefix 0x0e00000100 (65536 bytes)"),
				65536,
			),
		];

		for (prefix, count) in prefix_count_map {
			let bytes = vec![1u8; count];
			let data = [prefix.as_slice(), bytes.as_slice()].concat();
			assert_eq!(
				Decoder::new(&data).read_push_bytes().unwrap_or_else(|_| panic!(
					"read_push_bytes should decode {} bytes of push data",
					count
				)),
				bytes
			);
		}
	}

	#[test]
	fn test_fail_read_push_data() {
		let data =
			hex::decode("4b010000").expect("hex decode should succeed for test data 0x4b010000");
		let err = Decoder::new(&data).read_push_bytes().unwrap_err();
		assert_eq!(err.to_string(), "Invalid op code")
	}

	#[test]
	fn test_read_push_data_string() {
		let empty = hex::decode("0c00")
			.expect("hex decode should succeed for empty push data prefix 0x0c00");
		assert_eq!(
			Decoder::new(&empty)
				.read_push_string()
				.expect("read_push_string should decode empty string"),
			""
		);

		let a = hex::decode("0c0161")
			.expect("hex decode should succeed for push data 'a' prefix 0x0c0161");
		assert_eq!(
			Decoder::new(&a)
				.read_push_string()
				.expect("read_push_string should decode single character 'a'"),
			"a"
		);

		let bytes = vec![0u8; 10000];
		let input = [
			hex::decode("0e10270000")
				.expect("hex decode should succeed for 10000-byte prefix 0x0e10270000"),
			bytes.as_slice().to_vec(),
		]
		.concat();
		let expected = String::from_utf8(bytes).expect("bytes should be valid UTF-8");

		assert_eq!(
			Decoder::new(&input)
				.read_push_string()
				.expect("read_push_string should decode 10000 null bytes"),
			expected
		);
	}

	#[test]
	fn test_read_push_data_big_integer() {
		let zero = hex::decode("10").expect("hex decode should succeed for PUSH0 opcode 0x10");
		assert_eq!(
			Decoder::new(&zero)
				.read_push_int()
				.expect("read_push_int should decode PUSH0 as 0"),
			BigInt::from(0)
		);

		let one = hex::decode("11").expect("hex decode should succeed for PUSH1 opcode 0x11");
		assert_eq!(
			Decoder::new(&one)
				.read_push_int()
				.expect("read_push_int should decode PUSH1 as 1"),
			BigInt::from(1)
		);

		let minus_one =
			hex::decode("0f").expect("hex decode should succeed for PUSHM1 opcode 0x0f");
		assert_eq!(
			Decoder::new(&minus_one)
				.read_push_int()
				.expect("read_push_int should decode PUSHM1 as -1"),
			BigInt::from(-1)
		);

		let sixteen = hex::decode("20").expect("hex decode should succeed for PUSH16 opcode 0x20");
		assert_eq!(
			Decoder::new(&sixteen)
				.read_push_int()
				.expect("read_push_int should decode PUSH16 as 16"),
			BigInt::from(16)
		);
	}

	#[test]
	fn test_read_u32() {
		let max = [0xffu8; 4];
		assert_eq!(
			Decoder::new(&max)
				.read_u32()
				.expect("read_u32 should decode max u32 value [0xff; 4]"),
			4_294_967_295
		);

		let one = hex::decode("01000000")
			.expect("hex decode should succeed for u32 value 1 (little-endian 0x01000000)");
		assert_eq!(
			Decoder::new(&one)
				.read_u32()
				.expect("read_u32 should decode little-endian 0x01000000 as 1"),
			1
		);

		let zero = [0u8; 4];
		assert_eq!(
			Decoder::new(&zero).read_u32().expect("read_u32 should decode zero [0; 4] as 0"),
			0
		);

		let custom = hex::decode("8cae0000ff")
			.expect("hex decode should succeed for custom u32 value 0x8cae0000");
		assert_eq!(
			Decoder::new(&custom)
				.read_u32()
				.expect("read_u32 should decode little-endian 0x8cae0000 as 44684"),
			44_684
		);
	}

	#[test]
	fn test_read_i64() {
		let min = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80];
		assert_eq!(
			Decoder::new(&min).read_i64().expect("read_i64 should decode i64::MIN"),
			i64::MIN
		);

		let max = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f];
		assert_eq!(
			Decoder::new(&max).read_i64().expect("read_i64 should decode i64::MAX"),
			i64::MAX
		);

		let zero = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
		assert_eq!(Decoder::new(&zero).read_i64().expect("read_i64 should decode zero as 0"), 0);

		let custom = [0x11, 0x33, 0x22, 0x8c, 0xae, 0x00, 0x00, 0x00, 0xff];
		assert_eq!(
			Decoder::new(&custom)
				.read_i64()
				.expect("read_i64 should decode custom little-endian value as 749675361041"),
			749_675_361_041
		);
	}

	#[test]
	fn test_read_serializable_list_rejects_length_exceeding_remaining_bytes() {
		// List length = 2, but only 1 element byte remains.
		let bytes = [0x02_u8, 0x01_u8];
		let err = Decoder::new(&bytes).read_serializable_list::<u8>().unwrap_err();
		assert_eq!(err.to_string(), "Invalid encoding: List length exceeds remaining bytes");
	}

	#[test]
	fn test_read_serializable_list_bounded_rejects_excess_length() {
		let bytes = [0x03_u8, 0x01_u8, 0x02_u8, 0x03_u8];
		let err = Decoder::new(&bytes).read_serializable_list_bounded::<u8>(2).unwrap_err();
		assert_eq!(err.to_string(), "Invalid encoding: List length 3 exceeds maximum 2");
	}
}
