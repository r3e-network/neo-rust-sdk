#![allow(clippy::redundant_closure)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::print_literal)]
#![allow(clippy::needless_return)]
#![allow(clippy::useless_conversion)]
#![allow(clippy::manual_strip)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::enum_variant_names)]
#![allow(clippy::wrong_self_convention)]
#![allow(clippy::single_match)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::useless_format)]
#![allow(clippy::ptr_arg)]
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::derivable_impls)]
use clap::{Parser, Subcommand};
use colored::*;
use commands::{
	contract::{handle_contract_command, ContractArgs},
	defi::{handle_defi_command, DefiArgs},
	fs::handle_fs_command,
	neofs::{handle_neofs_command, NeoFSArgs},
	network::{handle_network_command, NetworkArgs, NetworkConfig},
	nft::{handle_nft_command, NftArgs},
	tools::{handle_tools_command, ToolsArgs},
	wallet::{handle_wallet_command, CliState, WalletArgs},
};
use errors::CliError;
use std::path::PathBuf;

// Import the utils_core module
mod utils_core;

// Re-export utility functions
pub use utils_core::{
	ensure_account_loaded, print_error, print_info, print_success, print_warning, prompt_password,
	prompt_yes_no,
};

// Import config functions
use utils::config::{get_config_path, save_config, Config};

mod commands;
mod config;
mod errors;
mod utils;
mod wizard;
mod generator;

/// Neo CLI - A comprehensive command-line interface for the Neo N3 blockchain
///
/// This tool provides access to all Neo blockchain functionality including
/// wallet management, smart contracts, DeFi operations, NFTs, and more.
#[derive(Parser, Debug)]
#[clap(
	name = "neo-cli",
	version = env!("CARGO_PKG_VERSION"),
	author = "R3E Network (c) 2020-2025",
	about = "A comprehensive CLI for Neo N3 blockchain operations",
	long_about = "Neo CLI provides a complete command-line interface for interacting with the Neo N3 blockchain. \
	             Features include wallet management, smart contract deployment and interaction, DeFi operations, \
	             NFT management, network monitoring, and developer tools.",
	arg_required_else_help = true
)]
pub struct Cli {
	/// Path to config file
	#[arg(short, long, help = "Path to configuration file")]
	config: Option<PathBuf>,

	/// Enable verbose output
	#[arg(short, long, help = "Enable verbose logging")]
	verbose: bool,

	/// Output format (json, yaml, table)
	#[arg(long, default_value = "table", help = "Output format")]
	format: String,

	/// Network to use (mainnet, testnet, local)
	#[arg(short, long, help = "Network to connect to")]
	network: Option<String>,

	#[command(subcommand)]
	command: Commands,
}

/// Available commands organized by functionality
#[derive(Subcommand, Debug)]
enum Commands {
	/// Initialize a new configuration file
	#[command(about = "Initialize Neo CLI configuration")]
	Init {
		/// Path to save the configuration file
		#[arg(short, long, help = "Custom path for config file")]
		path: Option<PathBuf>,

		/// Network to configure (mainnet, testnet, local)
		#[arg(short, long, default_value = "testnet", help = "Default network")]
		network: String,

		/// Force overwrite existing config
		#[arg(short, long, help = "Overwrite existing configuration")]
		force: bool,
	},

	/// Wallet management operations
	#[command(about = "Manage Neo wallets and accounts")]
	Wallet(WalletArgs),

	/// Smart contract operations
	#[command(about = "Deploy and interact with smart contracts")]
	Contract(ContractArgs),

	/// Network operations and monitoring
	#[command(about = "Network status and blockchain operations")]
	Network(NetworkArgs),

	/// DeFi operations and protocols
	#[command(about = "DeFi protocols and token operations")]
	DeFi(DefiArgs),

	/// NFT operations and management
	#[command(about = "NFT minting, transfer, and management")]
	Nft(NftArgs),

	/// NeoFS file storage operations
	#[command(about = "Decentralized file storage on NeoFS")]
	NeoFS(NeoFSArgs),

