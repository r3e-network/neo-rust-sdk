#![allow(dead_code)]
use crate::{
	errors::CliError,
	utils_core::{
		create_table, display_key_value, print_info, print_section_header, print_success,
		print_warning, prompt_input, prompt_password, prompt_yes_no, status_indicator,
		with_loading,
	},
};
use bip39::Language;
use clap::{Args, Subcommand};
use comfy_table::{Cell, Color};
use hex;
use neo3::{
	NeoVMStateType,
	neo_clients::{APITrait, HttpProvider, RpcClient},
	neo_crypto::Secp256r1PublicKey,
	neo_protocol::{Account, AccountTrait},
	neo_types::AddressExtension,
	neo_wallets::{Wallet, WalletBackup, WalletTrait},
};
use std::path::PathBuf;

pub struct CliState {
	pub wallet: Option<Wallet>,
	pub wallet_path: Option<PathBuf>,
	// Cached password for the current CLI session only.
	pub wallet_password: Option<String>,
	pub rpc_client: Option<RpcClient<HttpProvider>>,
	pub network_type: Option<String>,
	pub current_network: Option<crate::commands::network::NetworkConfig>,
	pub networks: Vec<crate::commands::network::NetworkConfig>,
}

impl Default for CliState {
	fn default() -> Self {
		Self {
			wallet: None,
			wallet_path: None,
			wallet_password: None,
			rpc_client: None,
			network_type: None,
			current_network: None,
			networks: Vec::new(),
		}
	}
}

impl CliState {
	pub fn get_network_type_string(&self) -> String {
		self.network_type.clone().unwrap_or_else(|| "testnet".to_string())
	}

	pub fn set_network_type(&mut self, network: String) {
		self.network_type = Some(network);
	}

	pub fn get_rpc_client(&self) -> Result<&RpcClient<HttpProvider>, CliError> {
		self.rpc_client.as_ref().ok_or_else(|| {
			CliError::Config("No RPC client configured. Use 'network connect' first.".to_string())
		})
	}

	pub fn get_account(&self) -> Result<Account, CliError> {
		let wallet = self
			.wallet
			.as_ref()
			.ok_or_else(|| CliError::Wallet("No wallet open. Open a wallet first.".to_string()))?;

		if let Some(default_account) = wallet.default_account() {
			return Ok(default_account.clone());
		}

		wallet
			.accounts()
			.into_iter()
			.next()
			.ok_or_else(|| CliError::Wallet("Wallet has no accounts.".to_string()))
	}
}

#[derive(Args, Debug)]
pub struct WalletArgs {
	#[command(subcommand)]
	pub command: WalletCommands,
}

#[derive(Subcommand, Debug)]
pub enum WalletCommands {
	/// Create a new wallet
	#[command(about = "Create a new Neo wallet")]
	Create {
		/// Path to save the wallet
		#[arg(short, long, help = "Path to save the wallet file")]
		path: Option<PathBuf>,

		/// Wallet name
		#[arg(short, long, help = "Name for the wallet")]
		name: Option<String>,

		/// Password for the wallet (if not provided, will prompt)
		#[arg(long, help = "Password for the wallet")]
		password: Option<String>,
	},

	/// Open an existing wallet
	#[command(about = "Open an existing wallet")]
	Open {
		/// Path to the wallet file
		#[arg(short, long, help = "Path to the wallet file")]
		path: PathBuf,

		/// Password for the wallet (if not provided, will prompt)
		#[arg(long, help = "Password for the wallet")]
		password: Option<String>,
	},

	/// Close the current wallet
	#[command(about = "Close the currently open wallet")]
	Close,

	/// List addresses in the wallet
	#[command(about = "List all addresses in the wallet")]
	List,

	/// Show wallet information
	#[command(about = "Show detailed wallet information")]
	Info,

	/// Create a new address in the wallet
	#[command(about = "Create new addresses in the wallet")]
	CreateAddress {
		/// Number of addresses to create
		#[arg(short, long, default_value = "1", help = "Number of addresses to create")]
		count: u16,

		/// Label for the address
		#[arg(short, long, help = "Label for the new address")]
		label: Option<String>,
	},

	/// Import a private key
	#[command(about = "Import a private key into the wallet")]
	Import {
		/// WIF string or path to a file containing WIF keys
		#[arg(short, long, help = "WIF private key or file path")]
		wif_or_file: String,

		/// Label for the imported account
		#[arg(short, long, help = "Label for the imported account")]
		label: Option<String>,
	},

	/// Import a watch-only address
	#[command(about = "Import a watch-only address into the wallet")]
	ImportAddress {
		/// Neo address to import
		#[arg(short, long, help = "Neo address")]
		address: String,

		/// Label for the imported address
		#[arg(short, long, help = "Label for the imported address")]
		label: Option<String>,
	},

	/// Create a multi-signature account
	#[command(about = "Create a multi-signature account from public keys")]
	CreateMultisig {
		/// Minimum number of signatures required
		#[arg(short = 'm', long, help = "Required signature threshold")]
		min_signatures: u32,

		/// Participant public keys as repeated values or comma-separated list
		#[arg(short, long, value_delimiter = ',', num_args = 1.., help = "Compressed or uncompressed public keys in hex")]
		pubkeys: Vec<String>,

		/// Label for the multi-signature account
		#[arg(short, long, help = "Label for the account")]
		label: Option<String>,
	},

	/// Set the default wallet account
	#[command(about = "Set the default wallet account")]
	SetDefault {
		/// Address to use as the default account
		#[arg(short, long, help = "Address in the wallet")]
		address: String,
	},

	/// Sign an arbitrary message with a wallet account
	#[command(about = "Sign a message with a wallet account")]
	SignMessage {
		/// Message to sign
		#[arg(short, long, help = "Message text")]
		message: String,

		/// Signing address (uses default account if omitted)
		#[arg(short, long, help = "Address to sign with")]
		address: Option<String>,

		/// Wallet password (prompts if omitted)
		#[arg(long, help = "Wallet password")]
		password: Option<String>,
	},

	/// Export private keys
	#[command(about = "Export private keys from the wallet")]
	Export {
		/// Path to save the exported keys
		#[arg(short, long, help = "Path to save exported keys")]
		path: Option<PathBuf>,

		/// Address to export (if not specified, exports all)
		#[arg(short, long, help = "Specific address to export")]
		address: Option<String>,

		/// Export format (wif, json, csv)
		#[arg(short, long, default_value = "wif", help = "Export format")]
		format: String,
	},

	/// Show unclaimed GAS
	#[command(about = "Show unclaimed GAS for wallet addresses")]
	Gas {
		/// Address to check (if not provided, checks all addresses)
		#[arg(short, long, help = "Specific address to check")]
		address: Option<String>,
	},

	/// Change wallet password
	#[command(about = "Change the wallet password")]
	Password,

	/// Transfer assets to another address
	#[command(about = "Transfer assets to another address")]
	Send {
		/// Asset ID (NEO, GAS, or script hash)
		#[arg(short, long, help = "Asset to transfer (NEO, GAS, or token hash)")]
		asset: String,

		/// Recipient address
		#[arg(short, long, help = "Recipient address")]
		to: String,

		/// Amount to transfer
		#[arg(short = 'm', long, help = "Amount to transfer")]
		amount: String,

		/// Sender address (if not specified, uses the first account)
		#[arg(short, long, help = "Sender address")]
		from: Option<String>,

		/// Extra network fee in GAS
		#[arg(long, help = "Additional network fee in GAS")]
		fee: Option<String>,
	},

