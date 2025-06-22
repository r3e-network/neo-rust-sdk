# Transactions

This tutorial covers creating and sending transactions on the Neo N3 blockchain using the NeoRust SDK.

## Understanding Neo Transactions

Neo N3 transactions are the fundamental units of work in the Neo blockchain. They represent operations such as token transfers, smart contract invocations, and more. Each transaction has the following key components:

- **Version**: The transaction format version
- **Nonce**: A random number to prevent replay attacks
- **Sender**: The account initiating the transaction
- **System Fee**: Fee for executing the transaction
- **Network Fee**: Fee for including the transaction in a block
- **Valid Until Block**: Block height until which the transaction is valid
- **Script**: The VM script to execute
- **Signers**: Accounts that need to sign the transaction
- **Witnesses**: Signatures and verification scripts

## Creating a Basic Transaction

Here's how to create a basic transaction using the TransactionBuilder:

```rust,edition2021
use neo3::prelude::*;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo N3 TestNet node
    let provider = Provider::new_http("https://testnet1.neo.coz.io:443");
    
    // Load your wallet
    let wallet_path = Path::new("my-wallet.json");
    let password = "my-secure-password";
    let wallet = Wallet::load(wallet_path, password)?;
    
    // Get the account that will send the transaction
    let account = wallet.default_account()?;
    
    // Create a transaction
    let transaction = TransactionBuilder::new()
        .version(0)
        .nonce(rand::random::<u32>())
        .valid_until_block(provider.get_block_count().await? + 100)
        .script(
            ScriptBuilder::new()
                .emit_app_call(
                    "d2a4cff31913016155e38e474a2c06d08be276cf".parse::<ScriptHash>()?,
                    "transfer",
                    &[
                        ContractParameter::hash160(account.address().script_hash()),
                        ContractParameter::hash160("NZNos2WqTbu5oCgyfss9kUJgBXJqhuYAaj".parse::<Address>()?),
                        ContractParameter::integer(1000),
                        ContractParameter::any(None),
                    ],
                )
                .to_array()
        )
        .sign(account)?
        .build();
    
    // Send the transaction
    let txid = provider.send_raw_transaction(&transaction).await?;
    println!("Transaction sent with ID: {}", txid);
    
    // Wait for the transaction to be confirmed
    let receipt = provider.wait_for_transaction(&txid, 60, 2).await?;
    println!("Transaction confirmed: {:?}", receipt);
    
    Ok(())
}
```

## Transferring NEO or GAS

The NeoRust SDK provides convenient methods for transferring NEO and GAS tokens:

```rust,edition2021
use neo3::prelude::*;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo N3 TestNet node
    let provider = Provider::new_http("https://testnet1.neo.coz.io:443");
    
    // Load your wallet
    let wallet_path = Path::new("my-wallet.json");
    let password = "my-secure-password";
    let wallet = Wallet::load(wallet_path, password)?;
    
    // Get the account that will send the transaction
    let account = wallet.default_account()?;
    
    // Create a GAS token instance
    let gas_token = GasToken::new(provider.clone());
    
    // Transfer 1 GAS to another address
    let recipient = "NZNos2WqTbu5oCgyfss9kUJgBXJqhuYAaj".parse::<Address>()?;
    let amount = 1_00000000; // 1 GAS (with 8 decimals)
    
    let txid = gas_token.transfer(account, recipient, amount, None).await?;
    println!("GAS transfer sent with transaction ID: {}", txid);
    
    // Similarly for NEO token
    let neo_token = NeoToken::new(provider.clone());
    
    // Transfer 1 NEO to another address
    let neo_amount = 1_00000000; // 1 NEO (with 8 decimals)
    
    let neo_txid = neo_token.transfer(account, recipient, neo_amount, None).await?;
    println!("NEO transfer sent with transaction ID: {}", neo_txid);
    
    Ok(())
}
```

## Multi-signature Transactions

Neo supports multi-signature accounts, which require multiple signatures to authorize a transaction:

