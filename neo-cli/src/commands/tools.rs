use crate::{
	commands::wallet::CliState,
	errors::CliError,
	utils_core::{create_table, print_section_header, print_success, status_indicator},
};
use base64::{Engine as _, engine::general_purpose};
use clap::{Args, Subcommand};
use colored::*;
use comfy_table::{Cell, Color};
use neo3::{
	neo_clients::{public_key_to_address, public_key_to_script_hash},
	neo_crypto::{
		Secp256r1PublicKey, Secp256r1Signature, base58check_encode, try_base58check_decode,
	},
	neo_types::{AddressExtension, ScriptHash, ScriptHashExtension},
};
use ripemd::Ripemd160;
use sha2::{Digest, Sha256};

const BASE58_ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const NEO_N3_ADDRESS_VERSION: u8 = 53;

fn strip_hex_prefix(input: &str) -> &str {
	input.strip_prefix("0x").or_else(|| input.strip_prefix("0X")).unwrap_or(input)
}

fn decode_hex_input(input: &str) -> Result<Vec<u8>, CliError> {
	hex::decode(strip_hex_prefix(input)).map_err(|e| CliError::InvalidInput(e.to_string()))
}

fn encode_base58(data: &[u8]) -> String {
	if data.is_empty() {
		return String::new();
	}

	let leading_zeroes = data.iter().take_while(|byte| **byte == 0).count();
	let mut digits: Vec<u8> = Vec::new();

	for byte in data {
		let mut carry = *byte as u32;
		for digit in &mut digits {
			let value = (*digit as u32) * 256 + carry;
			*digit = (value % 58) as u8;
			carry = value / 58;
		}
		while carry > 0 {
			digits.push((carry % 58) as u8);
			carry /= 58;
		}
	}

	let mut encoded = String::with_capacity(leading_zeroes + digits.len());
	for _ in 0..leading_zeroes {
		encoded.push('1');
	}
	for digit in digits.iter().rev() {
		encoded.push(BASE58_ALPHABET[*digit as usize] as char);
	}
	encoded
}

fn decode_base58(input: &str) -> Result<Vec<u8>, CliError> {
	if input.is_empty() {
		return Ok(Vec::new());
	}

	let leading_zeroes = input.bytes().take_while(|byte| *byte == b'1').count();
	let mut bytes: Vec<u8> = Vec::new();

	for byte in input.bytes() {
		let value =
			BASE58_ALPHABET.iter().position(|candidate| *candidate == byte).ok_or_else(|| {
				CliError::InvalidInput(format!("Invalid base58 character: {}", byte as char))
			})? as u32;

		let mut carry = value;
		for item in &mut bytes {
			let value = (*item as u32) * 58 + carry;
			*item = (value & 0xff) as u8;
			carry = value >> 8;
		}
		while carry > 0 {
			bytes.push((carry & 0xff) as u8);
			carry >>= 8;
		}
	}

	let mut decoded = vec![0u8; leading_zeroes];
	decoded.extend(bytes.iter().rev());
	Ok(decoded)
}

fn parse_tool_input(input: &str, format: &str) -> Result<Vec<u8>, CliError> {
	match format.to_ascii_lowercase().as_str() {
		"text" | "utf8" | "utf-8" => Ok(input.as_bytes().to_vec()),
		"hex" => decode_hex_input(input),
		"base64" => general_purpose::STANDARD
			.decode(input)
			.map_err(|e| CliError::InvalidInput(e.to_string())),
		"base58" => decode_base58(input),
		"base58check" => {
			try_base58check_decode(input).map_err(|e| CliError::InvalidInput(e.to_string()))
		},
		"file" => std::fs::read(input).map_err(|e| CliError::Io(e)),
		_ => Err(CliError::InvalidInput(format!(
			"Unsupported format '{}'. Use text, hex, base64, base58, base58check, or file.",
			format
		))),
	}
}

fn format_tool_output(data: &[u8], format: &str) -> Result<String, CliError> {
	match format.to_ascii_lowercase().as_str() {
		"text" | "utf8" | "utf-8" => Ok(String::from_utf8_lossy(data).to_string()),
		"hex" => Ok(hex::encode(data)),
		"base64" => Ok(general_purpose::STANDARD.encode(data)),
		"base58" => Ok(encode_base58(data)),
		"base58check" => Ok(base58check_encode(data)),
		_ => Err(CliError::InvalidInput(format!(
			"Unsupported output format '{}'. Use text, hex, base64, base58, or base58check.",
			format
		))),
	}
}

