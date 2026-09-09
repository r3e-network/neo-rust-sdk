import React, { useState } from 'react';
import Layout from '@theme/Layout';
import CodeBlock from '@theme/CodeBlock';
import Link from '@docusaurus/Link';
import clsx from 'clsx';
import styles from './playground.module.css';

// Example categories
const categories = [
  {
    id: 'connect',
    title: 'Network & Connection',
    description: 'Connect to Neo networks and retrieve blockchain information.',
    icon: '🔌',
  },
  {
    id: 'wallet',
    title: 'Wallet Management',
    description: 'Create, load, and manage Neo wallets with NEP-6 standard.',
    icon: '💰',
  },
  {
    id: 'tokens',
    title: 'Token Operations',
    description: 'Work with NEP-17 tokens, transfers, and balance checking.',
    icon: '🪙',
  },
  {
    id: 'contracts',
    title: 'Smart Contracts',
    description: 'Deploy and interact with smart contracts on Neo N3.',
    icon: '📦',
  },
  {
    id: 'events',
    title: 'Event Queries',
    description: 'Track contract and account events using NEP-27 and NEP-91.',
    icon: '⚡',
  },
  {
    id: 'advanced',
    title: 'Advanced Features',
    description: 'Session keys, gas-less transactions, and dynamic fees.',
    icon: '🎯',
  },
  {
    id: 'neo-x',
    title: 'Neo X Integration',
    description: 'Cross-chain operations and EVM compatibility features.',
    icon: '🔗',
  },
];

