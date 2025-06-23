use crate::{
	builder::{BuilderError, InteropService},
	codec::Decoder,
	neo_crypto::utils::ToHexString,
	Bytes, OpCode, OperandSize,
};
use tokio::io::AsyncReadExt;

/// A utility struct for reading and interpreting Neo smart contract scripts.
pub struct ScriptReader;

impl ScriptReader {
	/// Retrieves the InteropService code from a given hash.
	///
	/// # Arguments
	///
	/// * `_hash` - A string representation of the hash.
	///
	/// # Returns
	///
	/// An Option containing the InteropService if found, or None if not found.
	///
	/// # Example
	///
	/// ```rust
	/// use neo3::neo_builder::ScriptReader;
	///
	/// let hash = "9bf667ce".to_string();
	/// if let Some(_service) = ScriptReader::get_interop_service_code(hash) {
	///     println!("InteropService found");
	/// } else {
	///     println!("InteropService not found");
	/// }
	/// ```
	pub fn get_interop_service_code(_hash: String) -> Option<InteropService> {
		InteropService::from_hash(_hash)
	}

	/// Converts a byte script to a human-readable string of OpCodes.
	///
	/// # Arguments
	///
	/// * `script` - The byte script to convert.
	///
	/// # Returns
	///
	/// A string representation of the OpCodes in the script.
	///
	/// # Example
	///
	/// ```rust
	/// use neo3::neo_builder::ScriptReader;
	///
	/// let script = hex::decode("0c0548656c6c6f").unwrap();
	/// let op_code_string = ScriptReader::convert_to_op_code_string(&script);
	/// println!("OpCodes: {}", op_code_string);
	/// // Output: OpCodes: PUSHDATA1 5 48656c6c6f
	/// ```
	pub fn convert_to_op_code_string(script: &Bytes) -> String {
		let mut reader = Decoder::new(script);
		let mut result = String::new();

		while reader.pointer().clone() < script.len() {
			if let Ok(op_code) = OpCode::try_from(reader.read_u8()) {
				// Add the OpCode to the result string
				result.push_str(&format!("{:?}", op_code).to_uppercase());

				// Handle operands if present
				if let Some(size) = op_code.operand_size() {
					if size.size().clone() > 0 {
						// Fixed size operand
						result.push_str(&format!(
							" {}",
							reader
								.read_bytes(size.size().clone() as usize)
								.unwrap()
								.to_hex_string()
						));
					} else if size.prefix_size().clone() > 0 {
						// Variable size operand with prefix
						let prefix_size = Self::get_prefix_size(&mut reader, size).unwrap();
						result.push_str(&format!(
							" {} {}",
							prefix_size,
							reader.read_bytes(prefix_size).unwrap().to_hex_string()
						));
					}
				}
				result.push('\n');
			}
		}
		result
	}

	/// Helper function to get the size of a variable-length operand.
	///
	/// # Arguments
	///
	/// * `reader` - The Decoder to read from.
	/// * `size` - The OperandSize specifying the prefix size.
	///
	/// # Returns
	///
	/// A Result containing the size of the operand or a BuilderError.
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use neo3::neo_builder::ScriptReader;
	/// use neo3::neo_codec::Decoder;
	/// use neo3::neo_types::OperandSize;
	///
	/// let mut decoder = Decoder::new(&[0x05]); // Example: prefix size of 5
	/// let operand_size = OperandSize::with_prefix_size(1); // 1-byte prefix
	/// // Note: get_prefix_size is a private function used internally
	/// ```
	fn get_prefix_size(reader: &mut Decoder, size: OperandSize) -> Result<usize, BuilderError> {
		match size.prefix_size() {
			1 => Ok(reader.read_u8() as usize),
			2 => Ok(reader.read_i16().map(|v| v as usize)?),
			4 => Ok(reader.read_i32().map(|v| v as usize)?),
			_ => Err(BuilderError::UnsupportedOperation(
				"Only operand prefix sizes 1, 2, and 4 are supported".to_string(),
			)),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_convert_to_op_code_string() {
		// Test script in hexadecimal format
		let script =
			hex::decode("0c0548656c6c6f0c05576f726c642150419bf667ce41e63f18841140").unwrap();

		// Expected output after conversion
		let expected_op_code_string = "PUSHDATA1 5 48656c6c6f\nPUSHDATA1 5 576f726c64\nNOP\nSWAP\nSYSCALL 9bf667ce\nSYSCALL e63f1884\nPUSH1\nRET\n";

		// Convert the script to OpCode string
		let op_code_string = ScriptReader::convert_to_op_code_string(&script);

		// Assert that the conversion matches the expected output
		assert_eq!(op_code_string.as_str(), expected_op_code_string);
	}
}
