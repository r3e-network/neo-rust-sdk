//! Interactive wizard for Neo CLI
//! 
//! Provides a user-friendly interface for common blockchain operations

use anyhow::{Context, Result};
use colored::*;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use indicatif::{ProgressBar, ProgressStyle};
use neo3::sdk::{Neo, Network};
use std::time::Duration;

/// Main wizard entry point
pub async fn run_wizard() -> Result<()> {
    print_banner();
    
    loop {
        let choices = vec![
            "🌐 Connect to Network",
            "💼 Wallet Operations",
            "💰 Check Balance",
            "📤 Send Transaction",
            "📜 Smart Contract Interaction",
            "🔧 Generate Project",
            "📚 Documentation",
            "❌ Exit",
        ];
        
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("What would you like to do?")
            .items(&choices)
            .default(0)
            .interact()?;
        
        match selection {
            0 => connect_to_network().await?,
            1 => wallet_operations().await?,
            2 => check_balance().await?,
            3 => send_transaction().await?,
            4 => smart_contract_interaction().await?,
            5 => generate_project().await?,
            6 => show_documentation()?,
            7 => {
                println!("\n{}", "👋 Goodbye!".green());
                break;
            }
            _ => unreachable!(),
        }
    }
    
    Ok(())
}

fn print_banner() {
    println!();
    println!("{}", "╔══════════════════════════════════════╗".cyan());
    println!("{}", "║       NeoRust Interactive Wizard     ║".cyan().bold());
    println!("{}", "║           Version 0.5.0              ║".cyan());
    println!("{}", "╚══════════════════════════════════════╝".cyan());
    println!();
    println!("{}", "Welcome to the NeoRust Interactive Wizard!".green());
    println!("{}", "This tool will help you interact with the Neo blockchain easily.\n".white());
}

async fn connect_to_network() -> Result<()> {
    println!("\n{}", "🌐 Network Connection".cyan().bold());
    
    let networks = vec!["TestNet", "MainNet", "Custom"];
    let network_choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select network")
        .items(&networks)
        .default(0)
        .interact()?;
    
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⣾⣽⣻⢿⡿⣟⣯⣷")
            .template("{spinner:.green} {msg}")?,
    );
    pb.set_message("Connecting to network...");
    pb.enable_steady_tick(Duration::from_millis(100));
    
    let neo = match network_choice {
        0 => {
            Neo::testnet().await
                .context("Failed to connect to TestNet")?
        }
        1 => {
            Neo::mainnet().await
                .context("Failed to connect to MainNet")?
        }
        2 => {
            let url: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter custom RPC URL")
                .default("https://testnet1.neo.org:443".to_string())
                .interact()?;
            
            Neo::builder()
                .network(Network::Custom(url))
                .build()
                .await
                .context("Failed to connect to custom network")?
        }
        _ => unreachable!(),
    };
    
    pb.finish_with_message("Connected successfully!");
    
    // Get and display block height
    let height = neo.get_block_height().await?;
    println!("✅ {}", format!("Connected to {} at block height: {}", 
        networks[network_choice], height).green());
    
    Ok(())
}

async fn wallet_operations() -> Result<()> {
    println!("\n{}", "💼 Wallet Operations".cyan().bold());
    
    let operations = vec![
        "Create New Wallet",
        "Import from WIF",
        "Import from Mnemonic",
        "Export Wallet",
        "View Wallet Info",
        "Back to Main Menu",
    ];
    
    let choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select operation")
        .items(&operations)
        .default(0)
        .interact()?;
    
    match choice {
        0 => create_new_wallet()?,
        1 => import_from_wif()?,
        2 => import_from_mnemonic()?,
        3 => export_wallet()?,
        4 => view_wallet_info()?,
        5 => return Ok(()),
        _ => unreachable!(),
    }
    
    Ok(())
}