// Example data - 12 production-grade examples
const examples = {
  connect: [
    {
      title: 'Connect to Neo Testnet',
      description: 'Establish connection to Neo N3 testnet and retrieve blockchain info.',
      tags: ['connection', 'rpc', 'testnet'],
      code: `use neo3::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Method 1: High-level SDK (recommended)
    let neo = Neo::testnet().await?;
    let height = neo.get_block_height().await?;
    println!("Testnet tip: {}", height);

    // Method 2: Manual RPC client for advanced control
    let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
    let client = RpcClient::new(provider);
    
    let version = client.get_version().await?;
    let block_count = client.get_block_count().await?;
    
    println!("Network: {}", version.network);
    println!("Version: {}", version.useragent);
    println!("Current block: {}", block_count);
    
    Ok(())
}`
    },
    {
      title: 'Connect to Neo Mainnet',
      description: 'Connect to Neo N3 mainnet with error handling.',
      tags: ['connection', 'mainnet', 'production'],
      code: `use neo3::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Neo N3 mainnet
    let node_url = "https://mainnet1.neo.org:443";
    let provider = HttpProvider::new(node_url)?;
    let client = RpcClient::new(provider);
    
    // Fetch chain state
    let version = client.get_version().await?;
    let next_block_index = client.get_next_block_hash(None).await?;
    let header = client.get_header(&next_block_index?).await?;
    
    println!("=== Neo N3 Mainnet ===");
    println!("Protocol version: {}", version.protocol);
    println!("Validators count: {}", version.validators_count);
    println!("Next block hash: {:?}", next_block_index);
    
    // Optional: query native contracts
    let gas = GasToken::new(&client);
    let neo_token = NeoToken::new(&client);
    
    println!("GAS total supply: {}", gas.total_supply().await?);
    println!("NEO total supply: {}", neo_token.total_supply().await?);
    
    Ok(())
}`
    },
  ],
  wallet: [
    {
      title: 'Create New Account',
      description: 'Generate a new SECP256R1 account with private key.',
      tags: ['account', 'crypto', 'keys'],
      code: `use neo3::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a brand new account with random private key
    let account = Account::create()?;
    
    // Extract critical components
    let address = account.get_address();
    let script_hash = account.get_script_hash();
    let public_key = account.get_public_key();
    let private_key = account.get_private_key();
    
    println!("=== New Account Created ===");
    println!("Address: {}", address);
    println!("Script Hash: {:?}", script_hash);
    println!("Public Key (compressed): {}", hex::encode(public_key.encode_point(true)));
    
    // SECURITY WARNING: Store private key securely!
    let private_key_hex = hex::encode(private_key.as_bytes());
    println!("Private Key (hex): {}", private_key_hex);
    println!("\n⚠️  NEVER commit private keys to version control!");
    
    // Export to NEP-2 format (encrypted JSON)
    let password = "secure_password_here";
    let nep2_encrypted = crypto::encrypt_private_key_to_nep2(
        private_key.as_bytes(),
        password,
        crypto::EncryptionAlgorithm::Aes128Ctr,
    )?;
    
    println!("\nNEP-2 encrypted JSON:");
    println!("{}", serde_json::to_string_pretty(&nep2_encrypted)?);
    
    Ok(())
}`
    },
    {
      title: 'Load Wallet from NEP-6',
      description: 'Load and decrypt an existing NEP-6 wallet file.',
      tags: ['nep6', 'wallet', 'persistence'],
      code: `use neo3::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load NEP-6 wallet from file
    let wallet_json = std::fs::read_to_string("my_wallet.json")?;
    let nep6_wallet: Nep6Wallet = serde_json::from_str(&wallet_json)?;
    
    println!("Loaded wallet: '{}'", nep6_wallet.name);
    println!("Version: {} (NEP-6)", nep6_wallet.version);
    println!("Accounts stored: {}", nep6_wallet.accounts.len());
    
    // Decrypt wallet with password to get usable wallet object
    let password = "your_password_here";
    let wallet = Wallet::from_nep6(&nep6_wallet, password)?;
    
    println!("\n=== Decrypted Wallet ===");
    
    // List all accounts with labels
    for (index, account) in wallet.get_accounts().iter().enumerate() {
        println!("\nAccount #{}:", index + 1);
        println!("  Address: {}", account.get_address());
        println!("  Script Hash: {:?}", account.get_script_hash());
        
        if let Some(label) = account.get_label() {
            println!("  Label: {}", label);
        }
        
        // Check default account
        if account.get_script_hash() == *wallet.get_default_account() {
            println!("  ⭐ Default account");
        }
    }
    
    // Get mutable reference to modify wallet
    wallet.set_default_account(*wallet.get_accounts()[0].get_script_hash());
    
    // Save updated wallet
    let updated_nep6 = wallet.to_nep6(password)?;
    std::fs::write("updated_wallet.json", serde_json::to_string_pretty(&updated_nep6)?)?;
    
    Ok(())
}`
    },
  ],
  tokens: [
    {
      title: 'Check Token Balance',
      description: 'Query GAS and custom NEP-17 token balances.',
      tags: ['nep17', 'balance', 'gas'],
      code: `use neo3::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
    let client = RpcClient::new(provider);
    
    let target_address = "NTargetAddressHere".to_script_hash()?;
    
    // Query built-in GAS token (8 decimals)
    let gas_token = GasToken::new(&client);
    let gas_balance = gas_token.balance_of(&target_address).await?;
    let gas_decimals = gas_token.decimals().await?;
    let gas_total = gas_token.total_supply().await?;
    
    println!("=== GAS Token ===");
    println!("Balance: {} atoms (raw)", gas_balance);
    println!("Decimals: {}", gas_decimals);
    println!("Human readable: {:.8} GAS", gas_balance as f64 / 1e8);
    println!("Total supply: {} GAS", gas_total as f64 / 1e8);
    
    // Query built-in NEO token (0 decimals)
    let neo_token = NeoToken::new(&client);
    let neo_balance = neo_token.balance_of(&target_address).await?;
    let neo_voting_power = neo_token.voting_power(Some(&target_address)).await?;
    
    println!("\n=== NEO Token ===");
    println!("Balance: {} NEO", neo_balance);
    println!("Voting power: {}", neo_voting_power);
    
    // Query custom NEP-17 token
    let custom_token_hash = "0xCustomTokenHash".parse()?;
    let custom_token = Nep17Token::new(&client, custom_token_hash);
    
    let symbol = custom_token.symbol().await?;
    let decimals = custom_token.decimals().await?;
    let custom_balance = custom_token.balance_of(&target_address).await?;
    
    println!("\n=== Custom Token ===");
    println!("Symbol: {}", symbol);
    println!("Name: {}", custom_token.name().await?);
    println!("Balance: {} {} ({})", custom_balance, symbol, decimals);
    
    Ok(())
}`
    },
    {
      title: 'Transfer NEP-17 Token',
      description: 'Send tokens between accounts with proper signing.',
      tags: ['transfer', 'transaction', 'signing'],
      code: `use neo3::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
    let client = RpcClient::new(provider);
    
    // Import sender account
    let private_key = vec![/* your 32-byte private key */];
    let sender = Account::from_private_key(&private_key)?;
    let signer = AccountSigner::new(sender);
    
    let gas_token = GasToken::new(&client);
    let recipient_addr = "NRecipientAddressHere";
    let recipient = recipient_addr.to_script_hash()?;
    
    // Transfer parameters
    let amount = 1_000_000_000u64; // 10 GAS (8 decimals)
    let memo = Some("Payment for services".to_string());
    
    // Execute transfer
    let tx_hash = gas_token
        .transfer(&signer, &recipient, amount, None)
        .await?;
    
    println!("Transfer initiated!");
    println!("Transaction hash: {:?}", tx_hash);
    println!("\nMonitor on TestNet Explorer:");
    println!("https://testnet.neotube.io/transaction/{}", tx_hash);
    
    // Wait for confirmation (optional polling)
    loop {
        let receipt = client.get_transaction_receipt(&tx_hash).await?;
        match receipt.state {
            VMState::Halt => {
                println!("\n✅ Transaction confirmed: {:?}", receipt.tx_hash);
                break;
            }
            VMState::Fault => {
                eprintln!("❌ Transaction failed: {:?}", receipt.fault_exception);
                break;
            }
            _ => {
                println!("⏳ Pending... state: {:?}", receipt.state);
                tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
            }
        }
    }
    
    Ok(())
}`
    },
  ],
  contracts: [
    {
      title: 'Invoke Contract Read-Only',
      description: 'Execute read-only calls to smart contracts.',
      tags: ['invoke', 'readonly', 'contract'],
      code: `use neo3::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
    let client = RpcClient::new(provider);
    
    // GAS contract script hash
    let gas_contract: H160 = "d2a4cff31913016155e38e474a2c06d08be276cf".parse()?;
    
    // Read-only invocation (no signers needed)
    let result = client
        .invoke_function(
            &gas_contract,
            "symbol",
            vec![],
            vec![],
        )
        .await?;
    
    println!("Invocation Result:");
    println!("VM State: {:?}", result.state);
    println!("Gas consumed: {} {}", result.gas_consumed, FuelUnit::NANO_GAS);
    println!("Stack: {:?}", result.stack);
    
    // Decode typical contract call
    if let Some(stack) = &result.stack {
        if let Some(StackItem::ByteString { value, .. }) = stack.first() {
            if let Ok(symbol) = String::from_utf8(value.clone()) {
                println!("\nToken symbol: {}", symbol);
            }
        }
    }
    
    // Invoke function with parameters
    let result = client
        .invoke_function(
            &gas_contract,
            "balanceOf",
            vec![ContractParameter::ByteArray(/* script hash here */)],
            vec![],
        )
        .await?;
    
    println!("\nBalance call gas: {} nano-gas", result.gas_consumed);
    
    Ok(())
}`
    },
    {
      title: 'Deploy Smart Contract',
      description: 'Deploy a compiled NEF contract to the blockchain.',
      tags: ['deploy', 'nef', 'state-changing'],
      code: `use neo3::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
    let client = RpcClient::new(provider);
    
    // Load compiled contract artifacts
    let nef_file = std::fs::read("contract.nef")?;
    let manifest_json = std::fs::read_to_string("contract.manifest.json")?;
    let manifest: ContractManifest = serde_json::from_str(&manifest_json)?;
    
    // Import deployer account
    let private_key = vec![/* your 32-byte private key */];
    let deployer = Account::from_private_key(&private_key)?;
    let signer = AccountSigner::new(deployer);
    
    // Prepare deployment with optional parameter
    let mut options = DeployContractOptions::default();
    
    // Set additional gas for complex deployments
    options.additional_system_fee = Some(1_000_000_000u64); // 10 GAS
    
    // Execute deployment
    let tx_hash = client
        .deploy_contract(&signer, &nef_file, &manifest, vec![], options)
        .await?;
    
    println!("Contract deployed!");
    println!("Transaction: {:?}", tx_hash);
    
    // The deploy response includes the deployed contract hash
    // Monitor at: https://testnet.neotube.io/transaction/{}\n\n", tx_hash);\n    
    Ok(())
}`
    },
    {
      title: 'Call Contract Method',
      description: 'Execute state-changing methods with signed transactions.',
      tags: ['method-call', 'transaction', 'signed'],
      code: `use neo3::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
    let client = RpcClient::new(provider);
    
    let account = Account::from_private_key(vec![/* private key */])?;
    let signer = AccountSigner::new(account);
    
    let contract_hash: H160 = "0xYourContractHash".parse()?;
    
    // Build a multi-parameter call
    let params = vec![
        ContractParameter::Boolean(true),
        ContractParameter::String("New Value"),
        ContractParameter::Integer(42.into()),
    ];
    
    // Execute state-changing method
    let tx_hash = client
        .invoke_function_tx(
            &signer,
            &contract_hash,
            "updateConfiguration",
            params,
            None, // Use dynamic fee estimation
        )
        .await?;
    
    println!("Method called! Transaction: {:?}", tx_hash);
    
    // Alternative: Build custom transaction for more control
    let mut builder = TransactionBuilder::new(&client);
    
    builder.add_contract_call(
        &contract_hash,
        "setValue",
        vec![ContractParameter::String("custom_value".into())],
    )?;
    
    builder.add_transfer(
        &GasToken::new(&client).script_hash(),
        &recipient_hash?,
        1_000_000_000u64,
    )?;
    
    let tx = builder.build_and_sign(&signer).await?;
    let custom_tx_hash = client.send_raw_transaction(&tx).await?;
    
    println!("Multi-operation transaction: {:?}", custom_tx_hash);
    
    Ok(())
}`
    },
  ],
  events: [
    {
      title: 'NEP-27 Contract Events',
      description: 'Query historical contract events using NEP-27 standard.',
      tags: ['nep27', 'events', 'query'],
      code: `use neo3::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
    let client = RpcClient::new(provider);
    
    // GAS token contract hash
    let gas_contract: H160 = "d2a4cff31913016155e38e474a2c06d08be276cf".parse()?;
    
    // Get current block height
    let current_height = client.get_block_count().await?;
    let from_block = current_height.saturating_sub(1000); // Last 1000 blocks
    
    // Query Transfer events (standard NEP-17 event)
    let events = ContractEventQuery::new(gas_contract)
        .event_name("Transfer")
        .block_range(from_block, current_height)
        .limit(50)
        .execute(&client)
        .await?;
    
    println!("Found {} Transfer events:", events.len());
    
    for event in &events {
        println!("\nBlock {}, TX: {:?}", event.block_index, event.tx_hash);
        println!("Event: {}", event.event_name);
        
        // Decode Transfer event payload (typically [from, to, amount])
        if let StackItem::Array { value } = &event.state {
            if value.len() >= 3 {
                println!("From: {:?}", value[0]);
                println!("To: {:?}", value[1]);
                println!("Amount: {:?}", value[2]);
            }
        }
    }
    
    // Query specific event type only
    let claims = ContractEventQuery::new(gas_contract)
        .event_name("Claims")
        .from_block(from_block)
        .to_block(current_height)
        .execute(&client)
        .await?;
    
    println!("\nAlso found {} Claim events", claims.len());
    
    Ok(())
}`
    },
    {
      title: 'NEP-91 Account Activity',
      description: 'Track all events related to a specific account using NEP-91.',
      tags: ['nep91', 'account', 'activity'],
      code: `use neo3::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
    let client = RpcClient::new(provider);
    
    // Target account address
    let account = "NKuyEwDjQKsUJzZvMhLqJWxXQyXQyXQyXQ".parse::<Address>()?;
    let account_hash = account.to_script_hash()?;
    
    let current_height = client.get_block_count().await?;
    let from_block = current_height.saturating_sub(500);
    
    // Query ALL events involving this account
    let events = AccountEventQuery::new(account_hash)
        .block_range(from_block, current_height)
        .limit(100)
        .execute(&client)
        .await?;
    
    println!("Account activity found: {} events", events.len());
    
    // Filter by event name
    let transfer_events = AccountEventQuery::new(account_hash)
        .event_name("Transfer")
        .block_range(from_block, current_height)
        .execute(&client)
        .await?;
    
    println!("Transfer events: {}", transfer_events.len());
    
    // Filter by contract
    let gas_contract: H160 = "d2a4cff31913016155e38e474a2c06d08be276cf".parse()?;
    let gas_events = AccountEventQuery::new(account_hash)
        .contract(gas_contract)
        .event_name("Transfer")
        .block_range(from_block, current_height)
        .limit(20)
        .execute(&client)
        .await?;
    
    println!("GAS-related events: {}", gas_events.len());
    
    for event in &transfer_events[..transfer_events.len().min(5)] {
        println!("\nBlock {}: {:?} - {}", event.block_index, event.tx_hash, event.event_name);
    }
    
    Ok(())
}`
    },
  ],
  advanced: [
    {
      title: 'Session Keys',
      description: 'Use temporary, limited-permission session keys for delegated access.',
      tags: ['session-keys', 'delegation', 'security'],
      code: `use neo3::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Derive session key configuration with restrictions
    let config = SessionKeyConfig {
        expiry_block: 2000000, // Expires at block 2M
        permissions: CallFlagsWrapper::READ_ONLY,
        max_spend_limit: None,
    };
    
    // Parent account (owner of session key)
    let parent_key = Account::create()?;
    let parent_pubkey = parent_key.get_public_key();
    let parent_privkey = parent_key.get_private_key();
    
    // Derive session key pair from parent
    let (session_pubkey, proof) = derive_session_key(&parent_privkey, &config)?;
    
    println!("=== Session Key Generated ===");
    println!("Parent Public Key: {:?}", parent_pubkey);
    println!("Session Public Key: {:?}", session_pubkey);
    println!("Proof length: {} bytes", proof.len());
    println!("Expiry: block {}", config.expiry_block);
    println!("Permissions: {:?}", config.permissions.value());
    
    // Create session signer for use
    let session_signer = SessionSigner::new(
        session_pubkey,
        parent_privkey.clone(),
        config.clone(),
    );
    
    println!("\nSession signer ready for use.");
    println!("Can verify: {}", session_signer.can_execute(&config));
    
    // Create time-limited readonly config
    let readonly_config = SessionKeyConfig::readonly(1900000);
    println!("\nReadonly config expiry: {}", readonly_config.expiry_block);
    
    // Create transfer-limited config  
    let transfer_config = SessionKeyConfig::transfer_only(1950000, 1_000_000_000);
    println!("Transfer limit: {} atoms max", transfer_config.max_spend_limit.unwrap());
    
    Ok(())
}`
    },
    {
      title: 'Gas-Less Relayer',
      description: 'Enable sponsored transactions where a relayer pays fees.',
      tags: ['gasless', 'relayer', 'sponsor'],
      code: `use neo3::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
    let client = RpcClient::new(provider);
    
    // User account (doesn't need GAS)
    let user_account = Account::from_private_key(vec![/* user private key */])?;
    let user_signer = AccountSigner::new(user_account);
    
    // Sponsor/Paymaster account (pays fees)
    let sponsor_address: H160 = "0xSponsorAddressHash".parse()?;
    
    // Configure gas-less transaction
    let gasless_config = GaslessConfig::new(sponsor_address);
    
    let contract_hash: H160 = "0xContractHash".parse()?;
    
    // Setup relayer witness (placeholder)
    let relayer_witness = setup_relayer_witness_for_tx(sponsor_address);
    
    // Add user signer second
    let signers = build_sponsored_signers(&user_account, sponsor_address);
    
    println!("=== Sponsored Transaction Setup ===");
    println!("User: {:?}", user_account.get_address());
    println!("Sponsor: {:?}\n", sponsor_address);
    
    // Verify relayer policy constraints
    let policy = RelayerPolicy::new()
        .with_max_fee(5_000_000_000) // Max 50 GAS per tx
        .with_rate_limit(100, 3600); // 100 tx per hour
    
    println!("Max fee cap: 50 GAS");
    println!("Rate limit: 100 tx/hour");
    
    // Sign with relayer client (sponsor side)
    let relayer_client = RelayerClient::new(gasless_config);
    
    // The actual signing happens off-chain with sponsor's private key
    // This is where sponsor validates policy before signing
    
    println!("\nUser can now invoke contracts without holding GAS!");
    println!("Sponsor handles all network/system fees.");
    
    Ok(())
}`
    },
    {
      title: 'Dynamic Fee Estimation',
      description: 'Adjust transaction fees using priority-based dynamic estimation.',
      tags: ['fee', 'priority', 'optimization'],
      code: `use neo3::sdk::fee::{FeePriority, FeePolicy, resolve_fee_adjustment};
use neo3::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
    let client = RpcClient::new(provider);
    
    let account = Account::from_private_key(vec![/* private key */])?;
    let signer = AccountSigner::new(account);
    
    // Strategy 1: Dynamic fee estimation with medium priority (SDK default)
    let dynamic_medium = FeePolicy::dynamic(FeePriority::Medium);
    println!("Dynamic Medium Priority (default)");
    println!("Margin: {}%", FeePriority::Medium.margin_percent());
    
    // Strategy 2: Aggressive high priority for urgent transactions
    let dynamic_high = FeePolicy::dynamic(FeePriority::High);
    println!("\nDynamic High Priority");
    println!("Margin: {}% over base estimate", FeePriority::High.margin_percent());
    
    // Strategy 3: No margin for cost-sensitive operations
    let low_priority = FeePolicy::dynamic(FeePriority::Low);
    println!("\nDynamic Low Priority (no margin)");
    println!("Margin: 0%");
    
    // Strategy 4: Fixed fee override (offline mode)
    let fixed_policy = FeePolicy::fixed(
        1_000_000_000u64,  // Additional system fee: 10 GAS
        500_000_000u64,     // Additional network fee: 5 GAS
    );
    println!("\nFixed Fee Override");
    println!("System fee +10 GAS, Network fee +5 GAS");
    
    // Apply fee adjustment before signing
    let adjustment = resolve_fee_adjustment(&client, &fixed_policy).await?;
    println!("\nResolved adjustment:");
    println!("Additional system fee: {} atoms", adjustment.additional_system_fee);
    println!("Additional network fee: {} atoms", adjustment.additional_network_fee);
    
    Ok(())
}`
    },
  ],
  'neo-x': [
    {
      title: 'Neo X Bridge Operations',
      description: 'Bridge assets between Neo N3 and Neo X via cross-chain bridge.',
      tags: ['bridge', 'cross-chain', 'neo-x'],
      code: `use neo3::prelude::*;
use neo3::neo_x::NeoXBridgeContract;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Neo N3
    let n3_provider = HttpProvider::new("https://testnet1.neo.org:443")?;
    let n3_client = RpcClient::new(n3_provider);
    
    // Initialize bridge contract
    let bridge = NeoXBridgeContract::new(Some(&n3_client))?;
    
    // Get supported tokens and fees
    let gas_token: H160 = "d2a4cff31913016155e38e474a2c06d08be276cf".parse()?;
    let neo_token: H160 = "d6775b4fc7a63f269ecbf3660a3cd61a28f2dd96".parse()?;
    
    let gas_fee = bridge.get_fee(&gas_token).await?;
    let neo_fee = bridge.get_fee(&neo_token).await?;
    
    println!("=== Neo X Bridge ===");
    println!("GAS bridge fee: {} atoms", gas_fee);
    println!("NEO bridge fee: {} atoms", neo_fee);
    
    // Get bridge contract status
    let status = bridge.get_status().await?;
    println!("\nBridge status: {}", status);
    println!("Active: {}", status.active);
    
    // Query bridged amounts
    let gas_bridged = bridge.get_total_bridged(&gas_token).await?;
    println!("\nTotal GAS bridged: {} atoms", gas_bridged);
    
    Ok(())
}`
    },
  ],
};