#[derive(Args, Debug)]
pub struct ToolsArgs {
	#[command(subcommand)]
	pub command: ToolsCommands,
}

#[derive(Subcommand, Debug)]
pub enum ToolsCommands {
	/// Encode data in various formats
	#[command(about = "Encode data in various formats")]
	Encode {
		/// Input data
		#[arg(short, long, help = "Data to encode")]
		input: String,

		/// Encoding format (base64, hex, base58, base58check)
		#[arg(short, long, default_value = "base64", help = "Encoding format")]
		format: String,

		/// Input format (text, hex, file)
		#[arg(long, default_value = "text", help = "Input data format")]
		input_format: String,
	},

	/// Decode data from various formats
	#[command(about = "Decode data from various formats")]
	Decode {
		/// Encoded data
		#[arg(short, long, help = "Data to decode")]
		input: String,

		/// Decoding format (base64, hex, base58, base58check)
		#[arg(short, long, default_value = "base64", help = "Decoding format")]
		format: String,

		/// Output format (text, hex, base64, base58, base58check, file)
		#[arg(long, default_value = "text", help = "Output data format")]
		output_format: String,
	},

	/// Generate hash of data
	#[command(about = "Generate hash of data")]
	Hash {
		/// Input data
		#[arg(short, long, help = "Data to hash")]
		input: String,

		/// Hash algorithm (sha256, ripemd160, sha1, md5)
		#[arg(short, long, default_value = "sha256", help = "Hash algorithm")]
		algorithm: String,

		/// Input format (text, hex, file)
		#[arg(long, default_value = "text", help = "Input data format")]
		input_format: String,

		/// Output format (hex, base64)
		#[arg(long, default_value = "hex", help = "Output format")]
		output_format: String,
	},

	/// Convert between different formats
	#[command(about = "Convert between different formats")]
	Convert {
		/// Input data
		#[arg(short, long, help = "Data to convert")]
		input: String,

		/// Source format
		#[arg(short, long, help = "Source format")]
		from: String,

		/// Target format
		#[arg(short, long, help = "Target format")]
		to: String,
	},

	/// Generate Neo address from public key
	#[command(about = "Generate Neo address from public key")]
	Address {
		/// Public key (hex format)
		#[arg(short, long, help = "Public key in hex format")]
		pubkey: String,

		/// Address version (53 for Neo N3)
		#[arg(short, long, default_value = "53", help = "Address version")]
		version: u8,
	},

	/// Validate Neo address
	#[command(about = "Validate Neo address")]
	ValidateAddress {
		/// Address to validate
		#[arg(short, long, help = "Neo address to validate")]
		address: String,
	},

	/// Generate script hash from contract script
	#[command(about = "Generate script hash from contract script")]
	ScriptHash {
		/// Contract script (hex format)
		#[arg(short, long, help = "Contract script in hex format")]
		script: String,
	},

	/// Generate random data
	#[command(about = "Generate random data")]
	Random {
		/// Number of bytes to generate
		#[arg(short, long, default_value = "32", help = "Number of random bytes")]
		bytes: usize,

		/// Output format (hex, base64, base58, base58check)
		#[arg(short, long, default_value = "hex", help = "Output format")]
		format: String,
	},

	/// Verify signature
	#[command(about = "Verify digital signature")]
	VerifySignature {
		/// Message that was signed
		#[arg(short, long, help = "Original message")]
		message: String,

		/// Signature to verify
		#[arg(short, long, help = "Signature in hex format")]
		signature: String,

		/// Public key for verification
		#[arg(short, long, help = "Public key in hex format")]
		pubkey: String,
	},

	/// Calculate transaction fee
	#[command(about = "Calculate transaction fee")]
	CalculateFee {
		/// Transaction size in bytes
		#[arg(short, long, help = "Transaction size in bytes")]
		size: u64,

		/// Network fee per byte
		#[arg(short, long, default_value = "1000", help = "Network fee per byte")]
		fee_per_byte: u64,

		/// System fee
		#[arg(long, default_value = "0", help = "System fee")]
		system_fee: u64,
	},

