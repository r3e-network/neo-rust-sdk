use crate::{
	commands::wallet::CliState,
	errors::CliError,
	utils_core::{
		create_table, print_info, print_section_header, print_success, prompt_password,
		prompt_yes_no,
	},
};
use clap::{Args, Subcommand};
use comfy_table::{Cell, Color};
use neo3::{
	NeoVMStateType,
	builder::{AccountSigner, ScriptBuilder, Signer, TransactionBuilder},
	codec::NeoSerializable,
	neo_clients::{APITrait, HttpProvider},
	neo_protocol::{Account, AccountTrait},
	neo_types::{AddressExtension, ContractParameter, ScriptHashExtension},
};
use primitive_types::H160;

#[derive(Args, Debug)]
pub struct NftArgs {
	#[command(subcommand)]
	pub command: NftCommands,
}

#[derive(Subcommand, Debug)]
pub enum NftCommands {
	/// Mint a new NFT
	#[command(about = "Mint a new NFT")]
	Mint {
		/// Contract hash of the NFT collection
		#[arg(short, long, help = "NFT contract hash")]
		contract: String,

		/// Recipient address
		#[arg(short, long, help = "Address to receive the NFT")]
		to: String,

		/// Token ID
		#[arg(short = 'i', long, help = "Unique token ID")]
		token_id: String,

		/// Metadata URI
		#[arg(short, long, help = "URI pointing to token metadata")]
		metadata: Option<String>,

		/// Properties (JSON format)
		#[arg(short, long, help = "Token properties in JSON format")]
		properties: Option<String>,

		/// Signing account (uses wallet default if omitted)
		#[arg(short, long, help = "Signing account address")]
		account: Option<String>,
	},

	/// Transfer a non-divisible NEP-11 token
	#[command(about = "Transfer a non-divisible NEP-11 token")]
	Transfer {
		/// Token ID to transfer
		#[arg(short = 'i', long, help = "Token ID to transfer")]
		token_id: String,

		/// Contract hash of the NFT collection
		#[arg(short, long, help = "NFT contract hash")]
		contract: String,

		/// Sender address
		#[arg(short, long, help = "Current owner address")]
		from: String,

		/// Recipient address
		#[arg(short, long, help = "New owner address")]
		to: String,

		/// Transfer data (optional)
		#[arg(short, long, help = "Additional transfer data")]
		data: Option<String>,
	},

	/// List NFTs owned by an address
	#[command(about = "List NFTs owned by an address")]
	List {
		/// Owner address
		#[arg(short, long, help = "Address to check for NFTs")]
		owner: String,

		/// Contract hash (optional, lists from all contracts if not specified)
		#[arg(short, long, help = "Specific contract to check")]
		contract: Option<String>,

		/// Show detailed information
		#[arg(short, long, help = "Show detailed NFT information")]
		detailed: bool,
	},

	/// Get NFT information
	#[command(about = "Get detailed information about an NFT")]
	Info {
		/// Contract hash of the NFT collection
		#[arg(short, long, help = "NFT contract hash")]
		contract: String,

		/// Token ID
		#[arg(short = 'i', long, help = "Token ID to query")]
		token_id: String,
	},

	/// Get NFT metadata
	#[command(about = "Get NFT metadata")]
	Metadata {
		/// Contract hash of the NFT collection
		#[arg(short, long, help = "NFT contract hash")]
		contract: String,

		/// Token ID
		#[arg(short = 'i', long, help = "Token ID to query")]
		token_id: String,

		/// Download metadata to file
		#[arg(short, long, help = "Download metadata to file")]
		download: bool,
	},

	/// Burn an NFT
	#[command(about = "Burn (destroy) an NFT")]
	Burn {
		/// Contract hash of the NFT collection
		#[arg(short, long, help = "NFT contract hash")]
		contract: String,

		/// Token ID to burn
		#[arg(short = 'i', long, help = "Token ID to burn")]
		token_id: String,

		/// Owner address
		#[arg(short, long, help = "Current owner address")]
		owner: String,
	},