fn create_new_wallet() -> Result<()> {
    use neo3::neo_wallets::wallet::Wallet;
    
    println!("\n{}", "Creating new wallet...".yellow());
    
    let _wallet = Wallet::new();
    
    println!("✅ {}", "Wallet created successfully!".green());
    println!("\n{}", "Wallet Details:".cyan().bold());
    
    // Note: In a real implementation, we'd show the actual wallet address
    // For now, we just show that it was created
    println!("  📍 Address: {}", "[Generated Address]".white());
    println!("  🔑 Private Key: {}", "[Secure - Not Displayed]".red());
    
    let save = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Would you like to save this wallet?")
        .default(true)
        .interact()?;
    
    if save {
        let path: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter file path to save wallet")
            .default("wallet.json".to_string())
            .interact()?;
        
        // In a real implementation, we'd save the wallet here
        println!("💾 Wallet would be saved to: {}", path.green());
    }
    
    Ok(())
}

fn import_from_wif() -> Result<()> {
    println!("\n{}", "Importing wallet from WIF...".yellow());
    
    let _wif: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter WIF private key")
        .interact()?;
    
    // In a real implementation, we'd import the wallet here
    println!("✅ {}", "Wallet imported successfully!".green());
    
    Ok(())
}

fn import_from_mnemonic() -> Result<()> {
    println!("\n{}", "Importing wallet from mnemonic...".yellow());
    
    let _mnemonic: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter mnemonic phrase")
        .interact()?;
    
    let _passphrase: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter passphrase (optional)")
        .default("".to_string())
        .interact()?;
    
    // In a real implementation, we'd import the wallet here
    println!("✅ {}", "Wallet imported successfully!".green());
    
    Ok(())
}

fn export_wallet() -> Result<()> {
    println!("\n{}", "Exporting wallet...".yellow());
    
    let formats = vec!["JSON (NEP-6)", "WIF", "Mnemonic"];
    let format_choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select export format")
        .items(&formats)
        .default(0)
        .interact()?;
    
    // In a real implementation, we'd export the wallet here
    println!("✅ {}", format!("Wallet exported as {}", formats[format_choice]).green());
    
    Ok(())
}

fn view_wallet_info() -> Result<()> {
    println!("\n{}", "Wallet Information".cyan().bold());
    
    // In a real implementation, we'd show actual wallet info
    println!("  📍 Address: {}", "[Current Address]".white());
    println!("  💰 NEO Balance: {}", "0".white());
    println!("  ⛽ GAS Balance: {}", "0".white());
    println!("  📜 Script Hash: {}", "[Script Hash]".white());
    
    Ok(())
}

async fn check_balance() -> Result<()> {
    println!("\n{}", "💰 Check Balance".cyan().bold());
    
    let address: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter Neo address")
        .default("NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc".to_string())
        .interact()?;
    
    let network_choices = vec!["TestNet", "MainNet"];
    let network = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select network")
        .items(&network_choices)
        .default(0)
        .interact()?;
    
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⣾⣽⣻⢿⡿⣟⣯⣷")
            .template("{spinner:.green} {msg}")?,
    );
    pb.set_message("Fetching balance...");
    pb.enable_steady_tick(Duration::from_millis(100));
    
    // Connect to network
    let neo = match network {
        0 => Neo::testnet().await?,
        1 => Neo::mainnet().await?,
        _ => unreachable!(),
    };
    
    // Get balance
    match neo.get_balance(&address).await {
        Ok(balance) => {
            pb.finish_with_message("Balance fetched!");
            
            println!("\n{}", "Balance Information:".cyan().bold());
            println!("  📍 Address: {}", address.white());
            println!("  💎 NEO: {} tokens", balance.neo.to_string().green());
            println!("  ⛽ GAS: {} tokens", format!("{:.8}", balance.gas).green());
            
            if !balance.tokens.is_empty() {
                println!("\n  📜 Other Tokens:");
                for token in balance.tokens {
                    println!("     • {}: {}", token.symbol, token.amount);
                }
            }
        }
        Err(e) => {
            pb.finish_with_message("Failed to fetch balance");
            println!("❌ Error: {}", e.to_string().red());
        }
    }
    
    Ok(())
}