	/// Format JSON data
	#[command(about = "Format and validate JSON data")]
	FormatJson {
		/// JSON data to format
		#[arg(short, long, help = "JSON data to format")]
		input: String,

		/// Compact output
		#[arg(short, long, help = "Compact JSON output")]
		compact: bool,
	},
}

/// Handle tools command with comprehensive functionality
pub async fn handle_tools_command(args: ToolsArgs, _state: &mut CliState) -> Result<(), CliError> {
	match args.command {
		ToolsCommands::Encode { input, format, input_format } => {
			handle_encode(input, format, input_format).await
		},
		ToolsCommands::Decode { input, format, output_format } => {
			handle_decode(input, format, output_format).await
		},
		ToolsCommands::Hash { input, algorithm, input_format, output_format } => {
			handle_hash(input, algorithm, input_format, output_format).await
		},
		ToolsCommands::Convert { input, from, to } => handle_convert(input, from, to).await,
		ToolsCommands::Address { pubkey, version } => {
			handle_address_generation(pubkey, version).await
		},
		ToolsCommands::ValidateAddress { address } => handle_validate_address(address).await,
		ToolsCommands::ScriptHash { script } => handle_script_hash(script).await,
		ToolsCommands::Random { bytes, format } => handle_random_generation(bytes, format).await,
		ToolsCommands::VerifySignature { message, signature, pubkey } => {
			handle_verify_signature(message, signature, pubkey).await
		},
		ToolsCommands::CalculateFee { size, fee_per_byte, system_fee } => {
			handle_calculate_fee(size, fee_per_byte, system_fee).await
		},
		ToolsCommands::FormatJson { input, compact } => handle_format_json(input, compact).await,
	}
}

/// Encode data in various formats
async fn handle_encode(
	input: String,
	format: String,
	input_format: String,
) -> Result<(), CliError> {
	print_section_header("Data Encoding");

	let data = parse_tool_input(&input, &input_format)?;
	let encoded = format_tool_output(&data, &format)?;

	let mut table = create_table();
	table.add_row(vec![
		Cell::new("Input Format").fg(Color::Cyan),
		Cell::new(&input_format).fg(Color::Green),
	]);
	table.add_row(vec![
		Cell::new("Output Format").fg(Color::Cyan),
		Cell::new(&format).fg(Color::Green),
	]);
	table.add_row(vec![
		Cell::new("Input Size").fg(Color::Cyan),
		Cell::new(format!("{} bytes", data.len())).fg(Color::Yellow),
	]);
	table.add_row(vec![
		Cell::new("Encoded Data").fg(Color::Cyan),
		Cell::new(&encoded).fg(Color::Blue),
	]);

	println!("{table}");
	print_success("✅ Data encoded successfully!");

	Ok(())
}

/// Decode data from various formats
async fn handle_decode(
	input: String,
	format: String,
	output_format: String,
) -> Result<(), CliError> {
	print_section_header("Data Decoding");

	let decoded = parse_tool_input(&input, &format)?;

	// Format output
	let output = if output_format.eq_ignore_ascii_case("file") {
		let filename = format!("decoded_output_{}.bin", chrono::Utc::now().timestamp());
		std::fs::write(&filename, &decoded).map_err(|e| CliError::Io(e))?;
		format!("Saved to file: {}", filename)
	} else {
		format_tool_output(&decoded, &output_format)?
	};

	let mut table = create_table();
	table.add_row(vec![
		Cell::new("Input Format").fg(Color::Cyan),
		Cell::new(&format).fg(Color::Green),
	]);
	table.add_row(vec![
		Cell::new("Output Format").fg(Color::Cyan),
		Cell::new(&output_format).fg(Color::Green),
	]);
	table.add_row(vec![
		Cell::new("Decoded Size").fg(Color::Cyan),
		Cell::new(format!("{} bytes", decoded.len())).fg(Color::Yellow),
	]);
	table.add_row(vec![
		Cell::new("Decoded Data").fg(Color::Cyan),
		Cell::new(&output).fg(Color::Blue),
	]);

	println!("{table}");
	print_success("✅ Data decoded successfully!");

	Ok(())
}