// Copy to clipboard functionality
const copyToClipboard = (text: string) => {
  navigator.clipboard.writeText(text).then(() => {
    // Could add a toast notification here
  });
};

export default function Playground(): JSX.Element {
  const [activeCategory, setActiveCategory] = useState('connect');
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);

  const currentCategory = categories.find(cat => cat.id === activeCategory);
  const currentExamples = examples[activeCategory] || [];

  const handleCopy = (code: string, index: number) => {
    copyToClipboard(code);
    setCopiedIndex(index);
    setTimeout(() => setCopiedIndex(null), 2000);
  };

  return (
    <Layout
      title="Playground"
      description="Interactive Neo Rust SDK playground - Learn by example with real-world code samples">
      
      {/* Header */}
      <header className={styles.playgroundHeader}>
        <div className="container">
          <h1 className={clsx('gradient-text', styles.pageTitle)}>
            Interactive Playground
          </h1>
          <p className={styles.pageSubtitle}>
            Explore the Neo Rust SDK through live, copyable code examples. 
            From basic connections to advanced patterns like session keys, gas-less transactions, 
            and cross-chain bridges - everything you need to build production-ready applications.
          </p>
        </div>
      </header>

      <main className={styles.playgroundMain}>
        <div className="container">
          
          {/* Category Navigation */}
          <div className={styles.categoryNav}>
            <div className={styles.categoryButtons}>
              {categories.map((category) => (
                <button
                  key={category.id}
                  onClick={() => setActiveCategory(category.id)}
                  className={clsx(
                    'btn',
                    styles.categoryButton,
                    activeCategory === category.id ? 'btn-primary' : 'btn-secondary'
                  )}
                >
                  <span>{category.icon}</span>
                  {category.title}
                </button>
              ))}
            </div>
          </div>

          {/* Category Description */}
          {currentCategory && (
            <div className={styles.categoryDescription}>
              <div className={styles.categoryInfo}>
                <h2 className={styles.categoryTitle}>
                  {currentCategory.icon} {currentCategory.title}
                </h2>
                <p className={styles.categorySubtitle}>
                  {currentCategory.description}
                </p>
              </div>
            </div>
          )}

          {/* Examples Grid */}
          <div className={styles.examplesGrid}>
            {currentExamples.map((example, index) => (
              <div key={index} className={clsx('card', styles.example)}>
                <div className={styles.exampleHeader}>
                  <div>
                    <h3 className={styles.exampleTitle}>{example.title}</h3>
                    <p className={styles.exampleDescription}>{example.description}</p>
                    <div className={styles.tags}>
                      {example.tags.map((tag) => (
                        <span key={tag} className={styles.tag}>
                          {tag}
                        </span>
                      ))}
                    </div>
                  </div>
                  <button
                    onClick={() => handleCopy(example.code, index)}
                    className={clsx('btn btn-secondary', styles.copyButton)}
                    title="Copy to clipboard"
                  >
                    {copiedIndex === index ? '✅ Copied!' : '📋'}
                  </button>
                </div>
                <div className={styles.codeContainer}>
                  <CodeBlock language="rust" showLineNumbers>
                    {example.code}
                  </CodeBlock>
                </div>
              </div>
            ))}
          </div>

          {/* CTA Section */}
          <div className={styles.ctaSection}>
            <div className={styles.ctaContent}>
              <h2 className={styles.ctaTitle}>Ready to Build?</h2>
              <p className={styles.ctaSubtitle}>
                Start building your Neo application with the Neo Rust SDK. 
                Check out our comprehensive documentation and get started today.
              </p>
              <div className={styles.ctaButtons}>
                <Link to="/docs/intro" className={clsx('btn btn-primary', styles.ctaButton)}>
                  📚 Read Documentation
                </Link>
                <Link to="/sdk/intro" className={clsx('btn btn-secondary', styles.ctaButton)}>
                  🦀 Explore SDK
                </Link>
                <a 
                  href="https://github.com/r3e-network/neo-rust-sdk"
                  className={clsx('btn btn-secondary', styles.ctaButton)}
                  target="_blank" 
                  rel="noopener noreferrer"
                >
                  ⭐ View on GitHub
                </a>
              </div>
            </div>
          </div>

        </div>
      </main>
    </Layout>
  );
} 