	/// Show wallet balance
	#[command(about = "Show wallet balance")]
	Balance {
		/// Address to show balance for (if not provided, shows all addresses)
		#[arg(short, long, help = "Specific address to check")]
		address: Option<String>,

		/// Only show this token (NEO, GAS, or script hash)
		#[arg(short, long, help = "Specific token to display")]
		token: Option<String>,

		/// Show detailed balance information
		#[arg(short, long, help = "Show detailed balance information")]
		detailed: bool,
	},

	/// Backup wallet
	#[command(about = "Create a backup of the wallet")]
	Backup {
		/// Path to save the backup
		#[arg(short, long, help = "Path to save the backup")]
		path: PathBuf,
	},

	/// Restore wallet from backup
	#[command(about = "Restore wallet from backup")]
	Restore {
		/// Path to the backup file
		#[arg(short, long, help = "Path to the backup file")]
		path: PathBuf,
	},

	/// Create HD wallet with mnemonic phrase
	#[command(about = "Create or restore HD wallet with BIP-39/44 support")]
	HDWallet {
		/// Create new HD wallet or restore from mnemonic
		#[arg(short, long, help = "Create new HD wallet")]
		create: bool,

		/// Mnemonic phrase for restoration (12/24 words)
		#[arg(short, long, help = "Mnemonic phrase for restoration")]
		mnemonic: Option<String>,

		/// Number of accounts to derive
		#[arg(short, long, default_value = "1", help = "Number of accounts to derive")]
		accounts: u32,

		/// Custom derivation path (BIP-44)
		#[arg(short, long, help = "Custom derivation path")]
		path: Option<String>,

		/// Save wallet to file
		#[arg(short, long, help = "Save HD wallet to file")]
		save: Option<PathBuf>,
	},

	/// Subscribe to blockchain events via WebSocket
	#[command(about = "Subscribe to real-time blockchain events")]
	Subscribe {
		/// WebSocket URL
		#[arg(short, long, help = "WebSocket endpoint URL")]
		url: Option<String>,

		/// Event types to subscribe (block, transaction, notification, etc.)
		#[arg(short, long, help = "Event types to subscribe")]
		events: Vec<String>,

		/// Filter by contract hash
		#[arg(short, long, help = "Filter by contract hash")]
		contract: Option<String>,
	},

	/// Simulate transaction before sending
	#[command(about = "Simulate transaction for gas estimation")]
	Simulate {
		/// Transaction script (base64 or hex)
		#[arg(short, long, help = "Transaction script to simulate")]
		script: String,

		/// Signers for the transaction
		#[arg(long, help = "Transaction signers")]
		signers: Vec<String>,

		/// Show detailed simulation results
		#[arg(short, long, help = "Show detailed results")]
		detailed: bool,
	},
}

/// Handle wallet command with comprehensive functionality
pub async fn handle_wallet_command(args: WalletArgs, state: &mut CliState) -> Result<(), CliError> {
	match args.command {
		WalletCommands::Create { path, name, password } => {
			handle_create_wallet(path, name, password, state).await
		},
		WalletCommands::Open { path, password } => handle_open_wallet(path, password, state).await,
		WalletCommands::Close => handle_close_wallet(state).await,
		WalletCommands::List => handle_list_addresses(state).await,
		WalletCommands::Info => handle_wallet_info(state).await,
		WalletCommands::CreateAddress { count, label } => {
			handle_create_address(count, label, state).await
		},
		WalletCommands::Import { wif_or_file, label } => {
			handle_import_key(wif_or_file, label, state).await
		},
		WalletCommands::ImportAddress { address, label } => {
			handle_import_address(address, label, state).await
		},
		WalletCommands::CreateMultisig { min_signatures, pubkeys, label } => {
			handle_create_multisig(min_signatures, pubkeys, label, state).await
		},
		WalletCommands::SetDefault { address } => handle_set_default(address, state).await,
		WalletCommands::SignMessage { message, address, password } => {
			handle_sign_message(message, address, password, state).await
		},
		WalletCommands::Export { path, address, format } => {
			handle_export_key(path, address, format, state).await
		},
		WalletCommands::Gas { address } => handle_show_gas(address, state).await,
		WalletCommands::Password => handle_change_password(state).await,
		WalletCommands::Send { asset, to, amount, from, fee } => {
			handle_transfer(asset, to, amount, from, fee, state).await
		},
		WalletCommands::Balance { address, token, detailed } => {
			handle_balance(address, token, detailed, state).await
		},
		WalletCommands::Backup { path } => handle_backup_wallet(path, state).await,
		WalletCommands::Restore { path } => handle_restore_wallet(path, state).await,
		WalletCommands::HDWallet { create, mnemonic, accounts, path, save } => {
			handle_hd_wallet(create, mnemonic, accounts, path, save, state).await
		},
		WalletCommands::Subscribe { url, events, contract } => {
			handle_websocket_subscribe(url, events, contract, state).await
		},
		WalletCommands::Simulate { script, signers, detailed } => {
			handle_transaction_simulation(script, signers, detailed, state).await
		},
	}
}

/// Create a new wallet
async fn handle_create_wallet(
	path: Option<PathBuf>,
	name: Option<String>,
	password: Option<String>,
	state: &mut CliState,
) -> Result<(), CliError> {
	print_section_header("Creating New Wallet");

	let wallet_name = name.unwrap_or_else(|| {
		prompt_input("Enter wallet name").unwrap_or_else(|_| "MyWallet".to_string())
	});

	let wallet_path = path.unwrap_or_else(|| {
		PathBuf::from(format!("{}.json", wallet_name.to_lowercase().replace(" ", "_")))
	});

	if wallet_path.exists() {
		let overwrite = prompt_yes_no(&format!(
			"Wallet file '{}' already exists. Overwrite?",
			wallet_path.display()
		))
		.map_err(|e| CliError::Io(e))?;

		if !overwrite {
			print_warning("Wallet creation cancelled");
			return Ok(());
		}
	}

	let password = match password {
		Some(pwd) => pwd,
		None => {
			let pwd = prompt_password("Enter password for the new wallet")
				.map_err(|e| CliError::Io(e))?;
			let confirm_password =
				prompt_password("Confirm password").map_err(|e| CliError::Io(e))?;

			if pwd != confirm_password {
				return Err(CliError::Wallet("Passwords do not match".to_string()));
			}
			pwd
		},
	};

	let wallet = with_loading("Creating wallet...", async {
		let mut wallet = Wallet::new();
		wallet.name = wallet_name.clone();
		wallet
			.encrypt_accounts(&password)
			.map_err(|e| CliError::WalletOperation(format!("Failed to encrypt wallet: {e}")))?;
		wallet
			.save_to_file(wallet_path.clone())
			.map_err(|e| CliError::Wallet(format!("Failed to save wallet: {e}")))?;
		Ok::<Wallet, CliError>(wallet)
	})
	.await?;

	state.wallet = Some(wallet);
	state.wallet_path = Some(wallet_path.clone());
	state.wallet_password = Some(password);

	let mut table = create_table();
	table.add_row(vec![
		Cell::new("Wallet Name").fg(Color::Cyan),
		Cell::new(&wallet_name).fg(Color::Green),
	]);
	table.add_row(vec![
		Cell::new("File Path").fg(Color::Cyan),
		Cell::new(wallet_path.display().to_string()).fg(Color::Green),
	]);
	table.add_row(vec![
		Cell::new("Status").fg(Color::Cyan),
		Cell::new("Created Successfully").fg(Color::Green),
	]);

	println!("{table}");
	print_info("💡 Use 'neo-cli wallet create-address' to create your first address");

	Ok(())
}