/// Generate hash of data
async fn handle_hash(
	input: String,
	algorithm: String,
	input_format: String,
	output_format: String,
) -> Result<(), CliError> {
	print_section_header("Data Hashing");

	// Parse input
	let data = match input_format.as_str() {
		"text" => input.as_bytes().to_vec(),
		"hex" => decode_hex_input(&input)?,
		"file" => std::fs::read(&input).map_err(|e| CliError::Io(e))?,
		_ => return Err(CliError::InvalidInput("Invalid input format".to_string())),
	};

	// Generate hash
	let hash_bytes = match algorithm.as_str() {
		"sha256" => {
			let mut hasher = Sha256::new();
			hasher.update(&data);
			hasher.finalize().to_vec()
		},
		"ripemd160" => {
			let mut hasher = Ripemd160::new();
			hasher.update(&data);
			hasher.finalize().to_vec()
		},
		_ => return Err(CliError::InvalidInput("Unsupported hash algorithm".to_string())),
	};

	// Format output
	let hash_output = match output_format.as_str() {
		"hex" => hex::encode(&hash_bytes),
		"base64" => general_purpose::STANDARD.encode(&hash_bytes),
		_ => return Err(CliError::InvalidInput("Invalid output format".to_string())),
	};

	let mut table = create_table();
	table.add_row(vec![
		Cell::new("Algorithm").fg(Color::Cyan),
		Cell::new(&algorithm.to_uppercase()).fg(Color::Green),
	]);
	table.add_row(vec![
		Cell::new("Input Size").fg(Color::Cyan),
		Cell::new(format!("{} bytes", data.len())).fg(Color::Yellow),
	]);
	table.add_row(vec![
		Cell::new("Hash Size").fg(Color::Cyan),
		Cell::new(format!("{} bytes", hash_bytes.len())).fg(Color::Yellow),
	]);
	table.add_row(vec![Cell::new("Hash").fg(Color::Cyan), Cell::new(&hash_output).fg(Color::Blue)]);

	println!("{table}");
	print_success("✅ Hash generated successfully!");

	Ok(())
}

/// Generate random data
async fn handle_random_generation(bytes: usize, format: String) -> Result<(), CliError> {
	print_section_header("Random Data Generation");

	use rand::RngCore;
	let mut rng = rand::rng();
	let mut random_bytes = vec![0u8; bytes];
	rng.fill_bytes(&mut random_bytes);

	let output = match format.as_str() {
		"hex" => hex::encode(&random_bytes),
		"base64" => general_purpose::STANDARD.encode(&random_bytes),
		"base58" => encode_base58(&random_bytes),
		"base58check" => base58check_encode(&random_bytes),
		_ => return Err(CliError::InvalidInput("Invalid output format".to_string())),
	};

	let mut table = create_table();
	table.add_row(vec![
		Cell::new("Bytes Generated").fg(Color::Cyan),
		Cell::new(bytes.to_string()).fg(Color::Green),
	]);
	table.add_row(vec![
		Cell::new("Output Format").fg(Color::Cyan),
		Cell::new(&format).fg(Color::Green),
	]);
	table.add_row(vec![
		Cell::new("Random Data").fg(Color::Cyan),
		Cell::new(&output).fg(Color::Blue),
	]);

	println!("{table}");
	print_success("🎲 Random data generated successfully!");

	Ok(())
}

/// Calculate transaction fee
async fn handle_calculate_fee(
	size: u64,
	fee_per_byte: u64,
	system_fee: u64,
) -> Result<(), CliError> {
	print_section_header("Transaction Fee Calculation");

	let network_fee = size * fee_per_byte;
	let total_fee = network_fee + system_fee;

	let mut table = create_table();
	table.add_row(vec![
		Cell::new("Transaction Size").fg(Color::Cyan),
		Cell::new(format!("{} bytes", size)).fg(Color::Green),
	]);
	table.add_row(vec![
		Cell::new("Fee Per Byte").fg(Color::Cyan),
		Cell::new(format!("{} GAS", fee_per_byte)).fg(Color::Green),
	]);
	table.add_row(vec![
		Cell::new("Network Fee").fg(Color::Cyan),
		Cell::new(format!("{} GAS", network_fee)).fg(Color::Yellow),
	]);
	table.add_row(vec![
		Cell::new("System Fee").fg(Color::Cyan),
		Cell::new(format!("{} GAS", system_fee)).fg(Color::Yellow),
	]);
	table.add_row(vec![
		Cell::new("Total Fee").fg(Color::Cyan),
		Cell::new(format!("{} GAS", total_fee)).fg(Color::Blue),
	]);

	println!("{table}");
	print_success("💰 Transaction fee calculated successfully!");

	Ok(())
}

