#![allow(dead_code)]
use crate::{
	errors::CliError,
	utils::{extensions::TransactionExtensions, print_error, print_info, print_success},
};
use clap::{Args, Subcommand};
use hex;
use neo3::{neo_clients::APITrait, neo_protocol::NeoBlock};
use primitive_types::{H160, H256};
use std::{io, io::Write, path::PathBuf, str::FromStr};

#[derive(Args, Debug)]
pub struct BlockchainArgs {
	#[command(subcommand)]
	pub command: BlockchainCommands,
}

#[derive(Subcommand, Debug)]
pub enum BlockchainCommands {
	/// Show chain tip, node version, mempool, and connectivity summary
	Status,

	/// Export blockchain data
	Export {
		/// Path to save the exported data
		#[arg(short, long)]
		path: PathBuf,

		/// Start block index (inclusive)
		#[arg(short, long, default_value = "0")]
		start: u32,

		/// End block index (inclusive, if not specified, exports to the latest block)
		#[arg(short, long)]
		end: Option<u32>,
	},

	/// Show block details
	ShowBlock {
		/// Block hash or index
		#[arg(short, long)]
		identifier: String,
	},

	/// Show block header details
	ShowHeader {
		/// Block hash or index
		#[arg(short, long)]
		identifier: String,
	},

	/// Fetch raw block data
	RawBlock {
		/// Block hash or index
		#[arg(short, long)]
		identifier: String,
	},

	/// Show transaction information
	ShowTx {
		/// Transaction hash
		#[arg(short = 'x', long)]
		hash: String,
	},

	/// Show transaction height
	TxHeight {
		/// Transaction hash
		#[arg(short = 'x', long)]
		hash: String,
	},

	/// Show application log for a transaction
	AppLog {
		/// Transaction hash
		#[arg(short = 'x', long)]
		hash: String,
	},

	/// Show current mempool
	Mempool,

	/// Calculate network fee for a raw transaction
	CalculateFee {
		/// Raw transaction hex
		#[arg(short = 'x', long)]
		hex: String,
	},

	/// Send a raw transaction
	SendRaw {
		/// Raw transaction hex
		#[arg(short = 'x', long)]
		hex: String,
	},

	/// Read contract storage
	GetStorage {
		/// Contract hash
		#[arg(short, long)]
		contract: String,

		/// Storage key as hex string
		#[arg(short, long)]
		key: String,
	},

	/// Find contract storage by prefix
	FindStorage {
		/// Contract hash
		#[arg(short, long)]
		contract: String,

		/// Storage key prefix as hex string
		#[arg(short, long)]
		prefix: String,

		/// Start index for paged find results
		#[arg(long, default_value = "0")]
		start: u64,
	},

	/// Show contract details
	ShowContract {
		/// Contract hash or script hash
		#[arg(short = 'c', long)]
		hash: String,
	},
}

/// CLI state is defined in wallet.rs
#[allow(dead_code)]
pub async fn handle_blockchain_command(
	args: BlockchainArgs,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	match args.command {
		BlockchainCommands::Status => show_status(state).await,
		BlockchainCommands::Export { path, start, end } => {
			export_blockchain(path, start, end, state).await
		},
		BlockchainCommands::ShowBlock { identifier } => show_block(identifier, state).await,
		BlockchainCommands::ShowHeader { identifier } => show_header(identifier, state).await,
		BlockchainCommands::RawBlock { identifier } => show_raw_block(identifier, state).await,
		BlockchainCommands::ShowTx { hash } => show_transaction(hash, state).await,
		BlockchainCommands::TxHeight { hash } => show_transaction_height(hash, state).await,
		BlockchainCommands::AppLog { hash } => show_application_log(hash, state).await,
		BlockchainCommands::Mempool => show_mempool(state).await,
		BlockchainCommands::CalculateFee { hex } => calculate_network_fee(hex, state).await,
		BlockchainCommands::SendRaw { hex } => send_raw_transaction(hex, state).await,
		BlockchainCommands::GetStorage { contract, key } => get_storage(contract, key, state).await,
		BlockchainCommands::FindStorage { contract, prefix, start } => {
			find_storage(contract, prefix, start, state).await
		},
		BlockchainCommands::ShowContract { hash } => show_contract(hash, state).await,
	}
}

fn strip_hex_prefix(input: &str) -> &str {
	input.strip_prefix("0x").unwrap_or(input)
}