/// Open an existing wallet
async fn handle_open_wallet(
	path: PathBuf,
	password: Option<String>,
	state: &mut CliState,
) -> Result<(), CliError> {
	print_section_header("Opening Wallet");

	if !path.exists() {
		return Err(CliError::Wallet(format!("Wallet file not found: {}", path.display())));
	}

	let password = match password {
		Some(pwd) => pwd,
		None => prompt_password("Enter wallet password").map_err(|e| CliError::Io(e))?,
	};

	let wallet = with_loading("Opening wallet...", async {
		Wallet::open_wallet(path.as_path(), &password)
			.map_err(|e| CliError::Wallet(format!("Failed to open wallet: {e}")))
	})
	.await?;

	state.wallet = Some(wallet);
	state.wallet_path = Some(path.clone());
	state.wallet_password = Some(password);

	display_key_value("Wallet Path", &path.display().to_string());
	display_key_value("Status", "Opened Successfully");

	if let Some(wallet) = &state.wallet {
		display_key_value("Wallet Name", wallet.name());
		display_key_value("Accounts", &wallet.accounts.len().to_string());
	}

	Ok(())
}

/// Close the current wallet
async fn handle_close_wallet(state: &mut CliState) -> Result<(), CliError> {
	if state.wallet.is_none() {
		print_warning("No wallet is currently open");
		return Ok(());
	}

	state.wallet = None;
	state.wallet_path = None;
	state.wallet_password = None;
	print_success("🔒 Wallet closed successfully");
	Ok(())
}

/// List all addresses in the wallet
async fn handle_list_addresses(state: &CliState) -> Result<(), CliError> {
	let wallet = state
		.wallet
		.as_ref()
		.ok_or_else(|| CliError::Wallet("No wallet open".to_string()))?;

	print_section_header("Wallet Addresses");

	if wallet.accounts.is_empty() {
		print_warning("No addresses found in wallet");
		print_info("💡 Use 'neo-cli wallet create-address' to create an address");
		return Ok(());
	}

	let mut table = create_table();
	table.set_header(vec![
		Cell::new("#").fg(Color::Cyan),
		Cell::new("Address").fg(Color::Cyan),
		Cell::new("Label").fg(Color::Cyan),
		Cell::new("Default").fg(Color::Cyan),
		Cell::new("Status").fg(Color::Cyan),
	]);

	let mut accounts: Vec<Account> = wallet.accounts();
	accounts.sort_by_key(|a| a.get_address());

	for (index, account) in accounts.iter().enumerate() {
		let label = account.label.clone().unwrap_or_else(|| "-".to_string());
		let default_mark = if account.is_default { "Yes" } else { "" };
		let status = if account.encrypted_private_key().is_some() {
			format!("{} Encrypted", status_indicator("success"))
		} else {
			format!("{} Unencrypted", status_indicator("warning"))
		};
		table.add_row(vec![
			Cell::new((index + 1).to_string()).fg(Color::Yellow),
			Cell::new(account.get_address()).fg(Color::Green),
			Cell::new(label).fg(Color::Blue),
			Cell::new(default_mark).fg(Color::Cyan),
			Cell::new(status).fg(Color::Green),
		]);
	}

	println!("{table}");
	print_info(&format!("Total addresses: {}", wallet.accounts.len()));

	Ok(())
}

/// Show detailed wallet information
async fn handle_wallet_info(state: &CliState) -> Result<(), CliError> {
	let wallet = state
		.wallet
		.as_ref()
		.ok_or_else(|| CliError::Wallet("No wallet open".to_string()))?;

	print_section_header("Wallet Information");

	let mut table = create_table();
	table.add_row(vec![
		Cell::new("File Path").fg(Color::Cyan),
		Cell::new(
			state
				.wallet_path
				.as_ref()
				.map(|p| p.display().to_string())
				.unwrap_or_else(|| "Not saved".to_string()),
		)
		.fg(Color::Green),
	]);
	table.add_row(vec![
		Cell::new("Total Accounts").fg(Color::Cyan),
		Cell::new(wallet.accounts.len().to_string()).fg(Color::Green),
	]);
	table.add_row(vec![
		Cell::new("Wallet Name").fg(Color::Cyan),
		Cell::new(wallet.name()).fg(Color::Green),
	]);
	table.add_row(vec![
		Cell::new("Version").fg(Color::Cyan),
		Cell::new(wallet.version()).fg(Color::Green),
	]);
	table.add_row(vec![
		Cell::new("Network").fg(Color::Cyan),
		Cell::new(state.get_network_type_string()).fg(Color::Green),
	]);
	table.add_row(vec![
		Cell::new("Status").fg(Color::Cyan),
		Cell::new(format!("{} Open", status_indicator("success"))).fg(Color::Green),
	]);

	println!("{table}");

	Ok(())
}

async fn handle_create_address(
	count: u16,
	label: Option<String>,
	state: &mut CliState,
) -> Result<(), CliError> {
	let wallet = state.wallet.as_mut().ok_or_else(|| {
		CliError::WalletNotLoaded("No wallet open. Use 'wallet open' first.".into())
	})?;
	let wallet_path = state.wallet_path.clone().ok_or_else(|| {
		CliError::Config("Wallet path unknown. Re-open the wallet with 'wallet open'.".into())
	})?;

	let password = match state.wallet_password.clone() {
		Some(pwd) => pwd,
		None => prompt_password("Enter wallet password")?,
	};

	print_section_header("Creating Address");

	let mut created_addresses = Vec::new();
	for _ in 0..count {
		let (script_hash, address) = {
			let account = wallet
				.create_account()
				.map_err(|e| CliError::WalletOperation(format!("Failed to create account: {e}")))?;
			(account.get_script_hash(), account.get_address())
		};

		if let Some(label) = label.as_ref()
			&& let Some(acc) = wallet.accounts.get_mut(&script_hash)
		{
			acc.label = Some(label.clone());
		}

		created_addresses.push(address);
	}

	// Ensure all private keys are encrypted before saving.
	wallet
		.encrypt_accounts(&password)
		.map_err(|e| CliError::WalletOperation(format!("Failed to encrypt wallet: {e}")))?;
	wallet
		.save_to_file(wallet_path)
		.map_err(|e| CliError::WalletOperation(format!("Failed to save wallet: {e}")))?;

	state.wallet_password = Some(password);

	let mut table = create_table();
	table.set_header(vec![Cell::new("#").fg(Color::Cyan), Cell::new("Address").fg(Color::Cyan)]);
	for (i, addr) in created_addresses.iter().enumerate() {
		table.add_row(vec![Cell::new((i + 1).to_string()).fg(Color::Yellow), Cell::new(addr)]);
	}
	println!("{table}");

	print_success(&format!("✅ Created {} address(es)", created_addresses.len()));
	Ok(())
}