/// Format JSON data
async fn handle_format_json(input: String, compact: bool) -> Result<(), CliError> {
	print_section_header("JSON Formatting");

	let parsed: serde_json::Value = serde_json::from_str(&input)
		.map_err(|e| CliError::InvalidInput(format!("Invalid JSON: {}", e)))?;

	let formatted = if compact {
		serde_json::to_string(&parsed).map_err(|e| CliError::InvalidInput(e.to_string()))?
	} else {
		serde_json::to_string_pretty(&parsed).map_err(|e| CliError::InvalidInput(e.to_string()))?
	};

	let mut table = create_table();
	table.add_row(vec![
		Cell::new("Format").fg(Color::Cyan),
		Cell::new(if compact { "Compact" } else { "Pretty" }).fg(Color::Green),
	]);
	table.add_row(vec![
		Cell::new("Valid JSON").fg(Color::Cyan),
		Cell::new(format!("{} Yes", status_indicator("success"))).fg(Color::Green),
	]);

	println!("{table}");
	println!("\n{}", "Formatted JSON:".bright_green().bold());
	println!("{formatted}");

	print_success("✅ JSON formatted successfully!");

	Ok(())
}

async fn handle_convert(input: String, from: String, to: String) -> Result<(), CliError> {
	print_section_header("Format Conversion");

	let data = parse_tool_input(&input, &from)?;
	let output = format_tool_output(&data, &to)?;

	let mut table = create_table();
	table.add_row(vec![Cell::new("From").fg(Color::Cyan), Cell::new(&from).fg(Color::Green)]);
	table.add_row(vec![Cell::new("To").fg(Color::Cyan), Cell::new(&to).fg(Color::Green)]);
	table.add_row(vec![
		Cell::new("Decoded Size").fg(Color::Cyan),
		Cell::new(format!("{} bytes", data.len())).fg(Color::Yellow),
	]);
	table.add_row(vec![Cell::new("Output").fg(Color::Cyan), Cell::new(&output).fg(Color::Blue)]);

	println!("{table}");
	print_success("✅ Data converted successfully!");
	Ok(())
}

async fn handle_address_generation(pubkey: String, version: u8) -> Result<(), CliError> {
	print_section_header("Neo Address Generation");

	if version != NEO_N3_ADDRESS_VERSION {
		return Err(CliError::InvalidInput(format!(
			"Unsupported address version {}. Neo N3 uses version {}.",
			version, NEO_N3_ADDRESS_VERSION
		)));
	}

	let pubkey_bytes = decode_hex_input(&pubkey)?;
	let public_key = Secp256r1PublicKey::from_bytes(&pubkey_bytes)
		.map_err(|e| CliError::InvalidInput(format!("Invalid public key: {}", e)))?;
	let address = public_key_to_address(&public_key);
	let script_hash = public_key_to_script_hash(&public_key);

	let mut table = create_table();
	table.add_row(vec![Cell::new("Address").fg(Color::Cyan), Cell::new(&address).fg(Color::Green)]);
	table.add_row(vec![
		Cell::new("Script Hash").fg(Color::Cyan),
		Cell::new(script_hash.to_hex_big_endian()).fg(Color::Blue),
	]);
	table.add_row(vec![
		Cell::new("Script Hash LE").fg(Color::Cyan),
		Cell::new(script_hash.to_hex()).fg(Color::Blue),
	]);
	table.add_row(vec![
		Cell::new("Public Key").fg(Color::Cyan),
		Cell::new(public_key.get_encoded_compressed_hex()).fg(Color::Yellow),
	]);
	table.add_row(vec![
		Cell::new("Version").fg(Color::Cyan),
		Cell::new(version.to_string()).fg(Color::Yellow),
	]);

	println!("{table}");
	print_success("✅ Address generated successfully!");
	Ok(())
}