fn parse_h256(input: &str, label: &str) -> Result<H256, CliError> {
	let bytes = hex::decode(strip_hex_prefix(input))
		.map_err(|e| CliError::Input(format!("Invalid {label} hex: {e}")))?;
	if bytes.len() != 32 {
		return Err(CliError::Input(format!("{label} must be 32 bytes")));
	}
	Ok(H256::from_slice(&bytes))
}

fn parse_h160(input: &str, label: &str) -> Result<H160, CliError> {
	let bytes = hex::decode(strip_hex_prefix(input))
		.map_err(|e| CliError::Input(format!("Invalid {label} hex: {e}")))?;
	if bytes.len() != 20 {
		return Err(CliError::Input(format!("{label} must be 20 bytes")));
	}
	Ok(H160::from_slice(&bytes))
}

fn identifier_is_hash(identifier: &str) -> bool {
	let hex = strip_hex_prefix(identifier);
	hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit())
}

async fn show_status(state: &mut crate::commands::wallet::CliState) -> Result<(), CliError> {
	let rpc_client = state.get_rpc_client()?;

	let block_count = rpc_client
		.get_block_count()
		.await
		.map_err(|e| CliError::Network(format!("Failed to get block count: {}", e)))?;
	let header_count = rpc_client
		.get_block_header_count()
		.await
		.map_err(|e| CliError::Network(format!("Failed to get header count: {}", e)))?;
	let best_hash = rpc_client
		.get_best_block_hash()
		.await
		.map_err(|e| CliError::Network(format!("Failed to get best block hash: {}", e)))?;
	let version = rpc_client
		.get_version()
		.await
		.map_err(|e| CliError::Network(format!("Failed to get node version: {}", e)))?;
	let mempool_count = rpc_client.get_raw_mem_pool().await.map(|m| m.len()).unwrap_or(0);
	let connections = rpc_client.get_connection_count().await.ok();

	println!("Network: {}", state.get_network_type_string());
	println!("Node: {}", version.user_agent);
	println!("Best Block Hash: {}", best_hash);
	println!("Block Count: {}", block_count);
	println!("Header Count: {}", header_count);
	println!("Mempool Transactions: {}", mempool_count);
	println!(
		"Connections: {}",
		connections.map_or_else(|| "Unavailable".to_string(), |count| count.to_string())
	);
	Ok(())
}

#[allow(dead_code)]
async fn export_blockchain(
	path: PathBuf,
	start: u32,
	end: Option<u32>,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	if state.rpc_client.is_none() {
		print_error("No RPC client is connected. Please connect to a node first.");
		return Err(CliError::Network("No RPC client is connected".to_string()));
	}

	print_info(&format!(
		"Exporting blockchain data from block {} to {}...",
		start,
		end.map_or("latest".to_string(), |e| e.to_string())
	));

	let rpc_client = state.rpc_client.as_ref().unwrap();
	let latest_block = rpc_client
		.get_block_count()
		.await
		.map_err(|e| CliError::Network(format!("Failed to get block count: {}", e)))?;
	let end_block = end.unwrap_or(if latest_block > 0 { latest_block - 1 } else { 0 });

	if start > end_block {
		print_error("Start block index is greater than end block index");
		return Err(CliError::Input("Invalid block range".to_string()));
	}

	// Create export directory if it doesn't exist
	std::fs::create_dir_all(&path).map_err(|e| CliError::Io(e))?;

	// Export blocks
	let mut exported = 0;
	let total_blocks = end_block - start + 1;

	for i in start..=end_block {
		print!("\rExporting block {} of {}...", i, end_block);
		io::stdout().flush().map_err(|e| CliError::Io(e))?;

		let block = match rpc_client.get_block_by_index(i, true).await {
			Ok(block) => block,
			Err(e) => {
				print_error(&format!("Failed to retrieve block {}: {}", i, e));
				continue;
			},
		};

		// Export block to JSON file
		let block_path = path.join(format!("block_{}.json", i));
		let json = serde_json::to_string_pretty(&block)
			.map_err(|e| CliError::Input(format!("Failed to serialize block: {}", e)))?;

		std::fs::write(&block_path, json).map_err(|e| CliError::Io(e))?;

		exported += 1;

		// Show progress
		if exported % 100 == 0 || exported == total_blocks {
			print_info(&format!(
				"Exported {}/{} blocks ({}%)",
				exported,
				total_blocks,
				(exported * 100) / total_blocks
			));
		}
	}

	print_success(&format!("Blockchain data exported to: {:?} ({} blocks)", path, exported));
	Ok(())
}

