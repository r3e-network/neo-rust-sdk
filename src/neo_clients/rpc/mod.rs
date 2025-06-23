//! # Neo RPC Client Module (v0.1.8)
//!
//! The RPC module provides client implementations for interacting with Neo nodes
//! through JSON-RPC API calls.
//!
//! ## Overview
//!
//! This module implements RPC client functionality for communicating with Neo blockchain nodes, including:
//!
//! - **Transport Protocols**: HTTP and WebSocket transport implementations
//! - **Connection Management**: Connection pooling and request batching
//! - **Pub/Sub Support**: Real-time notifications for blockchain events
//! - **Client Interfaces**: Type-safe wrappers for Neo's JSON-RPC methods
//!
//! ## Example
//!
//! ```no_run
//! use neo3::neo_clients::{HttpProvider, RpcClient};
//! use neo3::neo_types::{Address, ScriptHash};
//! use neo3::neo_protocol::{ApplicationLog, Nep17Balances};
//! use primitive_types::{H160, H256};
//! use std::str::FromStr;
//!
//! async fn rpc_examples() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create an HTTP client connected to a Neo TestNet node
//!     let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
//!     let client = RpcClient::new(provider);
//!     
//!     // Get basic blockchain information
//!     let block_count = client.get_block_count().await?;
//!     println!("Current block height: {}", block_count);
//!     
//!     let best_block_hash = client.get_best_block_hash().await?;
//!     println!("Best block hash: {}", best_block_hash);
//!     
//!     // Get detailed block information
//!     let block = client.get_block(best_block_hash, true).await?;
//!     println!("Block time: {}, tx count: {}",
//!              block.time,
//!              block.tx.as_ref().map(|txs| txs.len()).unwrap_or(0));
//!     
//!     // Query account information
//!     let address = Address::from_str("NUVPACTpQvd2HHmBgFjJJRWwVXJiR3uAEh")?;
//!     let script_hash = address.to_script_hash();
//!     
//!     // Get NEP-17 token balances
//!     let balances = client.get_nep17_balances(script_hash).await?;
//!     
//!     for balance in balances.balances {
//!         println!("Token: {}, Amount: {}",
//!                  balance.asset_hash,
//!                  balance.amount);
//!     }
//!     
//!     // Get application logs for a transaction
//!     if let Some(tx) = block.tx.as_ref().and_then(|txs| txs.first()) {
//!         let app_log = client.get_application_log(tx.hash).await?;
//!         println!("Transaction triggers: {} executions", app_log.executions.len());
//!         
//!         // Print any notifications emitted by the contract
//!         for execution in app_log.executions {
//!             for notification in execution.notifications {
//!                 println!("Contract {} emitted event: {}",
//!                          notification.contract,
//!                          notification.event_name);
//!             }
//!         }
//!     }
//!     
//!     Ok(())
//! }
//! ```

pub use connections::*;
pub use pubsub::{PubsubClient, SubscriptionStream};
pub use rpc_client::*;
pub use transports::*;

mod rpc_client;

mod connections;
mod pubsub;
mod transports;