```rust,edition2021
use neo3::prelude::*;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo N3 TestNet node
    let provider = Provider::new_http("https://testnet1.neo.coz.io:443");
    
    // Load wallets for all signers
    let wallet1_path = Path::new("wallet1.json");
    let wallet2_path = Path::new("wallet2.json");
    let wallet3_path = Path::new("wallet3.json");
    
    let password = "my-secure-password";
    
    let wallet1 = Wallet::load(wallet1_path, password)?;
    let wallet2 = Wallet::load(wallet2_path, password)?;
    let wallet3 = Wallet::load(wallet3_path, password)?;
    
    let account1 = wallet1.default_account()?;
    let account2 = wallet2.default_account()?;
    let account3 = wallet3.default_account()?;
    
    // Create a multi-signature account (2 of 3)
    let multi_sig_account = Account::create_multi_sig(
        2,
        &[
            account1.public_key().clone(),
            account2.public_key().clone(),
            account3.public_key().clone(),
        ],
    )?;
    
    println!("Multi-signature address: {}", multi_sig_account.address());
    
    // Create a transaction from the multi-signature account
    let mut transaction = TransactionBuilder::new()
        .version(0)
        .nonce(rand::random::<u32>())
        .valid_until_block(provider.get_block_count().await? + 100)
        .script(
            ScriptBuilder::new()
                .emit_app_call(
                    "d2a4cff31913016155e38e474a2c06d08be276cf".parse::<ScriptHash>()?,
                    "transfer",
                    &[
                        ContractParameter::hash160(multi_sig_account.address().script_hash()),
                        ContractParameter::hash160("NZNos2WqTbu5oCgyfss9kUJgBXJqhuYAaj".parse::<Address>()?),
                        ContractParameter::integer(1000),
                        ContractParameter::any(None),
                    ],
                )
                .to_array()
        )
        .build();
    
    // Sign with the required number of accounts (2 of 3)
    transaction = transaction.sign(account1)?;
    transaction = transaction.sign(account2)?;
    
    // Send the transaction
    let txid = provider.send_raw_transaction(&transaction).await?;
    println!("Multi-signature transaction sent with ID: {}", txid);
    
    Ok(())
}
```

## Transaction Fees

Neo N3 transactions require two types of fees:

1. **System Fee**: Cost of executing the transaction script
2. **Network Fee**: Cost of including the transaction in a block

You can estimate these fees before sending a transaction:

```rust,edition2021
use neo3::prelude::*;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo N3 TestNet node
    let provider = Provider::new_http("https://testnet1.neo.coz.io:443");
    
    // Load your wallet
    let wallet_path = Path::new("my-wallet.json");
    let password = "my-secure-password";
    let wallet = Wallet::load(wallet_path, password)?;
    
    // Get the account that will send the transaction
    let account = wallet.default_account()?;
    
    // Create a transaction
    let transaction = TransactionBuilder::new()
        .version(0)
        .nonce(rand::random::<u32>())
        .valid_until_block(provider.get_block_count().await? + 100)
        .script(
            ScriptBuilder::new()
                .emit_app_call(
                    "d2a4cff31913016155e38e474a2c06d08be276cf".parse::<ScriptHash>()?,
                    "transfer",
                    &[
                        ContractParameter::hash160(account.address().script_hash()),
                        ContractParameter::hash160("NZNos2WqTbu5oCgyfss9kUJgBXJqhuYAaj".parse::<Address>()?),
                        ContractParameter::integer(1000),
                        ContractParameter::any(None),
                    ],
                )
                .to_array()
        )
        .build();
    
    // Estimate system fee
    let system_fee = provider.estimate_system_fee(&transaction).await?;
    println!("Estimated system fee: {} GAS", system_fee);
    
    // Estimate network fee
    let network_fee = provider.estimate_network_fee(&transaction).await?;
    println!("Estimated network fee: {} GAS", network_fee);
    
    // Total fee
    let total_fee = system_fee + network_fee;
    println!("Total estimated fee: {} GAS", total_fee);
    
    // Add fees to the transaction
    let transaction_with_fees = TransactionBuilder::from_transaction(transaction)
        .system_fee(system_fee)
        .network_fee(network_fee)
        .sign(account)?
        .build();
    
    // Send the transaction
    let txid = provider.send_raw_transaction(&transaction_with_fees).await?;
    println!("Transaction sent with ID: {}", txid);
    
    Ok(())
}
```

## Checking Transaction Status

You can check the status of a transaction after sending it:

```rust,edition2021
use neo3::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a Neo N3 TestNet node
    let provider = Provider::new_http("https://testnet1.neo.coz.io:443");
    
    // Transaction ID to check
    let txid = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".parse::<TxHash>()?;
    
    // Get transaction
    let transaction = provider.get_transaction(&txid).await?;
    
    if let Some(tx) = transaction {
        println!("Transaction found: {:?}", tx);
        
        // Get application log
        let app_log = provider.get_application_log(&txid).await?;
        
        if let Some(log) = app_log {
            println!("Transaction execution:");
            println!("  VM State: {}", log.execution.vm_state);
            println!("  Gas Consumed: {}", log.execution.gas_consumed);
            
            for (i, notification) in log.execution.notifications.iter().enumerate() {
                println!("  Notification #{}: {}", i + 1, notification.event_name);
                println!("    Contract: {}", notification.contract);
                println!("    State: {:?}", notification.state);
            }
        }
    } else {
        println!("Transaction not found. It may be pending or invalid.");
    }
    
    Ok(())
}
```

## Best Practices

1. **Always Verify Addresses**: Double-check recipient addresses before sending transactions.
2. **Set Appropriate Valid Until Block**: Set a reasonable expiration for your transactions.
3. **Estimate Fees**: Always estimate and include appropriate fees to ensure your transaction is processed.
4. **Wait for Confirmation**: Always wait for transaction confirmation before considering it complete.
5. **Error Handling**: Implement proper error handling for transaction failures.
6. **Test on TestNet**: Always test your transactions on TestNet before moving to MainNet.

<!-- toc -->