	/// Set NFT properties
	#[command(about = "Set properties for an NFT")]
	SetProperties {
		/// Contract hash of the NFT collection
		#[arg(short, long, help = "NFT contract hash")]
		contract: String,

		/// Token ID
		#[arg(short = 'i', long, help = "Token ID to update")]
		token_id: String,

		/// Properties (JSON format)
		#[arg(short, long, help = "Properties in JSON format")]
		properties: String,

		/// Signing account (uses wallet default if omitted)
		#[arg(short, long, help = "Signing account address")]
		account: Option<String>,
	},

	/// Get collection information
	#[command(about = "Get information about an NFT collection")]
	Collection {
		/// Contract hash of the NFT collection
		#[arg(short, long, help = "NFT contract hash")]
		contract: String,
	},
}

/// Handle NFT command with comprehensive functionality
pub async fn handle_nft_command(args: NftArgs, state: &mut CliState) -> Result<(), CliError> {
	match args.command {
		NftCommands::Mint { contract, to, token_id, metadata, properties, account } => {
			handle_mint_nft(contract, to, token_id, metadata, properties, account, state).await
		},
		NftCommands::Transfer { contract, token_id, from, to, data } => {
			handle_transfer_nft(contract, token_id, from, to, data, state).await
		},
		NftCommands::List { owner, contract, detailed } => {
			handle_list_nfts(owner, contract, detailed, state).await
		},
		NftCommands::Info { contract, token_id } => {
			handle_nft_info(contract, token_id, state).await
		},
		NftCommands::Metadata { contract, token_id, download } => {
			handle_nft_metadata(contract, token_id, download, state).await
		},
		NftCommands::Burn { contract, token_id, owner } => {
			handle_burn_nft(contract, token_id, owner, state).await
		},
		NftCommands::SetProperties { contract, token_id, properties, account } => {
			handle_set_properties(contract, token_id, properties, account, state).await
		},
		NftCommands::Collection { contract } => handle_collection_info(contract, state).await,
	}
}

fn strip_hex_prefix(input: &str) -> &str {
	input.strip_prefix("0x").unwrap_or(input)
}

fn parse_h160(input: &str, label: &str) -> Result<H160, CliError> {
	let bytes = hex::decode(strip_hex_prefix(input))
		.map_err(|e| CliError::InvalidInput(format!("Invalid {label} hex: {e}")))?;
	if bytes.len() != 20 {
		return Err(CliError::InvalidInput(format!("{label} must be 20 bytes")));
	}
	Ok(H160::from_slice(&bytes))
}

fn token_id_bytes(token_id: &str) -> Result<Vec<u8>, CliError> {
	if let Some(hex) = token_id.strip_prefix("0x") {
		return hex::decode(hex)
			.map_err(|e| CliError::InvalidInput(format!("Invalid token-id hex: {e}")));
	}
	Ok(token_id.as_bytes().to_vec())
}

fn optional_data_parameter(data: Option<String>) -> Result<ContractParameter, CliError> {
	let Some(data) = data else {
		return Ok(ContractParameter::any());
	};

	let trimmed = data.trim();
	if trimmed.is_empty() {
		return Ok(ContractParameter::any());
	}

	match serde_json::from_str::<serde_json::Value>(trimmed) {
		Ok(value) => Ok(ContractParameter::from(value)),
		Err(_) => Ok(ContractParameter::string(data)),
	}
}

fn json_contract_parameter(input: &str, label: &str) -> Result<ContractParameter, CliError> {
	let value: serde_json::Value = serde_json::from_str(input)
		.map_err(|e| CliError::InvalidInput(format!("Invalid {label} JSON: {e}")))?;
	Ok(ContractParameter::from(value))
}