async fn handle_validate_address(address: String) -> Result<(), CliError> {
	print_section_header("Neo Address Validation");

	let script_hash = address
		.address_to_script_hash()
		.map_err(|e| CliError::InvalidInput(format!("Invalid Neo address: {}", e)))?;

	let mut table = create_table();
	table.add_row(vec![Cell::new("Address").fg(Color::Cyan), Cell::new(&address).fg(Color::Green)]);
	table.add_row(vec![
		Cell::new("Valid").fg(Color::Cyan),
		Cell::new(format!("{} Yes", status_indicator("success"))).fg(Color::Green),
	]);
	table.add_row(vec![
		Cell::new("Script Hash").fg(Color::Cyan),
		Cell::new(script_hash.to_hex_big_endian()).fg(Color::Blue),
	]);
	table.add_row(vec![
		Cell::new("Script Hash LE").fg(Color::Cyan),
		Cell::new(script_hash.to_hex()).fg(Color::Blue),
	]);

	println!("{table}");
	print_success("✅ Address is valid!");
	Ok(())
}

async fn handle_script_hash(script: String) -> Result<(), CliError> {
	print_section_header("Script Hash");

	let script_bytes = decode_hex_input(&script)?;
	let script_hash = <ScriptHash as ScriptHashExtension>::from_script(&script_bytes);

	let mut table = create_table();
	table.add_row(vec![
		Cell::new("Script Size").fg(Color::Cyan),
		Cell::new(format!("{} bytes", script_bytes.len())).fg(Color::Yellow),
	]);
	table.add_row(vec![
		Cell::new("Script Hash").fg(Color::Cyan),
		Cell::new(script_hash.to_hex_big_endian()).fg(Color::Blue),
	]);
	table.add_row(vec![
		Cell::new("Script Hash LE").fg(Color::Cyan),
		Cell::new(script_hash.to_hex()).fg(Color::Blue),
	]);
	table.add_row(vec![
		Cell::new("Address").fg(Color::Cyan),
		Cell::new(script_hash.to_address()).fg(Color::Green),
	]);

	println!("{table}");
	print_success("✅ Script hash generated successfully!");
	Ok(())
}

async fn handle_verify_signature(
	message: String,
	signature: String,
	pubkey: String,
) -> Result<(), CliError> {
	print_section_header("Signature Verification");

	let signature_bytes = decode_hex_input(&signature)?;
	let pubkey_bytes = decode_hex_input(&pubkey)?;
	let signature = Secp256r1Signature::from_bytes(&signature_bytes)
		.map_err(|e| CliError::InvalidInput(format!("Invalid signature: {}", e)))?;
	let public_key = Secp256r1PublicKey::from_bytes(&pubkey_bytes)
		.map_err(|e| CliError::InvalidInput(format!("Invalid public key: {}", e)))?;

	public_key
		.verify(message.as_bytes(), &signature)
		.map_err(|e| CliError::InvalidInput(format!("Signature verification failed: {}", e)))?;

	let mut table = create_table();
	table.add_row(vec![
		Cell::new("Message Size").fg(Color::Cyan),
		Cell::new(format!("{} bytes", message.len())).fg(Color::Yellow),
	]);
	table.add_row(vec![
		Cell::new("Public Key").fg(Color::Cyan),
		Cell::new(public_key.get_encoded_compressed_hex()).fg(Color::Blue),
	]);
	table.add_row(vec![
		Cell::new("Valid").fg(Color::Cyan),
		Cell::new(format!("{} Yes", status_indicator("success"))).fg(Color::Green),
	]);

	println!("{table}");
	print_success("✅ Signature is valid!");
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn base58_known_vectors() {
		let cases = [
			("", ""),
			(" ", "Z"),
			("-", "n"),
			("0", "q"),
			("1", "r"),
			("-1", "4SU"),
			("11", "4k8"),
			("abc", "ZiCa"),
			("1234598760", "3mJr7AoUXx2Wqd"),
		];

		for (plain, encoded) in cases {
			assert_eq!(encode_base58(plain.as_bytes()), encoded);
			assert_eq!(decode_base58(encoded).unwrap(), plain.as_bytes());
		}
	}

	#[test]
	fn base58_preserves_leading_zeroes() {
		let data = [0, 0, 1, 2, 3, 255];
		let encoded = encode_base58(&data);
		assert!(encoded.starts_with("11"));
		assert_eq!(decode_base58(&encoded).unwrap(), data);
	}

	#[test]
	fn base58_rejects_invalid_characters() {
		assert!(decode_base58("0OIl").is_err());
	}
}