async fn handle_import_key(
	wif_or_file: String,
	label: Option<String>,
	state: &mut CliState,
) -> Result<(), CliError> {
	let wallet = state.wallet.as_mut().ok_or_else(|| {
		CliError::WalletNotLoaded("No wallet open. Use 'wallet open' first.".into())
	})?;
	let wallet_path = state.wallet_path.clone().ok_or_else(|| {
		CliError::Config("Wallet path unknown. Re-open the wallet with 'wallet open'.".into())
	})?;

	let password = match state.wallet_password.clone() {
		Some(pwd) => pwd,
		None => prompt_password("Enter wallet password")?,
	};

	print_section_header("Import Private Key");

	let candidate_path = PathBuf::from(&wif_or_file);
	let wifs: Vec<String> = if candidate_path.exists() {
		let content = std::fs::read_to_string(&candidate_path)
			.map_err(|e| CliError::FileSystem(format!("Failed to read file: {e}")))?;
		content
			.lines()
			.map(|l| l.trim())
			.filter(|l| !l.is_empty() && !l.starts_with('#'))
			.map(|l| l.to_string())
			.collect()
	} else {
		vec![wif_or_file]
	};

	if wifs.is_empty() {
		return Err(CliError::InvalidInput("No WIF keys provided".to_string()));
	}

	let mut imported = Vec::new();
	for wif in wifs {
		let mut account =
			Account::from_wif(&wif).map_err(|e| CliError::Wallet(format!("Invalid WIF: {e}")))?;
		if let Some(label) = label.as_ref() {
			account.label = Some(label.clone());
		}

		let script_hash = account.get_script_hash();
		if wallet.accounts.contains_key(&script_hash) {
			print_warning(&format!("Account already exists: {}", account.get_address()));
			continue;
		}

		let address = account.get_address();
		wallet.add_account(account);
		imported.push(address);
	}

	if imported.is_empty() {
		print_warning("No new accounts imported");
		return Ok(());
	}

	// Encrypt imported private keys and persist the wallet.
	wallet
		.encrypt_accounts(&password)
		.map_err(|e| CliError::WalletOperation(format!("Failed to encrypt wallet: {e}")))?;
	wallet
		.save_to_file(wallet_path)
		.map_err(|e| CliError::WalletOperation(format!("Failed to save wallet: {e}")))?;

	state.wallet_password = Some(password);

	let mut table = create_table();
	table.set_header(vec![Cell::new("#").fg(Color::Cyan), Cell::new("Address").fg(Color::Cyan)]);
	for (i, addr) in imported.iter().enumerate() {
		table.add_row(vec![Cell::new((i + 1).to_string()).fg(Color::Yellow), Cell::new(addr)]);
	}
	println!("{table}");
	print_success(&format!("✅ Imported {} account(s)", imported.len()));
	Ok(())
}

async fn handle_import_address(
	address: String,
	label: Option<String>,
	state: &mut CliState,
) -> Result<(), CliError> {
	let wallet = state.wallet.as_mut().ok_or_else(|| {
		CliError::WalletNotLoaded("No wallet open. Use 'wallet open' first.".into())
	})?;
	let wallet_path = state.wallet_path.clone().ok_or_else(|| {
		CliError::Config("Wallet path unknown. Re-open the wallet with 'wallet open'.".into())
	})?;

	address
		.address_to_script_hash()
		.map_err(|e| CliError::InvalidInput(format!("Invalid Neo address '{address}': {e}")))?;

	let mut account = Account::from_address(&address).map_err(|e| {
		CliError::WalletOperation(format!("Failed to create watch-only account: {e}"))
	})?;
	if let Some(label) = label {
		account.label = Some(label);
	}

	let script_hash = account.get_script_hash();
	if wallet.accounts.contains_key(&script_hash) {
		return Err(CliError::Wallet(format!("Address already exists in wallet: {address}")));
	}

	wallet.add_account(account);
	wallet
		.save_to_file(wallet_path)
		.map_err(|e| CliError::WalletOperation(format!("Failed to save wallet: {e}")))?;

	let mut table = create_table();
	table.add_row(vec![Cell::new("Address").fg(Color::Cyan), Cell::new(&address).fg(Color::Green)]);
	table.add_row(vec![
		Cell::new("Type").fg(Color::Cyan),
		Cell::new("Watch-only").fg(Color::Yellow),
	]);
	println!("{table}");
	print_success("✅ Watch-only address imported");
	Ok(())
}

async fn handle_create_multisig(
	min_signatures: u32,
	pubkeys: Vec<String>,
	label: Option<String>,
	state: &mut CliState,
) -> Result<(), CliError> {
	let wallet = state.wallet.as_mut().ok_or_else(|| {
		CliError::WalletNotLoaded("No wallet open. Use 'wallet open' first.".into())
	})?;
	let wallet_path = state.wallet_path.clone().ok_or_else(|| {
		CliError::Config("Wallet path unknown. Re-open the wallet with 'wallet open'.".into())
	})?;

	if pubkeys.is_empty() {
		return Err(CliError::InvalidInput("At least one public key is required".to_string()));
	}
	if min_signatures == 0 || min_signatures as usize > pubkeys.len() {
		return Err(CliError::InvalidInput(format!(
			"min-signatures must be between 1 and the number of public keys ({})",
			pubkeys.len()
		)));
	}

	let mut public_keys = pubkeys
		.iter()
		.map(|pubkey| {
			let bytes = hex::decode(pubkey.trim_start_matches("0x"))
				.map_err(|e| CliError::InvalidInput(format!("Invalid public key hex: {e}")))?;
			Secp256r1PublicKey::from_bytes(&bytes)
				.map_err(|e| CliError::InvalidInput(format!("Invalid public key: {e}")))
		})
		.collect::<Result<Vec<_>, _>>()?;

	let mut account = Account::multi_sig_from_public_keys(&mut public_keys, min_signatures)
		.map_err(|e| {
			CliError::WalletOperation(format!("Failed to create multi-sig account: {e}"))
		})?;
	if let Some(label) = label {
		account.label = Some(label);
	}

	let address = account.get_address();
	let script_hash = account.get_script_hash();
	if wallet.accounts.contains_key(&script_hash) {
		return Err(CliError::Wallet(format!(
			"Multi-signature account already exists in wallet: {address}"
		)));
	}

	wallet.add_account(account);
	wallet
		.save_to_file(wallet_path)
		.map_err(|e| CliError::WalletOperation(format!("Failed to save wallet: {e}")))?;

	let mut table = create_table();
	table.add_row(vec![Cell::new("Address").fg(Color::Cyan), Cell::new(&address).fg(Color::Green)]);
	table.add_row(vec![
		Cell::new("Required Signatures").fg(Color::Cyan),
		Cell::new(min_signatures.to_string()).fg(Color::Yellow),
	]);
	table.add_row(vec![
		Cell::new("Participants").fg(Color::Cyan),
		Cell::new(pubkeys.len().to_string()).fg(Color::Yellow),
	]);
	table.add_row(vec![
		Cell::new("Script Hash").fg(Color::Cyan),
		Cell::new(format!("{:x}", script_hash)).fg(Color::Blue),
	]);
	println!("{table}");
	print_success("✅ Multi-signature account created");
	Ok(())
}