#[allow(dead_code)]
async fn show_block(
	identifier: String,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	if state.rpc_client.is_none() {
		print_error("No RPC client is connected. Please connect to a node first.");
		return Err(CliError::Network("No RPC client is connected".to_string()));
	}

	print_info(&format!("Fetching block: {}", identifier));

	let rpc_client = state.rpc_client.as_ref().unwrap();

	if identifier.starts_with("0x")
		|| (identifier.len() == 64 && identifier.chars().all(|c| c.is_ascii_hexdigit()))
	{
		// Identifier is a hash
		match rpc_client.get_block_by_hash(&identifier, true).await {
			Ok(block) => show_block_by_hash(block),
			Err(e) => {
				print_error(&format!("Failed to get block by hash: {}", e));
				Err(CliError::Network(format!("Failed to get block by hash: {}", e)))
			},
		}
	} else {
		// Try to parse as a block index (integer)
		let index = identifier.parse::<u32>().map_err(|_| {
			CliError::Input(format!(
				"Invalid block identifier. Must be a block hash or block index: {}",
				identifier
			))
		})?;

		show_block_by_index(index, state).await
	}
}

async fn show_header(
	identifier: String,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	let rpc_client = state.get_rpc_client()?;
	let header = if identifier_is_hash(&identifier) {
		let hash = parse_h256(&identifier, "block hash")?;
		rpc_client
			.get_block_header(hash)
			.await
			.map_err(|e| CliError::Rpc(format!("Failed to get block header: {}", e)))?
	} else {
		let index = identifier.parse::<u32>().map_err(|_| {
			CliError::Input(format!(
				"Invalid block identifier. Must be a block hash or block index: {}",
				identifier
			))
		})?;
		rpc_client
			.get_block_header_by_index(index)
			.await
			.map_err(|e| CliError::Rpc(format!("Failed to get block header: {}", e)))?
	};

	println!("{}", serde_json::to_string_pretty(&header)?);
	Ok(())
}

async fn show_raw_block(
	identifier: String,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	let rpc_client = state.get_rpc_client()?;
	let raw = if identifier_is_hash(&identifier) {
		let hash = parse_h256(&identifier, "block hash")?;
		rpc_client
			.get_raw_block(hash)
			.await
			.map_err(|e| CliError::Rpc(format!("Failed to get raw block: {}", e)))?
	} else {
		let index = identifier.parse::<u32>().map_err(|_| {
			CliError::Input(format!(
				"Invalid block identifier. Must be a block hash or block index: {}",
				identifier
			))
		})?;
		rpc_client
			.get_raw_block_by_index(index)
			.await
			.map_err(|e| CliError::Rpc(format!("Failed to get raw block: {}", e)))?
	};

	println!("{raw}");
	Ok(())
}

#[allow(dead_code)]
async fn show_block_by_index(
	index: u32,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	if state.rpc_client.is_none() {
		print_error("No RPC client is connected. Please connect to a node first.");
		return Err(CliError::Network("No RPC client is connected".to_string()));
	}

	print_info(&format!("Fetching block at index: {}", index));

	let rpc_client = state.rpc_client.as_ref().unwrap();

	// Get block by index
	let block = match rpc_client.get_block_by_index(index, true).await {
		Ok(block) => block,
		Err(e) => return Err(CliError::Rpc(format!("Failed to get block by index: {}", e))),
	};

	// Display block information
	println!("Block Hash: {}", block.hash);
	println!("Block Index: {}", block.index);
	println!("Block Time: {}", block.time);
	println!("Block Size: {}", block.size);
	println!("Transaction Count: {}", block.transactions.as_ref().map_or(0, |tx| tx.len()));
	println!("Merkle Root: {}", block.merkle_root_hash);
	println!("Previous Block: {}", block.prev_block_hash);
	println!("Next Consensus: {}", block.next_consensus);

	// Show transactions if there are any
	if let Some(transactions) = &block.transactions
		&& !transactions.is_empty()
	{
		println!("\nTransactions:");
		for (i, tx) in transactions.iter().enumerate() {
			println!("  {}. Hash: {}", i + 1, tx.hash);
		}
	}

	print_success("Block information retrieved successfully");
	Ok(())
}

