//! Example demonstrating the new high-level SDK API
//! 
//! This shows how the simplified API makes common operations much easier.

use neo3::sdk::{Neo, Network, Token};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 NeoRust High-Level SDK Example\n");
    
    // Simple connection to TestNet
    println!("📡 Connecting to Neo TestNet...");
    let neo = Neo::testnet().await?;
    println!("✅ Connected successfully!\n");
    
    // Get the current block height
    let height = neo.get_block_height().await?;
    println!("📊 Current block height: {}\n", height);
    
    // Check balance for a test address
    let address = "NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc";
    println!("💰 Checking balance for: {}", address);
    
    let balance = neo.get_balance(address).await?;
    println!("   NEO: {} tokens", balance.neo);
    println!("   GAS: {} tokens", balance.gas);
    
    if !balance.tokens.is_empty() {
        println!("   Other tokens:");
        for token in &balance.tokens {
            println!("     - {}: {}", token.symbol, token.amount);
        }
    }
    
    // Example of custom configuration
    println!("\n🔧 Creating custom configured client...");
    let custom_neo = Neo::builder()
        .network(Network::MainNet)
        .timeout(Duration::from_secs(60))
        .retries(5)
        .cache(true)
        .metrics(false)
        .build()
        .await?;
    println!("✅ Custom client created for MainNet");
    
    // Get MainNet block height
    let mainnet_height = custom_neo.get_block_height().await?;
    println!("📊 MainNet block height: {}", mainnet_height);
    
    println!("\n✨ Example completed successfully!");
    
    Ok(())
}