async fn handle_set_default(address: String, state: &mut CliState) -> Result<(), CliError> {
	let wallet = state.wallet.as_mut().ok_or_else(|| {
		CliError::WalletNotLoaded("No wallet open. Use 'wallet open' first.".into())
	})?;
	let wallet_path = state.wallet_path.clone().ok_or_else(|| {
		CliError::Config("Wallet path unknown. Re-open the wallet with 'wallet open'.".into())
	})?;

	let script_hash = address
		.address_to_script_hash()
		.map_err(|e| CliError::InvalidInput(format!("Invalid Neo address '{address}': {e}")))?;
	if !wallet.accounts.contains_key(&script_hash) {
		return Err(CliError::Wallet(format!("Address not found in wallet: {address}")));
	}

	wallet.set_default_account(script_hash);
	wallet
		.save_to_file(wallet_path)
		.map_err(|e| CliError::WalletOperation(format!("Failed to save wallet: {e}")))?;

	print_success(&format!("✅ Default account set to {address}"));
	Ok(())
}

async fn handle_sign_message(
	message: String,
	address: Option<String>,
	password: Option<String>,
	state: &mut CliState,
) -> Result<(), CliError> {
	let wallet = state.wallet.as_ref().ok_or_else(|| {
		CliError::WalletNotLoaded("No wallet open. Use 'wallet open' first.".into())
	})?;

	let mut account =
		if let Some(address) = address {
			let script_hash = address.address_to_script_hash().map_err(|e| {
				CliError::InvalidInput(format!("Invalid Neo address '{address}': {e}"))
			})?;
			wallet.accounts.get(&script_hash).cloned().ok_or_else(|| {
				CliError::Wallet(format!("Address not found in wallet: {address}"))
			})?
		} else {
			state.get_account()?
		};

	if account.is_multi_sig() {
		return Err(CliError::InvalidInput(
			"Multi-signature accounts require transaction-specific multi-signing".to_string(),
		));
	}
	if account.key_pair().is_none() {
		let password = password
			.or_else(|| state.wallet_password.clone())
			.ok_or_else(|| CliError::Authentication("Wallet password is required".to_string()))?;
		account
			.decrypt_private_key(&password)
			.map_err(|e| CliError::WalletOperation(format!("Failed to decrypt account: {e}")))?;
	}

	let key_pair = account
		.key_pair()
		.as_ref()
		.ok_or_else(|| CliError::Wallet("Account does not contain a private key".to_string()))?;
	let signature = key_pair
		.sign(message.as_bytes())
		.map_err(|e| CliError::WalletOperation(format!("Failed to sign message: {e}")))?;

	let mut table = create_table();
	table.add_row(vec![
		Cell::new("Address").fg(Color::Cyan),
		Cell::new(account.get_address()).fg(Color::Green),
	]);
	table.add_row(vec![
		Cell::new("Message Size").fg(Color::Cyan),
		Cell::new(format!("{} bytes", message.len())).fg(Color::Yellow),
	]);
	table.add_row(vec![
		Cell::new("Signature").fg(Color::Cyan),
		Cell::new(hex::encode(signature.to_bytes())).fg(Color::Blue),
	]);
	println!("{table}");
	Ok(())
}

async fn handle_export_key(
	path: Option<PathBuf>,
	address: Option<String>,
	format: String,
	state: &CliState,
) -> Result<(), CliError> {
	let wallet = state.wallet.as_ref().ok_or_else(|| {
		CliError::WalletNotLoaded("No wallet open. Use 'wallet open' first.".into())
	})?;

	let format = format.to_lowercase();
	if !matches!(format.as_str(), "wif" | "json" | "csv") {
		return Err(CliError::InvalidArgument(
			"format".to_string(),
			"Supported formats: wif, json, csv".to_string(),
		));
	}

	let mut accounts: Vec<Account> = wallet.accounts();
	if let Some(target) = address.as_deref() {
		accounts.retain(|a| a.get_address() == target);
		if accounts.is_empty() {
			return Err(CliError::Wallet(format!("Account not found in wallet: {target}")));
		}
	}

	if accounts.is_empty() {
		return Err(CliError::Wallet("Wallet has no accounts.".to_string()));
	}

	print_warning("⚠️  Exporting private keys is sensitive and unsafe if mishandled.");
	print_warning("   Do not share exported files. Store them securely and offline.");
	if !prompt_yes_no("Proceed with exporting private keys?")? {
		return Err(CliError::UserCancelled("Export cancelled by user".to_string()));
	}

	let password = match state.wallet_password.clone() {
		Some(pwd) => pwd,
		None => prompt_password("Enter wallet password")?,
	};

	let mut exported: Vec<(String, String)> = Vec::new();
	for account in accounts {
		let mut account_clone = account.clone();
		if account_clone.key_pair().is_none() {
			if account_clone.encrypted_private_key().is_none() {
				print_warning(&format!(
					"Skipping watch-only account (no private key): {}",
					account_clone.get_address()
				));
				continue;
			}
			account_clone.decrypt_private_key(&password).map_err(|e| {
				CliError::WalletOperation(format!(
					"Failed to decrypt account {}: {e}",
					account_clone.get_address()
				))
			})?;
		}

		let key_pair = account_clone
			.key_pair()
			.as_ref()
			.ok_or_else(|| CliError::Wallet("No key pair available after decryption".to_string()))?
			.clone();
		let wif = key_pair
			.export_as_wif()
			.map_err(|e| CliError::WalletOperation(format!("Failed to export WIF: {e}")))?;
		exported.push((account_clone.get_address(), wif));
	}

	if exported.is_empty() {
		return Err(CliError::Wallet("No private keys available to export.".to_string()));
	}

	// Determine destination
	let output = match format.as_str() {
		"json" => serde_json::to_string_pretty(
			&exported
				.iter()
				.map(|(address, wif)| serde_json::json!({ "address": address, "wif": wif }))
				.collect::<Vec<_>>(),
		)?,
		"csv" => {
			let mut out = String::from("address,wif\n");
			for (address, wif) in &exported {
				out.push_str(&format!("{address},{wif}\n"));
			}
			out
		},
		_ => {
			// wif
			let mut out = String::new();
			for (address, wif) in &exported {
				out.push_str(&format!("{address}\t{wif}\n"));
			}
			out
		},
	};

	if let Some(path) = path {
		use std::fs::OpenOptions;
		use std::io::Write;

		let mut file = OpenOptions::new()
			.create(true)
			.truncate(true)
			.write(true)
			.open(&path)
			.map_err(|e| CliError::FileSystem(format!("Failed to create export file: {e}")))?;

		file.write_all(output.as_bytes())
			.map_err(|e| CliError::FileSystem(format!("Failed to write export file: {e}")))?;

		#[cfg(unix)]
		{
			use std::os::unix::fs::PermissionsExt;
			let perm = std::fs::Permissions::from_mode(0o600);
			let _ = std::fs::set_permissions(&path, perm);
		}

		print_success(&format!("✅ Exported {} key(s) to {}", exported.len(), path.display()));
		return Ok(());
	}

	// No file path provided: only allow printing when exporting a single account.
	if exported.len() != 1 {
		return Err(CliError::InvalidArgument(
			"path".to_string(),
			"Refusing to print multiple private keys to stdout. Use --path to export to a file."
				.to_string(),
		));
	}

	print_warning(
		"⚠️  Printing private keys to the terminal can leak secrets via shell history/logs.",
	);
	if !prompt_yes_no("Print the private key to stdout?")? {
		return Err(CliError::UserCancelled("Export cancelled by user".to_string()));
	}

	let (addr, wif) = &exported[0];
	println!("Address: {addr}");
	println!("WIF: {wif}");
	Ok(())
}