#[allow(dead_code)]
fn show_block_by_hash(block: NeoBlock) -> Result<(), CliError> {
	// Display block information
	println!("Block Hash: {}", block.hash);
	println!("Block Index: {}", block.index);
	println!("Block Time: {}", block.time);
	println!("Block Size: {}", block.size);
	println!("Transaction Count: {}", block.transactions.as_ref().map_or(0, |tx| tx.len()));
	println!("Merkle Root: {}", block.merkle_root_hash);
	println!("Previous Block: {}", block.prev_block_hash);
	println!("Next Consensus: {}", block.next_consensus);

	// Show transactions if there are any
	if let Some(transactions) = &block.transactions
		&& !transactions.is_empty()
	{
		println!("\nTransactions:");
		for (i, tx) in transactions.iter().enumerate() {
			println!("  {}. Hash: {}", i + 1, tx.hash);
		}
	}

	print_success("Block information retrieved successfully");
	Ok(())
}

#[allow(dead_code)]
async fn show_transaction(
	hash: String,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	if state.rpc_client.is_none() {
		print_error("No RPC client is connected. Please connect to a node first.");
		return Err(CliError::Network("No RPC client is connected".to_string()));
	}

	print_info(&format!("Fetching transaction information for: {}", hash));

	let rpc_client = state.rpc_client.as_ref().unwrap();

	// Remove '0x' prefix if present
	let hash_str = hash.strip_prefix("0x").unwrap_or(&hash);

	// Convert to H256
	let hash_bytes = hex::decode(hash_str)
		.map_err(|e| CliError::Input(format!("Invalid transaction hash format: {}", e)))?;

	if hash_bytes.len() != 32 {
		return Err(CliError::Input("Transaction hash must be 32 bytes".to_string()));
	}

	let hash = primitive_types::H256::from_slice(&hash_bytes);

	let tx = rpc_client
		.get_transaction(hash)
		.await
		.map_err(|e| CliError::Rpc(format!("Failed to retrieve transaction: {}", e)))?;

	// Display transaction information
	println!("Transaction Hash: {}", tx.hash);
	println!("Transaction Type: {}", tx.type_name());
	println!("Transaction Size: {}", tx.size);
	println!("Transaction Version: {}", tx.version);
	println!("Transaction Nonce: {}", tx.nonce);
	println!("Transaction Sender: {}", tx.sender);
	println!("Transaction System Fee: {}", tx.sys_fee);
	println!("Transaction Network Fee: {}", tx.net_fee);
	println!("Transaction Valid Until Block: {}", tx.valid_until_block);

	// Display signers
	println!("\nTransaction Signers ({}):", tx.signers.len());
	for (i, signer) in tx.signers.iter().enumerate() {
		println!("  {}. Account: {}", i + 1, signer.account);
		println!("     Scopes: {:?}", signer.scopes);
		if !signer.allowed_contracts.is_empty() {
			println!("     Allowed Contracts: {:?}", signer.allowed_contracts);
		}
		if !signer.allowed_groups.is_empty() {
			println!("     Allowed Groups: {:?}", signer.allowed_groups);
		}
	}

	// Show witnesses if any
	if !tx.witnesses.is_empty() {
		println!("\nWitnesses ({}):", tx.witnesses.len());
		for (i, witness) in tx.witnesses.iter().enumerate() {
			println!("  {}. Invocation Script: 0x{}", i + 1, hex::encode(&witness.invocation));
			println!("     Verification Script: 0x{}", hex::encode(&witness.verification));
		}
	}

	// Display script
	println!("\nTransaction Script: 0x{}", hex::encode(&tx.script));

	print_success("Transaction information retrieved successfully");
	Ok(())
}

async fn show_transaction_height(
	hash: String,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	let rpc_client = state.get_rpc_client()?;
	let tx_hash = parse_h256(&hash, "transaction hash")?;
	let height = rpc_client
		.get_transaction_height(tx_hash)
		.await
		.map_err(|e| CliError::Rpc(format!("Failed to get transaction height: {}", e)))?;
	println!("{height}");
	Ok(())
}

async fn show_application_log(
	hash: String,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	let rpc_client = state.get_rpc_client()?;
	let tx_hash = parse_h256(&hash, "transaction hash")?;
	let log = rpc_client
		.get_application_log(tx_hash)
		.await
		.map_err(|e| CliError::Rpc(format!("Failed to get application log: {}", e)))?;
	println!("{}", serde_json::to_string_pretty(&log)?);
	Ok(())
}

