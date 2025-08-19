//! Gas Estimation Example
//! 
//! This example demonstrates how to use the new real-time gas estimation
//! features in NeoRust v0.4.4 for accurate transaction fee calculation.

use neo3::prelude::*;
use neo3::neo_builder::{GasEstimator, ScriptBuilder, TransactionBuilder, Signer, AccountSigner};
use neo3::neo_clients::{HttpProvider, RpcClient, APITrait};
use neo3::neo_protocol::{Account, AccountTrait};
use neo3::neo_types::{ScriptHash, ContractParameter, ScriptHashExtension};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== NeoRust Gas Estimation Example ===\n");
    
    // Connect to Neo TestNet
    let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
    let client = RpcClient::new(provider);
    
    println!("Connected to Neo TestNet");
    let block_count = client.get_block_count().await?;
    println!("Current block height: {}\n", block_count);
    
    // Create test account
    // This is a TestNet test account - replace with your own WIF for actual use
    let account = Account::from_wif("L1eV34wPoj9weqhGijdDLtVQzUpWGHszXXpdU9dPuh2nRFFzFa7E")
        .unwrap_or_else(|_| Account::create().expect("Failed to create account"));
    
    println!("Using account: {}", account.get_address());
    
    // Example 1: Simple transfer gas estimation
    example_simple_transfer(&client, &account).await?;
    
    // Example 2: Complex contract call gas estimation
    example_contract_call(&client, &account).await?;
    
    // Example 3: Batch gas estimation for multiple operations
    example_batch_estimation(&client, &account).await?;
    
    // Example 4: Gas estimation with safety margin
    example_with_safety_margin(&client, &account).await?;
    
    Ok(())
}

async fn example_simple_transfer(
    client: &RpcClient<HttpProvider>,
    account: &Account,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 1: Simple Transfer Gas Estimation ---");
    
    // Build a simple NEO transfer script
    let neo_token = ScriptHash::from_str("ef4073a0f2b305a38ec4050e4d3d28bc40ea63f5")?;
    let recipient = ScriptHash::from_address("NbTiM6h8r99kpRtb428XcsUk1TzKed2gTc")?;
    
    let script = ScriptBuilder::new()
        .contract_call(
            &neo_token,
            "transfer",
            &[
                ContractParameter::h160(&account.get_script_hash()),
                ContractParameter::h160(&recipient),
                ContractParameter::integer(100_000_000), // 1 NEO
                ContractParameter::any(),
            ],
            None,
        )?
        .to_bytes();
    
    // Estimate gas consumption
    match GasEstimator::estimate_gas_realtime(
        client,
        &script,
        vec![Signer::called_by_entry(&account.get_script_hash())],
    ).await {
        Ok(gas) => {
            println!("Estimated gas for NEO transfer: {} GAS", gas as f64 / 100_000_000.0);
            println!("This is approximately ${:.4} USD (at $10/GAS)", 
                     (gas as f64 / 100_000_000.0) * 10.0);
        },
        Err(e) => {
            println!("Gas estimation failed (expected on testnet without balance): {}", e);
        }
    }
    
    println!();
    Ok(())
}

async fn example_contract_call(
    client: &RpcClient<HttpProvider>,
    account: &Account,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 2: Complex Contract Call Gas Estimation ---");
    
    // Build a more complex script with multiple operations
    let script = ScriptBuilder::new()
        .push_string("Hello, Neo!".to_string())
        .push_integer(42)
        .emit(OpCode::Pack)
        .push_integer(2)
        .emit(OpCode::Pack)
        .to_bytes();
    
    match GasEstimator::estimate_gas_realtime(
        client,
        &script,
        vec![],
    ).await {
        Ok(gas) => {
            println!("Estimated gas for complex operation: {} GAS", 
                     gas as f64 / 100_000_000.0);
        },
        Err(e) => {
            println!("Gas estimation error: {}", e);
        }
    }
    
    println!();
    Ok(())
}

async fn example_batch_estimation(
    client: &RpcClient<HttpProvider>,
    account: &Account,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 3: Batch Gas Estimation ---");
    
    // Prepare multiple scripts for batch estimation
    let scripts = vec![
        (
            ScriptBuilder::new()
                .push_integer(100)
                .push_integer(200)
                .emit(OpCode::Add)
                .to_bytes(),
            vec![],
        ),
        (
            ScriptBuilder::new()
                .push_string("Test1".to_string())
                .push_string("Test2".to_string())
                .emit(OpCode::Cat)
                .to_bytes(),
            vec![],
        ),
        (
            ScriptBuilder::new()
                .push_boolean(true)
                .emit(OpCode::Not)
                .to_bytes(),
            vec![],
        ),
    ];
    
    // Convert to expected format
    let scripts_ref: Vec<(&[u8], Vec<Signer>)> = scripts
        .iter()
        .map(|(script, signers)| (script.as_slice(), signers.clone()))
        .collect();
    
    match GasEstimator::batch_estimate_gas(client, scripts_ref).await {
        Ok(estimates) => {
            println!("Batch gas estimates:");
            for (i, gas) in estimates.iter().enumerate() {
                println!("  Operation {}: {} GAS", 
                         i + 1, 
                         *gas as f64 / 100_000_000.0);
            }
            
            let total: i64 = estimates.iter().sum();
            println!("Total gas for all operations: {} GAS", 
                     total as f64 / 100_000_000.0);
        },
        Err(e) => {
            println!("Batch estimation error: {}", e);
        }
    }
    
    println!();
    Ok(())
}

async fn example_with_safety_margin(
    client: &RpcClient<HttpProvider>,
    account: &Account,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 4: Gas Estimation with Safety Margin ---");
    
    // Build a script
    let script = ScriptBuilder::new()
        .push_integer(1000)
        .emit(OpCode::Sqrt)
        .to_bytes();
    
    // Estimate without margin
    let base_gas = GasEstimator::estimate_gas_realtime(
        client,
        &script,
        vec![],
    ).await.unwrap_or(0);
    
    // Estimate with 15% safety margin for production
    let safe_gas = GasEstimator::estimate_gas_with_margin(
        client,
        &script,
        vec![],
        15, // 15% margin
    ).await.unwrap_or(0);
    
    println!("Base gas estimate: {} GAS", base_gas as f64 / 100_000_000.0);
    println!("Safe gas estimate (15% margin): {} GAS", safe_gas as f64 / 100_000_000.0);
    
    if base_gas > 0 {
        let accuracy = GasEstimator::calculate_estimation_accuracy(safe_gas, base_gas);
        println!("Safety margin applied: {:.2}%", accuracy);
    }
    
    println!("\nTip: Use safety margins in production to account for:");
    println!("  - Network congestion");
    println!("  - Minor script variations");
    println!("  - Gas price fluctuations");
    
    Ok(())
}