async fn handle_show_gas(address: Option<String>, state: &CliState) -> Result<(), CliError> {
	let wallet = state.wallet.as_ref().ok_or_else(|| {
		CliError::WalletNotLoaded("No wallet open. Use 'wallet open' first.".into())
	})?;
	let rpc_client = state.get_rpc_client()?;

	print_section_header("Unclaimed GAS");

	let mut accounts: Vec<Account> = wallet.accounts();
	if let Some(addr) = address.as_deref() {
		accounts.retain(|a| a.get_address() == addr);
		if accounts.is_empty() {
			return Err(CliError::Wallet(format!("Account not found in wallet: {addr}")));
		}
	}

	let mut table = create_table();
	table.set_header(vec![
		Cell::new("Address").fg(Color::Cyan),
		Cell::new("Unclaimed GAS").fg(Color::Cyan),
	]);

	for account in accounts {
		let script_hash = account.get_script_hash();
		let gas = rpc_client
			.get_unclaimed_gas(script_hash)
			.await
			.map_err(|e| CliError::Network(format!("Failed to query unclaimed GAS: {e}")))?;

		table.add_row(vec![Cell::new(account.get_address()), Cell::new(gas.unclaimed)]);
	}

	println!("{table}");
	Ok(())
}

async fn handle_change_password(state: &mut CliState) -> Result<(), CliError> {
	let wallet = state.wallet.as_mut().ok_or_else(|| {
		CliError::WalletNotLoaded("No wallet open. Use 'wallet open' first.".into())
	})?;
	let wallet_path = state.wallet_path.clone().ok_or_else(|| {
		CliError::Config("Wallet path unknown. Re-open the wallet with 'wallet open'.".into())
	})?;

	print_section_header("Change Wallet Password");

	let current_password = prompt_password("Enter current wallet password")?;
	if !wallet.verify_password(&current_password) {
		return Err(CliError::Authentication("Invalid current password".to_string()));
	}

	let new_password = prompt_password("Enter new wallet password")?;
	let confirm_password = prompt_password("Confirm new wallet password")?;
	if new_password != confirm_password {
		return Err(CliError::Wallet("Passwords do not match".to_string()));
	}

	wallet
		.change_password(&current_password, &new_password)
		.map_err(|e| CliError::WalletOperation(format!("Failed to change password: {e}")))?;
	wallet
		.save_to_file(wallet_path)
		.map_err(|e| CliError::WalletOperation(format!("Failed to save wallet: {e}")))?;

	state.wallet_password = Some(new_password);
	print_success("🔐 Password changed successfully");
	Ok(())
}

async fn handle_transfer(
	asset: String,
	to: String,
	amount: String,
	from: Option<String>,
	fee: Option<String>,
	state: &mut CliState,
) -> Result<(), CliError> {
	let extra_network_fee = fee
		.as_deref()
		.map(|fee| {
			let amount = neo3::sdk::DecimalAmount::parse(fee, 8)
				.map_err(|e| CliError::InvalidInput(format!("Invalid fee amount '{fee}': {e}")))?;
			amount
				.raw()
				.parse::<i64>()
				.map_err(|e| CliError::InvalidInput(format!("Invalid fee amount '{fee}': {e}")))
		})
		.transpose()?;

	let original_default = state
		.wallet
		.as_ref()
		.and_then(|w| w.default_account().map(|a| a.get_script_hash()));

	if let Some(from_address) = from.as_deref() {
		let from_hash = from_address.address_to_script_hash().map_err(|e| {
			CliError::InvalidInput(format!("Invalid sender address '{from_address}': {e}"))
		})?;

		let wallet = state.wallet.as_mut().ok_or_else(|| {
			CliError::WalletNotLoaded("No wallet open. Use 'wallet open' first.".into())
		})?;
		if !wallet.accounts.contains_key(&from_hash) {
			return Err(CliError::Wallet(format!(
				"Sender address not found in wallet: {from_address}"
			)));
		}

		wallet.set_default_account(from_hash);
	}

	let result = crate::commands::defi::tokens::transfer_token_with_fee(
		&asset,
		&to,
		&amount,
		extra_network_fee,
		state,
	)
	.await;

	if let Some(hash) = original_default
		&& let Some(wallet) = state.wallet.as_mut()
	{
		wallet.set_default_account(hash);
	}

	result
}

async fn handle_balance(
	address: Option<String>,
	token: Option<String>,
	detailed: bool,
	state: &CliState,
) -> Result<(), CliError> {
	let rpc_client = state.get_rpc_client()?;

	print_section_header("Wallet Balances");

	let mut addresses: Vec<String> = if let Some(addr) = address {
		vec![addr]
	} else {
		let wallet = state.wallet.as_ref().ok_or_else(|| {
			CliError::WalletNotLoaded("No wallet open. Use 'wallet open' first.".into())
		})?;
		wallet.accounts().into_iter().map(|a| a.get_address()).collect()
	};

	if addresses.is_empty() {
		return Err(CliError::Wallet("Wallet has no accounts.".to_string()));
	}

	addresses.sort();
	addresses.dedup();

	for address in addresses {
		let script_hash = address
			.address_to_script_hash()
			.map_err(|e| CliError::InvalidInput(format!("Invalid address '{address}': {e}")))?;

		let balances = rpc_client
			.get_nep17_balances(script_hash)
			.await
			.map_err(|e| CliError::Network(format!("Failed to query balances: {e}")))?;

		println!();
		print_info(&format!("Address: {}", balances.address));

		let token_filter = token.as_deref().map(|t| t.trim().to_string());
		let token_filter_upper = token_filter.as_deref().map(|t| t.to_uppercase());
		let token_filter_hex =
			token_filter.as_deref().map(|t| t.trim_start_matches("0x").to_lowercase());

		let mut table = create_table();
		if detailed {
			table.set_header(vec![
				Cell::new("Symbol").fg(Color::Cyan),
				Cell::new("Amount").fg(Color::Cyan),
				Cell::new("Decimals").fg(Color::Cyan),
				Cell::new("Asset Hash").fg(Color::Cyan),
				Cell::new("Updated").fg(Color::Cyan),
			]);
		} else {
			table.set_header(vec![
				Cell::new("Symbol").fg(Color::Cyan),
				Cell::new("Amount").fg(Color::Cyan),
			]);
		}

		let mut rows = 0usize;
		for bal in balances.balances {
			let symbol = bal.symbol.clone().unwrap_or_else(|| "<unknown>".to_string());
			if let Some(filter_upper) = token_filter_upper.as_deref() {
				let asset_hex = format!("{:x}", bal.asset_hash);
				let filter_hex = token_filter_hex.as_deref().unwrap_or("");
				let matches = symbol.eq_ignore_ascii_case(filter_upper)
					|| asset_hex.eq_ignore_ascii_case(filter_hex);
				if !matches {
					continue;
				}
			}

			let decimals_str = bal.decimals.clone().unwrap_or_else(|| "0".to_string());
			let decimals = decimals_str.parse::<u8>().map_err(|err| {
				CliError::Rpc(format!(
					"Invalid decimals '{}' for token {}: {}",
					decimals_str, bal.asset_hash, err
				))
			})?;
			let amount = neo3::sdk::DecimalAmount::try_from_raw(bal.amount.clone(), decimals)
				.map_err(|err| {
					CliError::Rpc(format!(
						"Invalid balance '{}' for token {}: {}",
						bal.amount, bal.asset_hash, err
					))
				})?
				.to_fixed_string();

			rows += 1;
			if detailed {
				table.add_row(vec![
					Cell::new(symbol),
					Cell::new(amount),
					Cell::new(decimals_str),
					Cell::new(format!("{:x}", bal.asset_hash)),
					Cell::new(bal.last_updated_block.to_string()),
				]);
			} else {
				table.add_row(vec![Cell::new(symbol), Cell::new(amount)]);
			}
		}

		if rows == 0 {
			print_warning("No balances found (or filtered out).");
		} else {
			println!("{table}");
		}
	}

	Ok(())
}