fn select_account(state: &CliState, address: Option<&str>) -> Result<Account, CliError> {
	if let Some(address) = address {
		let script_hash = address.address_to_script_hash().map_err(|e| {
			CliError::InvalidInput(format!("Invalid account address '{address}': {e}"))
		})?;
		let wallet = state.wallet.as_ref().ok_or_else(|| {
			CliError::WalletNotLoaded("No wallet open. Use 'wallet open' first.".into())
		})?;
		return wallet
			.accounts
			.get(&script_hash)
			.cloned()
			.ok_or_else(|| CliError::Wallet(format!("Account not found in wallet: {address}")));
	}

	state.get_account()
}

async fn send_nft_transaction(
	contract_hash: H160,
	method: &str,
	params: Vec<ContractParameter>,
	signer_address: Option<&str>,
	state: &mut CliState,
) -> Result<(), CliError> {
	let account = select_account(state, signer_address)?;
	if account.is_multi_sig() {
		return Err(CliError::InvalidInput(
			"Multi-signature NFT transactions require offline/manual signing; build the call with 'contract invoke' and complete signing separately."
				.to_string(),
		));
	}

	let password = match state.wallet_password.clone() {
		Some(password) => password,
		None => {
			let password = prompt_password("Enter wallet password")?;
			state.wallet_password = Some(password.clone());
			password
		},
	};

	let rpc_client = state.get_rpc_client()?;
	let signer = AccountSigner::called_by_entry(&account)
		.map_err(|e| CliError::Builder(format!("Failed to create signer: {e}")))?;
	let signers = vec![Signer::AccountSigner(signer)];

	let simulation = rpc_client
		.invoke_function_diagnostics(
			contract_hash,
			method.to_string(),
			params.clone(),
			signers.clone(),
		)
		.await
		.map_err(|e| CliError::Network(format!("Failed to test NFT transaction: {e}")))?;

	print_info(&format!(
		"Invocation test state: {:?}, gas consumed: {}",
		simulation.state, simulation.gas_consumed
	));
	if simulation.state != NeoVMStateType::Halt {
		return Err(CliError::TransactionFailed(format!(
			"NFT transaction test failed: {}",
			simulation.exception.unwrap_or_else(|| "VM fault".to_string())
		)));
	}

	if !prompt_yes_no("Submit this NFT transaction?")? {
		return Err(CliError::UserCancelled("NFT transaction cancelled by user".to_string()));
	}

	let block_count = rpc_client
		.get_block_count()
		.await
		.map_err(|e| CliError::Network(format!("Failed to get block count: {e}")))?;

	let mut tx_builder: TransactionBuilder<'_, HttpProvider> =
		TransactionBuilder::with_client(rpc_client);
	tx_builder.version(0);
	tx_builder
		.nonce(rand::random::<u32>())
		.map_err(|e| CliError::Transaction(format!("Failed to set nonce: {e}")))?;
	tx_builder
		.valid_until_block(block_count + 100)
		.map_err(|e| CliError::Transaction(format!("Failed to set valid-until block: {e}")))?;
	tx_builder
		.set_signers(signers)
		.map_err(|e| CliError::Transaction(format!("Failed to set signers: {e}")))?;

	let script = ScriptBuilder::new()
		.contract_call(&contract_hash, method, &params, None)
		.map_err(|e| CliError::Builder(format!("Failed to build invocation script: {e}")))?
		.to_bytes();
	tx_builder.set_script(Some(script));

	let mut tx = tx_builder
		.build()
		.await
		.map_err(|e| CliError::Transaction(format!("Failed to build transaction: {e}")))?;

	let mut signing_account = account.clone();
	if signing_account.key_pair().is_none() {
		signing_account
			.decrypt_private_key(&password)
			.map_err(|e| CliError::WalletOperation(format!("Failed to decrypt account: {e}")))?;
	}
	let key_pair = signing_account
		.key_pair()
		.as_ref()
		.ok_or_else(|| CliError::Wallet("Account does not contain a private key".to_string()))?
		.clone();

	let tx_hash = tx
		.get_hash_data()
		.await
		.map_err(|e| CliError::Transaction(format!("Failed to get transaction hash data: {e}")))?;
	let witness = neo3::builder::Witness::create(tx_hash, &key_pair)
		.map_err(|e| CliError::Transaction(format!("Failed to create witness: {e}")))?;
	tx.add_witness(witness);

	let mut encoder = neo3::codec::Encoder::new();
	tx.encode(&mut encoder);
	let tx_hex = hex::encode(encoder.to_bytes());
	let result = rpc_client
		.send_raw_transaction(tx_hex)
		.await
		.map_err(|e| CliError::Network(format!("Failed to send NFT transaction: {e}")))?;

	print_success(&format!("NFT transaction submitted: {}", result.hash));
	Ok(())
}