	/// Developer tools and utilities
	#[command(about = "Developer tools and utilities")]
	Tools(ToolsArgs),

	/// Show version information
	#[command(about = "Show version and build information")]
	Version,

	/// Show configuration information
	#[command(about = "Display current configuration")]
	Config {
		/// Show configuration file path
		#[arg(short, long, help = "Show config file path")]
		path: bool,
	},

	/// NeoFS operations
	#[command(about = "NeoFS file storage operations")]
	Fs(commands::fs::FSArgs),
	
	/// Interactive wizard for guided operations
	#[command(about = "Launch interactive wizard for easy blockchain interaction")]
	Wizard,
	
	/// Generate a new Neo project from templates
	#[command(about = "Generate a new Neo project from pre-built templates")]
	Generate {
		/// Template to use (basic-dapp, nep17-token, nft, defi, oracle)
		#[arg(short, long, help = "Project template to use")]
		template: Option<String>,
		
		/// Project name
		#[arg(help = "Name of the new project")]
		name: String,
		
		/// Target directory (defaults to current directory)
		#[arg(short = 'd', long, help = "Target directory for the project")]
		dir: Option<PathBuf>,
		
		/// List available templates
		#[arg(short, long, help = "List all available templates")]
		list: bool,
	},
}

/// Initialize a new configuration file with enhanced options
async fn handle_init_command(
	path: Option<PathBuf>,
	network: String,
	force: bool,
) -> Result<(), CliError> {
	print_info("🚀 Initializing Neo CLI configuration...");

	// Check if config already exists
	let config_path =
		if let Some(custom_path) = &path { custom_path.clone() } else { get_config_path()? };

	if config_path.exists() && !force {
		print_warning(&format!(
			"Configuration file already exists at: {}\nUse --force to overwrite",
			config_path.display()
		));
		return Ok(());
	}

	// Create default config with specified network
	let mut config = Config::default();
	config.default_network = network.clone();

	// Validate network
	match network.as_str() {
		"mainnet" | "testnet" | "local" => {},
		_ => {
			print_error(&format!("Invalid network: {}. Use mainnet, testnet, or local", network));
			return Err(CliError::Config("Invalid network specified".to_string()));
		},
	}

	if let Some(custom_path) = path {
		// Create parent directories if they don't exist
		if let Some(parent) = custom_path.parent() {
			std::fs::create_dir_all(parent).map_err(|e| CliError::FileSystem(e.to_string()))?;
		}

		// Save config to custom path
		let config_str = serde_json::to_string_pretty(&config)
			.map_err(|e| CliError::Config(format!("Failed to serialize config: {}", e)))?;
		std::fs::write(&custom_path, config_str).map_err(|e| CliError::Io(e))?;

		print_success(&format!("✅ Configuration initialized at: {}", custom_path.display()));
	} else {
		// Save to default location
		save_config(&config)?;
		let config_path = get_config_path()?;
		print_success(&format!("✅ Configuration initialized at: {}", config_path.display()));
	}

	print_info(&format!("📡 Default network set to: {}", network.bright_cyan()));
	print_info("💡 Use 'neo-cli config' to view current settings");
	print_info("💡 Use 'neo-cli wallet create' to create your first wallet");

	Ok(())
}

/// Handle version command with detailed information
fn handle_version_command() -> Result<(), CliError> {
	println!("{}", "Neo CLI".bright_green().bold());
	println!("Version: {}", env!("CARGO_PKG_VERSION").bright_cyan());
	println!("Build: {}", "production".bright_yellow());
	println!("Built: {}", "2024-01-15");
	println!("Rust: {}", "1.75.0");
	println!("Target: {}", std::env::consts::ARCH);
	println!();
	println!("Neo N3 SDK: {}", "Production Ready".bright_green());
	println!("License: {}", "MIT".bright_blue());
	println!("Repository: {}", "https://github.com/R3E-Network/NeoRust".bright_blue());
	Ok(())
}