async fn handle_backup_wallet(path: PathBuf, state: &CliState) -> Result<(), CliError> {
	let wallet = state.wallet.as_ref().ok_or_else(|| {
		CliError::WalletNotLoaded(
			"No wallet is currently loaded. Use 'wallet open' first.".to_string(),
		)
	})?;

	// Create backup using WalletBackup
	match WalletBackup::backup(wallet, path.clone()) {
		Ok(_) => {
			println!("✅ Wallet backup created successfully!");
			println!("📁 Backup saved to: {}", path.display());
			println!("🔐 Backup contains {} accounts", wallet.accounts.len());
			println!("\n⚠️  Security reminders:");
			println!("   • Store this backup in a secure location");
			println!("   • Keep multiple copies in different locations");
			println!("   • Never share your backup file");
			println!("   • Remember your wallet password - it's required for recovery");
			Ok(())
		},
		Err(e) => Err(CliError::WalletOperation(format!("Failed to create backup: {}", e))),
	}
}

async fn handle_restore_wallet(path: PathBuf, state: &mut CliState) -> Result<(), CliError> {
	// Check if backup file exists
	if !path.exists() {
		return Err(CliError::InvalidOperation(format!(
			"Backup file not found: {}",
			path.display()
		)));
	}

	// Warn if a wallet is already loaded
	if state.wallet.is_some() {
		println!(
			"⚠️  Warning: A wallet is already loaded. Restoring will replace the current wallet."
		);
		print!("Continue? (y/N): ");
		use std::io::Write;
		std::io::stdout().flush().map_err(CliError::Io)?;

		let mut input = String::new();
		std::io::stdin().read_line(&mut input).map_err(CliError::Io)?;

		if !input.trim().to_lowercase().starts_with('y') {
			println!("Restore cancelled.");
			return Ok(());
		}
	}

	// Backups are valid NEP-6 wallets; restore by opening the backup file with a password.
	let password = prompt_password("Enter wallet password")?;
	let wallet = Wallet::open_wallet(path.as_path(), &password)
		.map_err(|e| CliError::WalletOperation(format!("Failed to open backup wallet: {e}")))?;

	println!("✅ Wallet restored successfully!");
	println!("📁 Loaded from: {}", path.display());
	println!("🏷️  Wallet name: {}", wallet.name());
	println!("🔐 Accounts loaded: {}", wallet.accounts.len());

	// Display account addresses
	println!("\n📋 Accounts:");
	let mut accounts: Vec<Account> = wallet.accounts();
	accounts.sort_by_key(|a| a.get_address());
	for (i, account) in accounts.iter().enumerate() {
		println!("   {}. {}", i + 1, account.get_address());
	}

	state.wallet = Some(wallet);
	state.wallet_path = Some(path);
	state.wallet_password = Some(password);

	Ok(())
}

// HD Wallet implementation
async fn handle_hd_wallet(
	create: bool,
	mnemonic: Option<String>,
	accounts: u32,
	derivation_path: Option<String>,
	save_path: Option<PathBuf>,
	_state: &mut CliState,
) -> Result<(), CliError> {
	print_section_header("HD Wallet Management");

	// Import HD wallet functionality from the SDK
	use neo3::sdk::hd_wallet::HDWallet;

	let mut hd_wallet = if create {
		print_info("🔑 Creating new HD wallet with BIP-39 mnemonic...");

		// Create new HD wallet
		let wallet = HDWallet::generate(24, None)
			.map_err(|e| CliError::WalletOperation(format!("Failed to create HD wallet: {}", e)))?;

		print_success(&format!("✅ HD Wallet created successfully!"));
		print_warning("⚠️ IMPORTANT: Save your mnemonic phrase securely!");
		println!();
		println!("Mnemonic phrase (24 words):");
		println!("{}", "━".repeat(60));
		println!("{}", wallet.mnemonic_phrase());
		println!("{}", "━".repeat(60));
		println!();

		wallet
	} else if let Some(phrase) = mnemonic {
		print_info("📥 Restoring HD wallet from mnemonic phrase...");

		HDWallet::from_phrase(&phrase, None, Language::English)
			.map_err(|e| CliError::WalletOperation(format!("Failed to restore HD wallet: {}", e)))?
	} else {
		return Err(CliError::WalletOperation(
			"Please specify --create or provide --mnemonic phrase".to_string(),
		));
	};

	// Derive accounts
	print_info(&format!("🔄 Deriving {} account(s)...", accounts));
	let mut table = create_table();

	for i in 0..accounts {
		let path = derivation_path.clone().unwrap_or_else(|| format!("m/44'/888'/0'/0/{}", i));
		match hd_wallet.derive_account(&path) {
			Ok(account) => {
				let pk_str = account
					.get_public_key()
					.map(|pk| pk.to_string())
					.unwrap_or_else(|| "n/a".to_string());
				table.add_row(vec![
					Cell::new(i),
					Cell::new(account.get_address()),
					Cell::new(format!("{}...", &pk_str[..pk_str.len().min(8)])),
				]);
			},
			Err(e) => {
				print_warning(&format!("Failed to derive account {}: {}", i, e));
			},
		}
	}

	println!("{}", table);

	// Save if requested
	if let Some(save_path) = save_path {
		print_info(&format!("💾 Saving HD wallet to {}...", save_path.display()));

		let wallet_data = serde_json::json!({
			"type": "HD",
			"mnemonic": hd_wallet.mnemonic_phrase(),
			"accounts": accounts,
			"derivation_path": derivation_path.unwrap_or_else(|| "m/44'/888'/0'/0/0".to_string()),
		});

		std::fs::write(&save_path, serde_json::to_string_pretty(&wallet_data)?)
			.map_err(|e| CliError::FileSystem(format!("Failed to save wallet: {}", e)))?;

		print_success(&format!("✅ HD wallet saved to {}", save_path.display()));
	}

	Ok(())
}