/// Mint a new NFT
async fn handle_mint_nft(
	contract: String,
	to: String,
	token_id: String,
	metadata: Option<String>,
	properties: Option<String>,
	account: Option<String>,
	state: &mut CliState,
) -> Result<(), CliError> {
	print_section_header("Minting NFT");

	let contract_hash = parse_h160(&contract, "contract hash")?;
	let to_hash = to
		.address_to_script_hash()
		.map_err(|e| CliError::InvalidInput(format!("Invalid recipient address '{to}': {e}")))?;
	let mut params = vec![
		ContractParameter::h160(&to_hash),
		ContractParameter::byte_array(token_id_bytes(&token_id)?),
	];
	if let Some(metadata) = metadata {
		params.push(ContractParameter::string(metadata));
	}
	if let Some(properties) = properties {
		params.push(json_contract_parameter(&properties, "properties")?);
	}

	send_nft_transaction(contract_hash, "mint", params, account.as_deref(), state).await
}

/// Transfer an NFT
async fn handle_transfer_nft(
	contract: String,
	token_id: String,
	from: String,
	to: String,
	data: Option<String>,
	state: &mut CliState,
) -> Result<(), CliError> {
	print_section_header("Transferring NFT");

	let contract_hash = parse_h160(&contract, "contract hash")?;
	let to_hash = to
		.address_to_script_hash()
		.map_err(|e| CliError::InvalidInput(format!("Invalid recipient address '{to}': {e}")))?;
	let params = vec![
		ContractParameter::h160(&to_hash),
		ContractParameter::byte_array(token_id_bytes(&token_id)?),
		optional_data_parameter(data)?,
	];

	send_nft_transaction(contract_hash, "transfer", params, Some(&from), state).await
}

/// List NFTs owned by an address
async fn handle_list_nfts(
	owner: String,
	contract: Option<String>,
	detailed: bool,
	state: &mut CliState,
) -> Result<(), CliError> {
	print_section_header("NFT Collection");

	let rpc_client = state.get_rpc_client()?;
	let owner_hash = owner
		.address_to_script_hash()
		.map_err(|e| CliError::InvalidInput(format!("Invalid owner address '{owner}': {e}")))?;
	let filter = contract.as_deref().map(|hash| parse_h160(hash, "contract hash")).transpose()?;

	let balances = rpc_client
		.get_nep11_balances(owner_hash)
		.await
		.map_err(|e| CliError::Network(format!("Failed to get NEP-11 balances: {e}")))?;

	if detailed {
		println!("{}", serde_json::to_string_pretty(&balances)?);
		return Ok(());
	}

	let mut table = create_table();
	table.set_header(vec![
		Cell::new("Contract").fg(Color::Cyan),
		Cell::new("Symbol").fg(Color::Cyan),
		Cell::new("Token ID").fg(Color::Cyan),
		Cell::new("Amount").fg(Color::Cyan),
		Cell::new("Updated").fg(Color::Cyan),
	]);

	let mut rows = 0usize;
	for balance in balances.balances {
		if filter.as_ref().is_some_and(|hash| hash != &balance.asset_hash) {
			continue;
		}
		for token in balance.tokens {
			rows += 1;
			table.add_row(vec![
				Cell::new(balance.asset_hash.to_hex_big_endian()).fg(Color::Blue),
				Cell::new(balance.symbol.clone()).fg(Color::Green),
				Cell::new(token.token_id).fg(Color::White),
				Cell::new(token.amount).fg(Color::Yellow),
				Cell::new(token.last_updated_block.to_string()).fg(Color::Yellow),
			]);
		}
	}

	println!("{table}");
	print_success(&format!("Found {rows} NFT token(s)"));
	Ok(())
}