/// Handle config command
async fn handle_config_command(show_path: bool) -> Result<(), CliError> {
	if show_path {
		let config_path = get_config_path()?;
		println!("{}", config_path.display());
		return Ok(());
	}

	let config_path = get_config_path()?;
	print_info(&format!("📁 Configuration file: {}", config_path.display()));

	if config_path.exists() {
		let config_content = std::fs::read_to_string(&config_path).map_err(|e| CliError::Io(e))?;

		println!("\n{}", "Current Configuration:".bright_green().bold());
		println!("{config_content}");
	} else {
		print_warning("No configuration file found. Run 'neo-cli init' to create one.");
	}

	Ok(())
}

#[tokio::main]
async fn main() -> Result<(), CliError> {
	// Parse command line arguments
	let cli = Cli::parse();

	// Initialize logger based on verbosity
	let log_level = if cli.verbose { "debug" } else { "info" };
	env_logger::init_from_env(env_logger::Env::default().default_filter_or(log_level));

	// Print banner for interactive commands
	if !matches!(cli.command, Commands::Version | Commands::Config { .. }) {
		println!("{}", "🔷 Neo CLI".bright_green().bold());
		println!("{}", "Production-Ready Neo N3 Blockchain Interface".bright_blue());
		println!();
	}

	// Initialize default networks
	let default_networks = vec![
		NetworkConfig {
			name: "Neo N3 Mainnet".to_string(),
			rpc_url: "https://mainnet1.neo.coz.io:443".to_string(),
			network_type: "mainnet".to_string(),
			chain_id: 860833102,
			is_default: false,
		},
		NetworkConfig {
			name: "Neo N3 Testnet".to_string(),
			rpc_url: "https://testnet1.neo.coz.io:443".to_string(),
			network_type: "testnet".to_string(),
			chain_id: 894710606,
			is_default: true,
		},
	];

	// Initialize CLI state with all necessary fields
	let mut state = CliState {
		wallet: None,
		rpc_client: None,
		network_type: Some("testnet".to_string()),
		current_network: Some(default_networks[1].clone()),
		networks: default_networks,
	};

	// Set network if specified
	if let Some(network) = cli.network {
		state.network_type = Some(network);
	}

	// Handle commands
	match cli.command {
		Commands::Init { path, network, force } => handle_init_command(path, network, force).await,
		Commands::Wallet(args) => handle_wallet_command(args, &mut state).await,
		Commands::Contract(args) => handle_contract_command(args, &mut state).await,
		Commands::Network(args) => handle_network_command(args, &mut state).await,
		Commands::DeFi(args) => handle_defi_command(args, &mut state).await,
		Commands::Nft(args) => handle_nft_command(args, &mut state).await,
		Commands::NeoFS(args) => handle_neofs_command(args, &mut state).await,
		Commands::Tools(args) => handle_tools_command(args, &mut state).await,
		Commands::Version => handle_version_command(),
		Commands::Config { path } => handle_config_command(path).await,
		Commands::Fs(args) => handle_fs_command(args, &mut state).await,
		Commands::Wizard => wizard::run_wizard().await.map_err(|e| CliError::Other(e.to_string())),
		Commands::Generate { template, name, dir, list } => {
			if list {
				generator::list_templates();
				Ok(())
			} else {
				let template_type = match template.as_deref() {
					Some("basic-dapp") | None => generator::ProjectTemplate::BasicDapp,
					Some("nep17-token") => generator::ProjectTemplate::Nep17Token,
					Some("nft") => generator::ProjectTemplate::NftCollection,
					Some("defi") => generator::ProjectTemplate::DefiProtocol,
					Some("oracle") => generator::ProjectTemplate::OracleConsumer,
					Some(t) => return Err(CliError::Other(format!("Unknown template: {}", t))),
				};
				generator::generate_project(template_type, &name, dir)
					.map_err(|e| CliError::Other(e.to_string()))
			}
		},
	}
}