// WebSocket subscription implementation
async fn handle_websocket_subscribe(
	url: Option<String>,
	events: Vec<String>,
	contract: Option<String>,
	state: &mut CliState,
) -> Result<(), CliError> {
	print_section_header("WebSocket Event Subscription");

	use neo3::sdk::websocket::{SubscriptionType, WebSocketClient};

	let ws_url = url.unwrap_or_else(|| {
		if let Some(network) = &state.current_network {
			network.rpc_url.replace("http", "ws").replace("https", "wss")
		} else {
			"wss://testnet1.neo.coz.io:443/ws".to_string()
		}
	});

	print_info(&format!("🔌 Connecting to WebSocket: {}", ws_url));

	let mut client = WebSocketClient::new(&ws_url)
		.await
		.map_err(|e| CliError::Network(format!("Failed to connect to WebSocket: {}", e)))?;
	client
		.connect()
		.await
		.map_err(|e| CliError::Network(format!("WS connect error: {}", e)))?;

	// Subscribe to requested events
	if events.is_empty() || events.contains(&"block".to_string()) {
		client
			.subscribe(SubscriptionType::NewBlocks)
			.await
			.map_err(|e| CliError::Network(format!("Failed to subscribe to blocks: {}", e)))?;
		print_success("✅ Subscribed to block events");
	}

	if events.contains(&"transaction".to_string()) {
		client.subscribe(SubscriptionType::NewTransactions).await.map_err(|e| {
			CliError::Network(format!("Failed to subscribe to transactions: {}", e))
		})?;
		print_success("✅ Subscribed to transaction events");
	}

	if let Some(contract_hash) = contract {
		let hash = contract_hash
			.parse()
			.map_err(|e| CliError::InvalidInput(format!("Invalid contract hash: {}", e)))?;
		client
			.subscribe(SubscriptionType::Notifications { contract: Some(hash), name: None })
			.await
			.map_err(|e| CliError::Network(format!("Failed to subscribe to contract: {}", e)))?;
		print_success(&format!("✅ Subscribed to contract {} notifications", contract_hash));
	}

	print_info("📡 Listening for events (press Ctrl+C to stop)...");
	println!();

	// Listen for events
	if let Some(mut rx) = client.take_event_receiver() {
		while let Some((event_type, data)) = rx.recv().await {
			println!("📨 Event {:?}: {}", event_type, serde_json::to_string(&data)?);
		}
	}

	Ok(())
}

// Transaction simulation implementation
async fn handle_transaction_simulation(
	script: String,
	_signers: Vec<String>,
	_detailed: bool,
	state: &mut CliState,
) -> Result<(), CliError> {
	print_section_header("Transaction Simulation");

	use neo3::sdk::transaction_simulator::TransactionSimulator;

	let rpc_client = state.get_rpc_client()?;

	// Create simulator
	let mut simulator = TransactionSimulator::new(std::sync::Arc::new(rpc_client.clone()));

	// Parse script (hex or base64)
	let script_bytes = if script.starts_with("0x") {
		hex::decode(&script[2..])
			.map_err(|e| CliError::InvalidInput(format!("Invalid hex script: {}", e)))?
	} else {
		use base64::{Engine as _, engine::general_purpose::STANDARD};
		STANDARD
			.decode(script.as_bytes())
			.map_err(|e| CliError::InvalidInput(format!("Invalid base64 script: {}", e)))?
	};

	// Parse signers
	let signer_list = Vec::new();

	print_info("🔍 Simulating transaction...");

	// Run simulation
	match simulator.simulate_transaction(&script_bytes, signer_list).await {
		Ok(result) => {
			print_success("✅ Simulation completed successfully!");
			println!();

			// Display results
			let mut table = create_table();

			table.add_row(vec![
				Cell::new("Execution State"),
				Cell::new(if result.vm_state == NeoVMStateType::Halt {
					"SUCCESS"
				} else {
					"FAILED"
				})
				.fg(if result.vm_state == NeoVMStateType::Halt {
					Color::Green
				} else {
					Color::Red
				}),
			]);

			table.add_row(vec![
				Cell::new("GAS Consumed"),
				Cell::new(format!("{} GAS", result.gas_consumed)),
			]);

			table.add_row(vec![
				Cell::new("System Fee"),
				Cell::new(format!("{} GAS", result.system_fee)),
			]);

			table.add_row(vec![
				Cell::new("Network Fee"),
				Cell::new(format!("{} GAS", result.network_fee)),
			]);

			table.add_row(vec![
				Cell::new("Total Fee"),
				Cell::new(format!("{} GAS", result.total_fee)),
			]);

			if !result.notifications.is_empty() {
				table.add_row(vec![
					Cell::new("Notifications"),
					Cell::new(format!("{} events", result.notifications.len())),
				]);
			}

			println!("{}", table);

			// Detailed notifications are already printed above
		},
		Err(e) => {
			print_warning(&format!("❌ Simulation failed: {}", e));
			return Err(CliError::WalletOperation(format!("Transaction simulation failed: {}", e)));
		},
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use tokio::{
		io::{AsyncReadExt, AsyncWriteExt},
		net::TcpListener,
	};

	async fn state_returning_balances(body: String) -> (CliState, tokio::task::JoinHandle<()>) {
		let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
		let endpoint = format!("http://{}", listener.local_addr().unwrap());
		let server = tokio::spawn(async move {
			let (mut stream, _) = listener.accept().await.unwrap();
			let mut request = vec![0_u8; 4096];
			let _ = stream.read(&mut request).await.unwrap();

			let response = format!(
				"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
				body.len(),
				body
			);
			stream.write_all(response.as_bytes()).await.unwrap();
		});
		let provider = HttpProvider::new(endpoint.as_str()).unwrap();
		let state = CliState { rpc_client: Some(RpcClient::new(provider)), ..CliState::default() };
		(state, server)
	}

	#[tokio::test]
	async fn balance_filter_ignores_malformed_unrelated_token() {
		let body = r#"{"jsonrpc":"2.0","id":0,"result":{"address":"NUrPrFLETzoe7N2FLi2dqTvLwc9L2Em84K","balance":[{"assethash":"0xd2a4cff31913016155e38e474a2c06d08be276cf","symbol":"BAD","decimals":"not-a-number","amount":"also-invalid","lastupdatedblock":1},{"assethash":"0xef4073a0f2b305a38ec4050e4d3d28bc40ea63f5","symbol":"GAS","decimals":"8","amount":"100000000","lastupdatedblock":2}]}}"#;
		let (state, server) = state_returning_balances(body.to_string()).await;
		let result = handle_balance(
			Some("NUrPrFLETzoe7N2FLi2dqTvLwc9L2Em84K".to_string()),
			Some("GAS".to_string()),
			false,
			&state,
		)
		.await;
		server.await.unwrap();

		assert!(
			result.is_ok(),
			"filtered balance should ignore malformed unrelated tokens: {result:?}"
		);
	}

	#[tokio::test]
	async fn selected_malformed_balance_is_an_rpc_error() {
		let body = r#"{"jsonrpc":"2.0","id":0,"result":{"address":"NUrPrFLETzoe7N2FLi2dqTvLwc9L2Em84K","balance":[{"assethash":"0xd2a4cff31913016155e38e474a2c06d08be276cf","symbol":"BAD","decimals":"not-a-number","amount":"also-invalid","lastupdatedblock":1}]}}"#;
		let (state, server) = state_returning_balances(body.to_string()).await;

		let result = handle_balance(
			Some("NUrPrFLETzoe7N2FLi2dqTvLwc9L2Em84K".to_string()),
			Some("BAD".to_string()),
			false,
			&state,
		)
		.await;
		server.await.unwrap();

		assert!(matches!(result, Err(CliError::Rpc(_))));
	}
}