/// Get NFT information
async fn handle_nft_info(
	contract: String,
	token_id: String,
	state: &mut CliState,
) -> Result<(), CliError> {
	print_section_header("NFT Information");

	let rpc_client = state.get_rpc_client()?;
	let contract_hash = parse_h160(&contract, "contract hash")?;
	let properties = rpc_client
		.get_nep11_properties(contract_hash, &token_id)
		.await
		.map_err(|e| CliError::Network(format!("Failed to get NFT properties: {e}")))?;

	let payload = serde_json::json!({
		"contract": contract_hash.to_hex_big_endian(),
		"token_id": token_id,
		"properties": properties,
	});
	println!("{}", serde_json::to_string_pretty(&payload)?);
	Ok(())
}

async fn handle_nft_metadata(
	contract: String,
	token_id: String,
	download: bool,
	state: &mut CliState,
) -> Result<(), CliError> {
	print_section_header("NFT Metadata");

	let rpc_client = state.get_rpc_client()?;
	let contract_hash = parse_h160(&contract, "contract hash")?;
	let properties = rpc_client
		.get_nep11_properties(contract_hash, &token_id)
		.await
		.map_err(|e| CliError::Network(format!("Failed to get NFT metadata: {e}")))?;
	let json = serde_json::to_string_pretty(&properties)?;

	if download {
		let filename = format!("nft_{}_metadata.json", token_id.replace('/', "_"));
		std::fs::write(&filename, &json).map_err(CliError::Io)?;
		print_success(&format!("Metadata saved to {filename}"));
	} else {
		println!("{json}");
	}
	Ok(())
}

async fn handle_burn_nft(
	contract: String,
	token_id: String,
	owner: String,
	state: &mut CliState,
) -> Result<(), CliError> {
	print_section_header("Burning NFT");

	let contract_hash = parse_h160(&contract, "contract hash")?;
	let params = vec![ContractParameter::byte_array(token_id_bytes(&token_id)?)];
	send_nft_transaction(contract_hash, "burn", params, Some(&owner), state).await
}

async fn handle_set_properties(
	contract: String,
	token_id: String,
	properties: String,
	account: Option<String>,
	state: &mut CliState,
) -> Result<(), CliError> {
	print_section_header("Setting NFT Properties");

	let contract_hash = parse_h160(&contract, "contract hash")?;
	let params = vec![
		ContractParameter::byte_array(token_id_bytes(&token_id)?),
		json_contract_parameter(&properties, "properties")?,
	];
	send_nft_transaction(contract_hash, "setProperties", params, account.as_deref(), state).await
}

async fn handle_collection_info(contract: String, state: &mut CliState) -> Result<(), CliError> {
	print_section_header("NFT Collection Information");

	let rpc_client = state.get_rpc_client()?;
	let contract_hash = parse_h160(&contract, "contract hash")?;
	let contract_state = rpc_client
		.get_contract_state(contract_hash)
		.await
		.map_err(|e| CliError::Network(format!("Failed to get NFT contract: {e}")))?;

	println!("{}", serde_json::to_string_pretty(&contract_state)?);
	Ok(())
}