async fn show_mempool(state: &mut crate::commands::wallet::CliState) -> Result<(), CliError> {
	let rpc_client = state.get_rpc_client()?;
	let mempool = rpc_client
		.get_raw_mempool()
		.await
		.map_err(|e| CliError::Rpc(format!("Failed to get mempool: {}", e)))?;
	println!("{}", serde_json::to_string_pretty(&mempool)?);
	Ok(())
}

async fn calculate_network_fee(
	hex: String,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	let rpc_client = state.get_rpc_client()?;
	let fee = rpc_client
		.calculate_network_fee(strip_hex_prefix(&hex).to_string())
		.await
		.map_err(|e| CliError::Rpc(format!("Failed to calculate network fee: {}", e)))?;
	println!("{}", serde_json::to_string_pretty(&fee)?);
	Ok(())
}

async fn send_raw_transaction(
	hex: String,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	let rpc_client = state.get_rpc_client()?;
	let result = rpc_client
		.send_raw_transaction(strip_hex_prefix(&hex).to_string())
		.await
		.map_err(|e| CliError::Rpc(format!("Failed to send raw transaction: {}", e)))?;
	println!("{}", serde_json::to_string_pretty(&result)?);
	Ok(())
}

async fn get_storage(
	contract: String,
	key: String,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	let rpc_client = state.get_rpc_client()?;
	let contract_hash = parse_h160(&contract, "contract hash")?;
	let value = rpc_client
		.get_storage(contract_hash, strip_hex_prefix(&key))
		.await
		.map_err(|e| CliError::Rpc(format!("Failed to get storage: {}", e)))?;
	println!("{value}");
	Ok(())
}

async fn find_storage(
	contract: String,
	prefix: String,
	start: u64,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	let rpc_client = state.get_rpc_client()?;
	let contract_hash = parse_h160(&contract, "contract hash")?;
	let result = rpc_client
		.find_storage(contract_hash, strip_hex_prefix(&prefix), start)
		.await
		.map_err(|e| CliError::Rpc(format!("Failed to find storage: {}", e)))?;
	println!("{result}");
	Ok(())
}

#[allow(dead_code)]
async fn show_contract(
	hash: String,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	if state.rpc_client.is_none() {
		print_error("No RPC client is connected. Please connect to a node first.");
		return Err(CliError::Network("No RPC client is connected".to_string()));
	}

	print_info(&format!("Fetching contract information for: {}", hash));

	let rpc_client = state.rpc_client.as_ref().unwrap();

	// Convert from string to H160
	let contract_hash = H160::from_str(&hash)
		.map_err(|_| CliError::Input(format!("Invalid contract hash format: {}", hash)))?;

	let contract = rpc_client
		.get_contract_state(contract_hash)
		.await
		.map_err(|e| CliError::Rpc(format!("Failed to retrieve contract: {}", e)))?;

	// Display contract information
	println!("Contract Hash: {}", contract.hash);
	println!("Contract ID: {}", contract.id);
	println!("Update Counter: {}", contract.update_counter);

	println!("\nManifest:");
	println!("  Name: {:?}", contract.manifest.name);
	println!("  Groups: {:?}", contract.manifest.groups);
	println!("  Features: {:?}", contract.manifest.features);
	println!("  Supported Standards: {:?}", contract.manifest.supported_standards);
	println!("  Trusts: {:?}", contract.manifest.trusts);

	if let Some(abi) = &contract.manifest.abi {
		println!("\n  ABI Methods ({}):", abi.methods.len());
		for (i, method) in abi.methods.iter().enumerate() {
			println!("    {}. {} ({} parameters)", i + 1, method.name, method.parameters.len());
		}

		println!("\n  ABI Events ({}):", abi.events.len());
		for (i, event) in abi.events.iter().enumerate() {
			println!("    {}. {} ({} parameters)", i + 1, event.name, event.parameters.len());
		}
	} else {
		println!("\n  No ABI available");
	}

	println!("\nPermissions ({}):", contract.manifest.permissions.len());
	for (i, perm) in contract.manifest.permissions.iter().enumerate() {
		println!("    {}. {:?}", i + 1, perm);
	}

	print_success("Contract information retrieved successfully");
	Ok(())
}
