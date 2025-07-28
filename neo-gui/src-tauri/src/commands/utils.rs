use crate::{ApiResponse, AppState};
use base64::{engine::general_purpose, Engine as _};
use neo3::{neo_protocol::AccountTrait, ScriptHashExtension};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tauri::{command, State};

#[derive(Debug, Deserialize)]
pub struct EncodeDataRequest {
	pub data: String,
	pub encoding: String, // "hex", "base64", "base58"
}

#[derive(Debug, Deserialize)]
pub struct DecodeDataRequest {
	pub data: String,
	pub encoding: String, // "hex", "base64", "base58"
}

#[derive(Debug, Deserialize)]
pub struct HashDataRequest {
	pub data: String,
	pub algorithm: String, // "sha256", "ripemd160", "hash160", "hash256"
}

#[derive(Debug, Deserialize)]
pub struct ValidateAddressRequest {
	pub address: String,
}

#[derive(Debug, Deserialize)]
pub struct AddressToScriptHashRequest {
	pub address: String,
}

#[derive(Debug, Deserialize)]
pub struct ScriptHashToAddressRequest {
	pub script_hash: String,
}

#[derive(Debug, Deserialize)]
pub struct DerivePublicKeyRequest {
	pub private_key: String,
}

#[derive(Debug, Deserialize)]
pub struct FormatAmountRequest {
	pub amount: String,
	pub decimals: u8,
	pub symbol: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ValidationResult {
	pub valid: bool,
	pub address_type: Option<String>,
	pub script_hash: Option<String>,
	pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FormattedAmount {
	pub formatted: String,
	pub raw: String,
	pub decimals: u8,
	pub symbol: String,
}

#[derive(Debug, Serialize)]
pub struct AddressValidationResponse {
	pub is_valid: bool,
	pub address_type: String,
	pub script_hash: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PrivateKeyGenerationResponse {
	pub private_key: String,
	pub public_key: String,
	pub address: String,
}

#[derive(Debug, Serialize)]
pub struct PublicKeyResponse {
	pub public_key: String,
	pub address: String,
}

/// Encode data to specified format
#[command]
pub async fn encode_data(
	request: EncodeDataRequest,
	_state: State<'_, AppState>,
) -> Result<ApiResponse<String>, String> {
	log::info!("Encoding data to {}", request.encoding);

	let result = match request.encoding.as_str() {
		"hex" => hex::encode(request.data.as_bytes()),
		"base64" => general_purpose::STANDARD.encode(request.data.as_bytes()),
		"base58" => {
			// Base58 encoding requires the bs58 crate for production use
			// Add bs58 = "0.5" to Cargo.toml dependencies for full base58 support
			return Ok(ApiResponse::error("Base58 encoding requires additional dependencies. Please add bs58 crate to Cargo.toml for production use.".to_string()));
		},
		_ => {
			return Ok(ApiResponse::error(format!("Unsupported encoding: {}", request.encoding)));
		},
	};

	Ok(ApiResponse::success(result))
}

/// Decode data from specified format
#[command]
pub async fn decode_data(
	request: DecodeDataRequest,
	_state: State<'_, AppState>,
) -> Result<ApiResponse<String>, String> {
	log::info!("Decoding data from {}", request.encoding);

	let result = match request.encoding.as_str() {
		"hex" => match hex::decode(&request.data) {
			Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
			Err(_) => return Ok(ApiResponse::error("Invalid hex data".to_string())),
		},
		"base64" => match general_purpose::STANDARD.decode(&request.data) {
			Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
			Err(_) => return Ok(ApiResponse::error("Invalid base64 data".to_string())),
		},
		"base58" => {
			// Base58 decoding requires the bs58 crate for production use
			// Add bs58 = "0.5" to Cargo.toml dependencies for full base58 support
			return Ok(ApiResponse::error("Base58 decoding requires additional dependencies. Please add bs58 crate to Cargo.toml for production use.".to_string()));
		},
		_ => {
			return Ok(ApiResponse::error(format!("Unsupported encoding: {}", request.encoding)));
		},
	};

	Ok(ApiResponse::success(result))
}

/// Hash data using specified algorithm
#[command]
pub async fn hash_data(
	request: HashDataRequest,
	_state: State<'_, AppState>,
) -> Result<ApiResponse<String>, String> {
	log::info!("Hashing data with {}", request.algorithm);

	let data_bytes = request.data.as_bytes();

	let result = match request.algorithm.as_str() {
		"sha256" => {
			use sha2::{Digest, Sha256};
			let mut hasher = Sha256::new();
			hasher.update(data_bytes);
			hex::encode(hasher.finalize())
		},
		"ripemd160" => {
			use ripemd::{Digest, Ripemd160};
			let mut hasher = Ripemd160::new();
			hasher.update(data_bytes);
			hex::encode(hasher.finalize())
		},
		"hash160" => {
			// SHA256 followed by RIPEMD160
			use ripemd::Ripemd160;
			use sha2::{Digest, Sha256};

			let mut sha256 = Sha256::new();
			sha256.update(data_bytes);
			let sha256_result = sha256.finalize();

			let mut ripemd160 = Ripemd160::new();
			ripemd160.update(&sha256_result);
			hex::encode(ripemd160.finalize())
		},
		"hash256" => {
			// Double SHA256
			use sha2::{Digest, Sha256};

			let mut hasher1 = Sha256::new();
			hasher1.update(data_bytes);
			let first_hash = hasher1.finalize();

			let mut hasher2 = Sha256::new();
			hasher2.update(&first_hash);
			hex::encode(hasher2.finalize())
		},
		_ => {
			return Ok(ApiResponse::error("Unsupported hash algorithm".to_string()));
		},
	};

	log::info!("Data hashed successfully");
	Ok(ApiResponse::success(result))
}

/// Validate a Neo address
#[command]
pub async fn validate_address(
	request: ValidateAddressRequest,
	_state: State<'_, AppState>,
) -> Result<ApiResponse<AddressValidationResponse>, String> {
	log::info!("Validating address: {}", request.address);

	// Professional Neo address validation using Neo SDK
	// This implementation provides comprehensive address format validation
	let is_valid = neo3::neo_types::script_hash::ScriptHash::from_address(&request.address).is_ok();

	let result = AddressValidationResponse {
		is_valid,
		address_type: if is_valid {
			if request.address.starts_with('N') {
				"standard".to_string()
			} else {
				"unknown".to_string()
			}
		} else {
			"invalid".to_string()
		},
		script_hash: if is_valid {
			// Generate proper script hash using Neo SDK
			match neo3::neo_types::script_hash::ScriptHash::from_address(&request.address) {
				Ok(hash) => Some(hash.to_string()),
				Err(_) => None,
			}
		} else {
			None
		},
	};

	Ok(ApiResponse::success(result))
}

/// Format amount with decimals and symbol
#[command]
pub async fn format_amount(
	request: FormatAmountRequest,
	_state: State<'_, AppState>,
) -> Result<ApiResponse<FormattedAmount>, String> {
	log::info!("Formatting amount: {} with {} decimals", request.amount, request.decimals);

	let amount_value = match request.amount.parse::<f64>() {
		Ok(val) => val,
		Err(_) => return Ok(ApiResponse::error("Invalid amount format".to_string())),
	};

	let divisor = 10_f64.powi(request.decimals as i32);
	let formatted_value = amount_value / divisor;
	let symbol = request.symbol.unwrap_or_else(|| "TOKEN".to_string());

	let formatted = if request.decimals == 0 {
		format!("{} {}", formatted_value as u64, symbol)
	} else {
		format!("{:.precision$} {}", formatted_value, symbol, precision = request.decimals as usize)
	};

	let result =
		FormattedAmount { formatted, raw: request.amount, decimals: request.decimals, symbol };

	log::info!("Amount formatted successfully");
	Ok(ApiResponse::success(result))
}

/// Convert address to script hash
#[command]
pub async fn address_to_script_hash(
	request: AddressToScriptHashRequest,
	_state: State<'_, AppState>,
) -> Result<ApiResponse<String>, String> {
	log::info!("Converting address to script hash: {}", request.address);

	// Professional address to script hash conversion using Neo SDK
	match neo3::neo_types::script_hash::ScriptHash::from_address(&request.address) {
		Ok(script_hash) => {
			let script_hash_string = script_hash.to_string();
			Ok(ApiResponse::success(script_hash_string))
		},
		Err(e) => Ok(ApiResponse::error(format!("Invalid address: {}", e))),
	}
}

/// Convert script hash to address
#[command]
pub async fn script_hash_to_address(
	request: ScriptHashToAddressRequest,
	_state: State<'_, AppState>,
) -> Result<ApiResponse<String>, String> {
	log::info!("Converting script hash to address: {}", request.script_hash);

	// Professional script hash to address conversion using Neo SDK
	match neo3::neo_types::script_hash::ScriptHash::from_str(&request.script_hash) {
		Ok(script_hash) => {
			let address = script_hash.to_address();
			Ok(ApiResponse::success(address))
		},
		Err(e) => Ok(ApiResponse::error(format!("Invalid script hash: {}", e))),
	}
}

/// Generate a new private key
#[command]
pub async fn generate_private_key(
	_state: State<'_, AppState>,
) -> Result<ApiResponse<PrivateKeyGenerationResponse>, String> {
	log::info!("Generating new private key");

	// Professional private key generation using Neo SDK cryptography
	match neo3::neo_protocol::Account::create() {
		Ok(account) => {
			let response = if let Some(key_pair) = account.key_pair() {
				PrivateKeyGenerationResponse {
					private_key: key_pair.export_as_wif(),
					public_key: hex::encode(key_pair.public_key.get_encoded(true)),
					address: account.address_or_scripthash().address(),
				}
			} else {
				return Ok(ApiResponse::error(
					"Failed to generate private key: No key pair available".to_string(),
				));
			};
			Ok(ApiResponse::success(response))
		},
		Err(e) => Ok(ApiResponse::error(format!("Failed to generate private key: {}", e))),
	}
}

/// Derive public key from private key
#[command]
pub async fn derive_public_key(
	request: DerivePublicKeyRequest,
	_state: State<'_, AppState>,
) -> Result<ApiResponse<PublicKeyResponse>, String> {
	log::info!("Deriving public key from private key");

	// Professional key derivation using Neo SDK cryptography
	match neo3::neo_protocol::Account::from_wif(&request.private_key) {
		Ok(account) => {
			let response = if let Some(key_pair) = account.key_pair() {
				PublicKeyResponse {
					public_key: hex::encode(key_pair.public_key.get_encoded(true)),
					address: account.address_or_scripthash().address(),
				}
			} else {
				return Ok(ApiResponse::error(
					"Invalid private key: No key pair available".to_string(),
				));
			};
			Ok(ApiResponse::success(response))
		},
		Err(e) => Ok(ApiResponse::error(format!("Invalid private key: {}", e))),
	}
}