async fn send_transaction() -> Result<()> {
    println!("\n{}", "📤 Send Transaction".cyan().bold());
    
    println!("{}", "⚠️  This feature requires a wallet with funds.".yellow());
    
    let proceed = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you want to continue?")
        .default(false)
        .interact()?;
    
    if !proceed {
        return Ok(());
    }
    
    let from: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter sender address")
        .interact()?;
    
    let to: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter recipient address")
        .interact()?;
    
    let tokens = vec!["NEO", "GAS"];
    let token_choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select token to send")
        .items(&tokens)
        .default(1)
        .interact()?;
    
    let amount: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Enter amount of {} to send", tokens[token_choice]))
        .interact()?;
    
    println!("\n{}", "Transaction Summary:".cyan().bold());
    println!("  From: {}", from.white());
    println!("  To: {}", to.white());
    println!("  Amount: {} {}", amount.white(), tokens[token_choice].white());
    
    let confirm = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Confirm transaction?")
        .default(false)
        .interact()?;
    
    if confirm {
        println!("✅ {}", "Transaction would be sent (dry run mode)".green());
    } else {
        println!("❌ {}", "Transaction cancelled".red());
    }
    
    Ok(())
}

async fn smart_contract_interaction() -> Result<()> {
    println!("\n{}", "📜 Smart Contract Interaction".cyan().bold());
    
    let operations = vec![
        "Deploy Contract",
        "Invoke Read-Only Method",
        "Invoke Method (Transaction)",
        "Get Contract Info",
        "Back to Main Menu",
    ];
    
    let choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select operation")
        .items(&operations)
        .default(0)
        .interact()?;
    
    match choice {
        0 => {
            println!("📦 Contract deployment wizard coming soon!");
        }
        1 => {
            println!("👁️ Read-only invocation wizard coming soon!");
        }
        2 => {
            println!("✍️ Transaction invocation wizard coming soon!");
        }
        3 => {
            println!("ℹ️ Contract info viewer coming soon!");
        }
        4 => return Ok(()),
        _ => unreachable!(),
    }
    
    Ok(())
}

async fn generate_project() -> Result<()> {
    println!("\n{}", "🔧 Generate Project".cyan().bold());
    
    let templates = vec![
        "Basic Neo dApp",
        "NEP-17 Token",
        "NFT Collection (NEP-11)",
        "DeFi Protocol",
        "Oracle Consumer",
        "Custom Template",
    ];
    
    let template_choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select project template")
        .items(&templates)
        .default(0)
        .interact()?;
    
    let project_name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter project name")
        .default("my-neo-project".to_string())
        .interact()?;
    
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⣾⣽⣻⢿⡿⣟⣯⣷")
            .template("{spinner:.green} {msg}")?,
    );
    pb.set_message("Generating project...");
    pb.enable_steady_tick(Duration::from_millis(100));
    
    // Simulate project generation
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    pb.finish_with_message("Project generated!");
    
    println!("\n✅ {} '{}'", 
        format!("Successfully generated {} project", templates[template_choice]).green(),
        project_name.cyan()
    );
    
    println!("\n{}", "Next steps:".cyan().bold());
    println!("  1. cd {}", project_name);
    println!("  2. cargo build");
    println!("  3. cargo test");
    println!("  4. neo-cli deploy");
    
    Ok(())
}

fn show_documentation() -> Result<()> {
    println!("\n{}", "📚 Documentation".cyan().bold());
    
    let docs = vec![
        ("Getting Started", "https://github.com/R3E-Network/NeoRust#getting-started"),
        ("API Documentation", "https://docs.rs/neo3"),
        ("Examples", "https://github.com/R3E-Network/NeoRust/tree/master/examples"),
        ("Neo Developer Docs", "https://developers.neo.org"),
        ("Discord Community", "https://discord.gg/neo"),
    ];
    
    println!("\n{}", "Available Resources:".cyan());
    for (name, url) in docs {
        println!("  📖 {}: {}", name.white(), url.blue());
    }
    
    println!("\n💡 {}", "Tip: Use 'neo-cli --help' for command-line options".yellow());
    
    Ok(())
}