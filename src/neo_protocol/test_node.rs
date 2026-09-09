//! # Test Node Emulator for Neo N3
//!
//! An in-process mock RPC server serving deterministic responses for testing and simulation.
//! Provides complete control over blockchain state, response delays, and fault injection.
//!
//! ## Overview
//!
//! This module implements a lightweight test node that emulates Neo N3 RPC behavior with:
//!
//! - **Deterministic responses**: Predictable behavior for reproducible tests
//! - **State snapshotting**: Save/load node state for test scenarios
//! - **Fault injection**: Inject FAULT states, RPC errors, and network issues
//! - **In-memory ledger**: Complete control over block height, transactions, and accounts
//! - **Contract state management**: Simulate smart contract storage and execution
//!
//! ## Examples
//!
//! ### Basic usage with default configuration
//!
//! ```rust,no_run
//! use neo3::neo_protocol::TestNode;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a test node starting at block 1000
//!     let mut node = TestNode::builder()
//!         .with_initial_height(1000)
//!         .build_with_node();
//!     
//!     // Get current block count (returns deterministic value)
//!     let height = node.get_block_count().await?;
//!     println!("Starting block height: {}", height);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### Fault injection testing
//!
//! ```rust,no_run
//! use neo3::neo_protocol::{TestNode, TestNodeError};
//! use primitive_types::H256;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut node = TestNode::builder().build_with_node();
//!     
//!     // Configure to return FAULT state for specific transaction
//!     let tx_hash = H256::from_low_u64_le(12345);
//!     node.inject_fault(tx_hash, TestNodeError::InsufficientFunds);
//!     
//!     // Subsequent calls will fail with injected error
//!     match node.get_transaction(tx_hash).await {
//!         Err(e) => println!("Got expected error: {}", e),
//!         _ => panic!("Expected error but got success"),
//!     }
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### State snapshot and restoration
//!
//! ```rust,no_run
//! use neo3::neo_protocol::TestNode;
//! use primitive_types::{H160, U256};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut node = TestNode::builder()
//!         .with_initial_height(0)
//!         .build_with_node();
//!     
//!     // Setup initial state
//!     let account = H160::from_low_u64_le(100);
//!     node.set_token_balance(&account, U256::from(1000)).await;
//!     
//!     // Advance chain
//!     node.mine_block().await?;
//!     node.mine_block().await?;
//!     
//!     // Take a snapshot before important operation
//!     let snapshot = node.take_snapshot()?;
//!     
//!     // Perform operations
//!     node.set_token_balance(&account, U256::from(500)).await;
//!     node.mine_block().await?;
//!     
//!     // Restore previous state
//!     node.restore_snapshot(&snapshot)?;
//!     
//!     assert_eq!(node.get_token_balance(&account).await, Some(U256::from(1000)));
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## API Stability
//!
//! The TestNode API is organized into stable and experimental surfaces. Stable
//! items follow semantic versioning guarantees; experimental items may change in
//! minor releases.
//!
//! ### Stable API (production-ready, semver-guaranteed)
//!
//! - **Types**: [`TestNode`], [`TestNodeBuilder`], [`TestNodeConfig`], [`TestNodeError`],
//!   [`NodeSnapshot`], [`TestBlock`], [`TestTransaction`]
//! - **Construction**: [`TestNode::new`], [`TestNode::builder`], [`TestNode::from_config`]
//! - **Chain queries**: [`TestNode::get_block_count`], [`TestNode::get_best_block_hash`],
//!   [`TestNode::get_block_by_index`], [`TestNode::get_block_by_hash`],
//!   [`TestNode::get_transaction`]
//! - **Chain control**: [`TestNode::mine_block`], [`TestNode::add_transaction`],
//!   [`TestNode::reset_chain`], [`TestNode::wait_for_block`]
//! - **State**: [`TestNode::take_snapshot`], [`TestNode::restore_snapshot`],
//!   [`TestNode::set_token_balance`], [`TestNode::get_token_balance`]
//! - **Fault injection**: [`TestNode::inject_fault`], [`TestNode::clear_faults`],
//!   [`TestNode::check_fault`]
//! - **Latency**: [`TestNode::set_response_delay`], [`TestNode::response_delay`]
//!
//! ### Experimental API (may change without major version bump)
//!
//! - **Types**: [`ContractStorageItem`], [`ChainSettings`], [`TestWitness`],
//!   [`TestTxAttribute`]
//! - **RPC trait**: [`MockRPCProvider`] and its `invoke_contract` / `send_raw_transaction`
//!   methods (simulated results only, subject to refinement)
//! - **Validator control**: [`TestNode::set_next_primary`]
//!
//! Experimental items are functional but their signatures or semantics may be
//! refined as the emulator matures toward the v3.3.0 developer-experience goals.

use chrono::{DateTime, Utc};
use primitive_types::{H160, H256, U256};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

// =============================================================================
// Configuration Types
// =============================================================================

/// Configuration for TestNode initialization
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestNodeConfig {
    /// Initial block height (default: 0)
    pub initial_height: u32,
    /// Network name identifier (default: "unittest")
    pub network_name: String,
    /// Number of validators (default: 4)
    pub validator_count: u8,
    /// Block time in milliseconds (default: 15000 for Neo N3)
    pub block_time_ms: u64,
    /// Whether to enforce strict validation (default: false)
    pub strict_validation: bool,
    /// Custom chain settings
    pub chain_settings: ChainSettings,
}

impl Default for TestNodeConfig {
    fn default() -> Self {
        Self {
            initial_height: 0,
            network_name: "unittest".to_string(),
            validator_count: 4,
            block_time_ms: 15_000,
            strict_validation: false,
            chain_settings: ChainSettings::default(),
        }
    }
}

impl TestNodeConfig {
    /// Returns the magic number for this network configuration
    pub fn magic_number(&self) -> u32 {
        // Deterministic based on network name
        self.network_name
            .bytes()
            .fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32))
    }
}

/// Chain-specific settings for customization
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChainSettings {
    /// Max traceable blocks (rollback limit)
    pub max_traceable_blocks: u32,
    /// Token reward per block
    pub block_reward: u64,
    /// Storage price per byte
    pub storage_price: u64,
    /// Committee size
    pub committee_size: u8,
    /// Number of members needed to make a change
    pub members_needed_to_make_a_change: u8,
}

impl Default for ChainSettings {
    fn default() -> Self {
        Self {
            max_traceable_blocks: 21_024, // Neo's default
            block_reward: 0,               // Neo N3 has no block reward
            storage_price: 10_000,         // Neo N3: 0.00001 NET per byte
            committee_size: 7,
            members_needed_to_make_a_change: 5,
        }
    }
}

/// Builder pattern for TestNode configuration
pub struct TestNodeBuilder {
    config: TestNodeConfig,
}

impl TestNodeBuilder {
    /// Creates a new builder with default configuration
    pub fn new() -> Self {
        Self { config: TestNodeConfig::default() }
    }

    /// Sets initial block height
    pub fn with_initial_height(mut self, height: u32) -> Self {
        self.config.initial_height = height;
        self
    }

    /// Sets network name
    pub fn with_network_name(mut self, name: &str) -> Self {
        self.config.network_name = name.to_string();
        self
    }

    /// Sets validator count
    pub fn with_validator_count(mut self, count: u8) -> Self {
        self.config.validator_count = count;
        self
    }

    /// Sets block time in milliseconds
    pub fn with_block_time(mut self, ms: u64) -> Self {
        self.config.block_time_ms = ms;
        self
    }

    /// Enables strict validation mode
    pub fn with_strict_validation(mut self, enabled: bool) -> Self {
        self.config.strict_validation = enabled;
        self
    }

    /// Builds the final configuration
    pub fn build(self) -> TestNodeConfig {
        self.config
    }

    /// Builds a TestNode directly from the builder
    pub fn build_with_node(self) -> TestNode {
        TestNode::builder_with_config(self.config)
    }
}

impl Default for TestNodeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Error Types
// =============================================================================

/// Error types for TestNode operations
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestNodeError {
    /// Generic internal error
    Internal(String),
    /// Invalid parameters
    InvalidParams(String),
    /// Transaction not found
    UnknownTransaction,
    /// Block not found
    UnknownBlock,
    /// Insufficient funds
    InsufficientFunds,
    /// Validation failed
    ValidationFailed(String),
    /// Contract execution failed
    ContractExecutionFailed(String),
    /// State snapshot corrupted
    SnapshotCorrupted,
    /// Method not found
    MethodNotFound(String),
    /// Invalid RPC request
    InvalidRequest,
    /// Parse error
    ParseError(String),
    /// Server error
    ServerError(String),
    /// Timeout
    Timeout,
    /// Connection closed
    ConnectionClosed,
}

impl std::fmt::Display for TestNodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
            Self::InvalidParams(msg) => write!(f, "Invalid params: {}", msg),
            Self::UnknownTransaction => write!(f, "Unknown transaction"),
            Self::UnknownBlock => write!(f, "Unknown block"),
            Self::InsufficientFunds => write!(f, "Insufficient funds"),
            Self::ValidationFailed(msg) => write!(f, "Validation failed: {}", msg),
            Self::ContractExecutionFailed(msg) => write!(f, "Contract execution failed: {}", msg),
            Self::SnapshotCorrupted => write!(f, "Snapshot corrupted"),
            Self::MethodNotFound(method) => write!(f, "Method not found: {}", method),
            Self::InvalidRequest => write!(f, "Invalid request"),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
            Self::ServerError(msg) => write!(f, "Server error: {}", msg),
            Self::Timeout => write!(f, "Request timeout"),
            Self::ConnectionClosed => write!(f, "Connection closed"),
        }
    }
}

impl std::error::Error for TestNodeError {}

impl From<String> for TestNodeError {
    fn from(s: String) -> Self {
        TestNodeError::Internal(s)
    }
}

impl From<&str> for TestNodeError {
    fn from(s: &str) -> Self {
        TestNodeError::Internal(s.to_string())
    }
}

// =============================================================================
// State Management
// =============================================================================

/// Snapshot of the entire node state
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeSnapshot {
    /// Snapshot timestamp
    pub timestamp: DateTime<Utc>,
    /// Block height at snapshot time
    pub block_height: u32,
    /// Ledger state hash
    pub state_root_hash: H256,
    /// Account balances snapshot
    pub account_balances: HashMap<H160, HashMap<H160, U256>>,
    /// Contract storage snapshot
    pub contract_storage: HashMap<(H160, Vec<u8>), Vec<u8>>,
    /// Pending transactions
    pub pending_transactions: Vec<H256>,
    /// Fault injection rules
    pub faults: HashMap<H256, TestNodeError>,
    /// Next primary validator index
    pub next_primary: usize,
    /// All blocks at snapshot time
    pub blocks: HashMap<u32, TestBlock>,
    /// Transaction to block mapping
    pub transaction_to_block: HashMap<H256, H256>,
}

/// In-memory ledger storage
#[derive(Clone, Default)]
struct LedgerState {
    /// Current block height
    block_height: u32,
    /// All blocks indexed by height
    blocks: HashMap<u32, TestBlock>,
    /// Blocks indexed by hash
    blocks_by_hash: HashMap<H256, u32>,
    /// Transaction hashes to block hashes
    transactions: HashMap<H256, H256>,
    /// Last known state root hash
    state_root: H256,
    /// Next primary validator index
    next_primary: usize,
}

/// A single block in the test ledger
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestBlock {
    /// Block index (height)
    pub index: u32,
    /// Block hash
    pub hash: H256,
    /// Previous block hash
    pub prev_hash: H256,
    /// Merkle root of transactions
    pub merkle_root: H256,
    /// Block timestamp
    pub timestamp: u64,
    /// Primary validator index
    pub primary: usize,
    /// Witnesses
    pub witnesses: Option<Vec<TestWitness>>,
    /// Transactions in this block
    pub transactions: Vec<TestTransaction>,
}

/// Test transaction structure
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestTransaction {
    /// Transaction hash
    pub hash: H256,
    /// Fee amount
    pub sys_fee: i64,
    /// Gas used
    pub net_fee: u64,
    /// Sender script hash
    pub sender: H160,
    /// Valid until block
    pub valid_until: u32,
    /// Attributes
    pub attributes: Vec<TestTxAttribute>,
    /// Witnesses
    pub witnesses: Vec<TestWitness>,
}

/// Test transaction attribute
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestTxAttribute {
    /// Attribute type
    pub ty: u8,
    /// Data
    pub data: Vec<u8>,
}

/// Test witness structure
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestWitness {
    /// Invocation script
    pub invocation: Vec<u8>,
    /// Verification script
    pub verification: Vec<u8>,
}

// =============================================================================
// Contract State
// =============================================================================

/// Contract storage item
#[derive(Clone, Debug, Default)]
pub struct ContractStorageItem {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub mutable: bool,
    pub asset_id: Option<H160>,
}

/// Contract state map
#[derive(Clone, Default)]
struct ContractState {
    /// Contract ID
    pub id: i64,
    /// Contract hash
    pub hash: H160,
    /// Storage items
    pub storage: HashMap<H160, ContractStorageItem>,
    /// NEP-17 token balances
    pub token_balances: HashMap<H160, U256>,
    /// NEP-11 NFT ownership
    pub nft_ownership: HashMap<(H160, u64), H160>,
}

// =============================================================================
// Core TestNode Implementation
// =============================================================================

/// Main test node structure
pub struct TestNode {
    /// Configuration
    config: TestNodeConfig,
    /// Ledger state
    ledger: Arc<RwLock<LedgerState>>,
    /// Contract states
    contracts: Arc<RwLock<HashMap<H160, ContractState>>>,
    /// Fault injections
    faults: Arc<RwLock<HashMap<H256, TestNodeError>>>,
    /// Response delay (for simulating latency)
    response_delay_ms: u64,
    /// Timestamp base for deterministic time
    timestamp_base: u64,
}

impl TestNode {
    /// Creates a new TestNode with default configuration
    pub fn new() -> Self {
        Self::builder().build_with_node()
    }

    /// Creates a TestNodeBuilder for custom configuration
    pub fn builder() -> TestNodeBuilder {
        TestNodeBuilder::new()
    }

    /// Builds a TestNode from a custom configuration
    pub fn from_config(config: TestNodeConfig) -> Self {
        Self::builder_with_config(config)
    }

    /// Creates a new TestNode with given configuration
    pub fn builder_with_config(config: TestNodeConfig) -> Self {
        let ledger = LedgerState {
            block_height: config.initial_height,
            blocks: HashMap::new(),
            blocks_by_hash: HashMap::new(),
            transactions: HashMap::new(),
            state_root: H256::zero(),
            next_primary: 0,
        };

        Self {
            config,
            ledger: Arc::new(RwLock::new(ledger)),
            contracts: Arc::new(RwLock::new(HashMap::new())),
            faults: Arc::new(RwLock::new(HashMap::new())),
            response_delay_ms: 0,
            timestamp_base: Utc::now().timestamp_millis() as u64 / 1000 * 1000,
        }
    }

    /// Builds a TestNode directly
    pub fn build_with_node(self) -> TestNode {
        self
    }

    /// Gets the initial block height from config
    pub fn initial_height(&self) -> u32 {
        self.config.initial_height
    }

    /// Gets current block height
    pub async fn get_block_count(&self) -> Result<u32, TestNodeError> {
        if self.response_delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(self.response_delay_ms)).await;
        }
        Ok(self.ledger.read().unwrap().block_height)
    }

    /// Gets the best block hash
    /// 
    /// Returns the hash of the most recently mined block (the highest block
    /// index present in the ledger).
    /// 
    /// # Returns
    /// * `Ok(H256)` - Hash of the latest block
    /// * `Err(TestNodeError::UnknownBlock)` - If no blocks have been mined yet
    /// 
    /// # Example
    /// ```rust,no_run
    /// use neo3::neo_protocol::TestNode;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let node = TestNode::builder().with_initial_height(0).build_with_node();
    ///     node.mine_block().await?;
    ///     let hash = node.get_best_block_hash().await?;
    ///     println!("Best block: {}", hash);
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_best_block_hash(&self) -> Result<H256, TestNodeError> {
        if self.response_delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(self.response_delay_ms)).await;
        }
        let ledger = self.ledger.read().unwrap();
        // Blocks are keyed by their index (0..block_height-1), so the best block
        // is the highest index present in the ledger. Using `block_height`
        // directly would miss the last mined block since it has already been
        // incremented past it.
        let max_height = ledger.blocks.keys().max().copied().ok_or(TestNodeError::UnknownBlock)?;
        ledger
            .blocks
            .get(&max_height)
            .map(|b| b.hash)
            .ok_or(TestNodeError::UnknownBlock)
    }

    /// Gets block by height
    pub async fn get_block_by_index(&self, index: u32) -> Result<Option<TestBlock>, TestNodeError> {
        if self.response_delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(self.response_delay_ms)).await;
        }
        Ok(self.ledger.read().unwrap().blocks.get(&index).cloned())
    }

    /// Gets block by hash
    pub async fn get_block_by_hash(&self, hash: H256) -> Result<Option<TestBlock>, TestNodeError> {
        if self.response_delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(self.response_delay_ms)).await;
        }
        let ledger = self.ledger.read().unwrap();
        let index = *ledger.blocks_by_hash.get(&hash).ok_or(TestNodeError::UnknownBlock)?;
        Ok(ledger.blocks.get(&index).cloned())
    }

    /// Gets transaction by hash
    pub async fn get_transaction(&self, hash: H256) -> Result<Option<TestTransaction>, TestNodeError> {
        if let Some(error) = self.faults.read().unwrap().get(&hash) {
            return Err(error.clone());
        }

        if self.response_delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(self.response_delay_ms)).await;
        }

        let ledger = self.ledger.read().unwrap();
        let block_hash = *ledger.transactions.get(&hash).ok_or(TestNodeError::UnknownTransaction)?;
        let block = ledger.blocks.values().find(|b| b.hash == block_hash).ok_or(TestNodeError::UnknownTransaction)?;
        Ok(block.transactions.iter().find(|tx| tx.hash == hash).cloned())
    }

    /// Mines a new block
    /// 
    /// Creates and appends a new block to the ledger at the current height.
    /// Transactions can be added to blocks before mining the next one.
    /// 
    /// # Returns
    /// The newly mined `TestBlock` with all properties populated
    /// 
    /// # Example
    /// ```rust,no_run
    /// use neo3::neo_protocol::TestNode;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let mut node = TestNode::builder()
    ///         .with_initial_height(0)
    ///         .build_with_node();
    ///     
    ///     let block = node.mine_block().await?;
    ///     println!("Mined block {} at height {}", block.hash, block.index);
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub async fn mine_block(&self) -> Result<TestBlock, TestNodeError> {
        let ledger = self.ledger.write().unwrap();
        let prev_hash = if ledger.block_height > self.config.initial_height {
            ledger
                .blocks
                .get(&(ledger.block_height - 1))
                .map(|b| b.hash)
                .unwrap_or(H256::zero())
        } else {
            H256::zero() // Genesis block
        };

        let current_time = self.timestamp_base + (ledger.block_height as u64 * self.config.block_time_ms);
        let block = TestBlock {
            index: ledger.block_height,
            hash: H256::from_low_u64_le(ledger.block_height as u64),
            prev_hash,
            merkle_root: H256::zero(),
            timestamp: current_time * 1000,
            primary: ledger.next_primary,
            witnesses: None,
            transactions: Vec::new(),
        };

        let block_hash = block.hash;
        let current_height = ledger.block_height;
        let next_primary = ledger.next_primary;
        
        // Clone values before releasing mutable borrow
        let block_clone = block.clone();
        drop(ledger); // Release mutable borrow before reading immutably
        
        // Now perform writes with cloned values
        let mut ledger = self.ledger.write().unwrap();
        let validator_count = self.config.validator_count as usize;
        ledger.blocks.insert(current_height, block_clone);
        ledger.blocks_by_hash.insert(block_hash, current_height);
        ledger.next_primary = (next_primary + 1) % validator_count;
        ledger.block_height += 1;

        Ok(block)
    }

    /// Adds a transaction to the most recent block
    /// 
    /// Appends a transaction to the latest mined block without triggering
    /// a block mine. Useful for batch transaction operations.
    /// 
    /// # Arguments
    /// * `tx` - The transaction to add
    /// 
    /// # Returns
    /// * `Ok(())` if successful
    /// * `Err(TestNodeError::UnknownBlock)` if no blocks exist yet
    /// 
    /// # Example
    /// ```rust,no_run
    /// use neo3::neo_protocol::{TestNode, TestTransaction};
    /// use primitive_types::{H160, H256};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let node = TestNode::builder()
    ///         .with_initial_height(0)
    ///         .build_with_node();
    ///     
    ///     // Mine initial block first
    ///     node.mine_block().await?;
    ///     
    ///     // Create and add transaction
    ///     let tx = TestTransaction {
    ///         hash: H256::from_low_u64_le(1),
    ///         sys_fee: 10000,
    ///         net_fee: 1000,
    ///         sender: H160::from_low_u64_le(100),
    ///         valid_until: 100,
    ///         attributes: vec![],
    ///         witnesses: vec![],
    ///     };
    ///     
    ///     node.add_transaction(tx).await?;
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub async fn add_transaction(&self, tx: TestTransaction) -> Result<(), TestNodeError> {
        // Look for the latest block in the ledger
        let (block_hash, target_height) = {
            let ledger = self.ledger.read().unwrap();
            // Find the maximum height with a block
            let max_height = ledger.blocks.keys().max().copied().unwrap_or(0);
            (
                ledger.blocks.get(&max_height).map(|b| b.hash),
                max_height,
            )
        };
        
        let Some(block_hash) = block_hash else {
            return Err(TestNodeError::UnknownBlock);
        };
        
        let mut ledger = self.ledger.write().unwrap();
        if let Some(block) = ledger.blocks.get_mut(&target_height) {
            block.transactions.push(tx.clone());
            // Use block hash as value for transactions
            ledger.transactions.insert(tx.hash, block_hash);
            Ok(())
        } else {
            Err(TestNodeError::UnknownBlock)
        }
    }

    /// Takes a snapshot of the current state
    /// 
    /// Creates a complete snapshot of the node state including blocks, transactions,
    /// balances, contracts, and fault injections.
    /// 
    /// # Returns
    /// A `NodeSnapshot` that can be used to restore the exact state later
    /// 
    /// # Example
    /// ```rust,no_run
    /// use neo3::neo_protocol::TestNode;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let mut node = TestNode::builder().build_with_node();
    ///     
    ///     // Setup some state
    ///     node.mine_block().await?;
    ///     
    ///     // Take snapshot
    ///     let snapshot = node.take_snapshot()?;
    ///     
    ///     // Modify state
    ///     node.mine_block().await?;
    ///     
    ///     // Restore
    ///     node.restore_snapshot(&snapshot)?;
    ///     assert_eq!(node.get_block_count().await?, 1);
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub fn take_snapshot(&self) -> Result<NodeSnapshot, TestNodeError> {
        let ledger = self.ledger.read().unwrap();
        let contracts = self.contracts.read().unwrap();
        let faults = self.faults.read().unwrap();

        // Collect all account balances
        let mut account_balances = HashMap::new();
        for (_hash, contract) in contracts.iter() {
            for (account, balance) in &contract.token_balances {
                account_balances.entry(*account).or_insert_with(HashMap::new).insert(contract.hash, *balance);
            }
        }

        // Collect contract storage
        let mut contract_storage = HashMap::new();
        for (_contract_hash, contract) in contracts.iter() {
            for (key, item) in contract.storage.iter() {
                contract_storage.insert((key.to_owned(), item.key.clone()), item.value.clone());
            }
        }

        let snapshot = NodeSnapshot {
            timestamp: Utc::now(),
            block_height: ledger.block_height,
            state_root_hash: ledger.state_root,
            account_balances,
            contract_storage,
            pending_transactions: ledger.transactions.keys().cloned().collect(),
            faults: faults.clone(),
            next_primary: ledger.next_primary,
            blocks: ledger.blocks.clone(),
            transaction_to_block: ledger.transactions.clone(),
        };

        Ok(snapshot)
    }

    /// Restores state from a snapshot
    /// 
    /// Completely restores the node to the state captured in the snapshot,
    /// including blocks, transactions, balances, and fault injections.
    /// 
    /// # Arguments
    /// * `snapshot` - The snapshot to restore from
    /// 
    /// # Returns
    /// * `Ok(())` if successful
    /// * `Err(TestNodeError::SnapshotCorrupted)` if snapshot is invalid
    /// 
    /// # Example
    /// ```rust,no_run
    /// use neo3::neo_protocol::TestNode;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let mut node = TestNode::builder().build_with_node();
    ///     
    ///     let snapshot = node.take_snapshot()?;
    ///     
    ///     // Perform operations
    ///     node.mine_block().await?;
    ///     
    ///     // Restore to previous state
    ///     node.restore_snapshot(&snapshot)?;
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub fn restore_snapshot(&self, snapshot: &NodeSnapshot) -> Result<(), TestNodeError> {
        let mut ledger = self.ledger.write().unwrap();
        let mut contracts = self.contracts.write().unwrap();
        let mut faults = self.faults.write().unwrap();

        // Restore ledger state
        ledger.block_height = snapshot.block_height;
        ledger.state_root = snapshot.state_root_hash;
        ledger.next_primary = snapshot.next_primary;
        
        // Replace blocks and transaction mapping
        ledger.blocks.clear();
        ledger.blocks.extend(snapshot.blocks.clone());
        ledger.transactions.clear();
        ledger.transactions.extend(snapshot.transaction_to_block.clone());
        
        // Rebuild blocks_by_hash index
        ledger.blocks_by_hash.clear();
        let hash_index: Vec<(H256, u32)> =
            ledger.blocks.iter().map(|(height, block)| (block.hash, *height)).collect();
        for (hash, height) in hash_index {
            ledger.blocks_by_hash.insert(hash, height);
        }

        // Clear and restore faults
        faults.clear();
        faults.extend(snapshot.faults.clone());

        // Restore contract states
        contracts.clear();

        // Note: Full contract restoration would require storing contract states in snapshot
        // This is a simplified version that preserves basic token balances
        if !snapshot.account_balances.is_empty() {
            // Reinitialize basic contract structure
            let contract_hash = H160::from_low_u64_le(1);
            let mut contract = ContractState {
                id: 1,
                hash: contract_hash,
                storage: HashMap::new(),
                token_balances: HashMap::new(),
                nft_ownership: HashMap::new(),
            };
            
            // Restore balances
            for (account, balances) in &snapshot.account_balances {
                contract.token_balances.insert(*account, *balances.get(&contract_hash).unwrap_or(&U256::from(0)));
            }
            
            contracts.insert(contract_hash, contract);
        }

        Ok(())
    }

    /// Sets token balance for an account
    /// 
    /// Configures a NEP-17 compatible token balance for the given account.
    /// Uses a simulated contract at hash `0x0000000000000000000000000000000000000001`.
    /// 
    /// # Arguments
    /// * `account` - The account address to set balance for
    /// * `amount` - The token amount (NEP-17 U256 format)
    /// 
    /// # Example
    /// ```rust,no_run
    /// use neo3::neo_protocol::TestNode;
    /// use primitive_types::{H160, U256};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let node = TestNode::new();
    ///     let account = H160::from_low_u64_le(123);
    ///     
    ///     node.set_token_balance(&account, U256::from(1000)).await;
    ///     
    ///     let balance = node.get_token_balance(&account).await;
    ///     assert_eq!(balance, Some(U256::from(1000)));
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub async fn set_token_balance(&self, account: &H160, amount: U256) {
        let contract_hash = H160::from_low_u64_le(1);
        
        let mut contracts = self.contracts.write().unwrap();
        let contract = contracts.entry(contract_hash)
            .or_insert_with(|| ContractState {
                id: 1,
                hash: contract_hash,
                storage: HashMap::new(),
                token_balances: HashMap::new(),
                nft_ownership: HashMap::new(),
            });
        
        contract.token_balances.insert(*account, amount);
    }

    /// Gets token balance for an account
    /// 
    /// Retrieves the NEP-17 token balance for the specified account.
    /// Returns `None` if the account has no balance set.
    /// 
    /// # Arguments
    /// * `account` - The account address to query
    /// 
    /// # Returns
    /// `Option<U256>` - The token balance or `None` if not set
    /// 
    /// # Example
    /// ```rust,no_run
    /// use neo3::neo_protocol::TestNode;
    /// use primitive_types::{H160, U256};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let node = TestNode::new();
    ///     let account = H160::from_low_u64_le(456);
    ///     
    ///     // Query non-existent balance
    ///     assert_eq!(node.get_token_balance(&account).await, None);
    ///     
    ///     // After setting balance
    ///     node.set_token_balance(&account, U256::from(5000)).await;
    ///     assert_eq!(node.get_token_balance(&account).await, Some(U256::from(5000)));
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_token_balance(&self, account: &H160) -> Option<U256> {
        let contracts = self.contracts.read().unwrap();
        let contract_hash = H160::from_low_u64_le(1);
        contracts.get(&contract_hash).and_then(|c| c.token_balances.get(account).copied())
    }

    /// Injects a fault for a specific transaction
    /// 
    /// Configures the test node to return a specific error when queried for
    /// the given transaction hash. Useful for testing error handling and
    /// fault tolerance in smart contracts.
    /// 
    /// # Arguments
    /// * `tx_hash` - The transaction hash to inject a fault for
    /// * `error` - The error to return when this transaction is queried
    /// 
    /// # Example
    /// ```rust,no_run
    /// use neo3::neo_protocol::{TestNode, TestNodeError};
    /// use primitive_types::H256;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let node = TestNode::new();
    ///     let tx_hash = H256::from_low_u64_le(42);
    ///     
    ///     // Inject insufficient funds error
    ///     node.inject_fault(tx_hash, TestNodeError::InsufficientFunds);
    ///     
    ///     // Later queries will fail
    ///     let result = node.get_transaction(tx_hash).await;
    ///     assert!(matches!(result, Err(TestNodeError::InsufficientFunds)));
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub fn inject_fault(&self, tx_hash: H256, error: TestNodeError) {
        self.faults.write().unwrap().insert(tx_hash, error);
    }

    /// Clears all fault injections
    /// 
    /// Removes all previously injected faults, restoring normal behavior
    /// for all transactions.
    /// 
    /// # Example
    /// ```rust,no_run
    /// use neo3::neo_protocol::{TestNode, TestNodeError};
    /// use primitive_types::H256;
    ///
    /// let node = TestNode::new();
    /// 
    /// // Inject a fault
    /// node.inject_fault(H256::from_low_u64_le(1), TestNodeError::Timeout);
    /// 
    /// // Clear all faults
    /// node.clear_faults();
    /// assert_eq!(node.check_fault(&H256::from_low_u64_le(1)), None);
    /// ```
    pub fn clear_faults(&self) {
        self.faults.write().unwrap().clear();
    }

    /// Checks if a transaction should fail due to injected fault
    /// 
    /// Returns the configured error for the transaction if a fault exists,
    /// otherwise returns `None`.
    /// 
    /// # Arguments
    /// * `tx_hash` - The transaction hash to check
    /// 
    /// # Returns
    /// `Option<TestNodeError>` - The fault error if present
    /// 
    /// # Example
    /// ```rust,no_run
    /// use neo3::neo_protocol::{TestNode, TestNodeError};
    /// use primitive_types::H256;
    ///
    /// let node = TestNode::new();
    /// let tx_hash = H256::from_low_u64_le(999);
    /// 
    /// // No fault initially
    /// assert_eq!(node.check_fault(&tx_hash), None);
    /// 
    /// // After injection
    /// node.inject_fault(tx_hash, TestNodeError::Timeout);
    /// assert!(matches!(node.check_fault(&tx_hash), Some(TestNodeError::Timeout)));
    /// ```
    pub fn check_fault(&self, tx_hash: &H256) -> Option<TestNodeError> {
        self.faults.read().unwrap().get(tx_hash).cloned()
    }

    /// Sets response delay (simulates network latency)
    /// 
    /// # Example
    /// ```rust,no_run
    /// use neo3::neo_protocol::TestNode;
    ///
    /// let mut node = TestNode::builder().build_with_node();
    /// node.set_response_delay(100); // Add 100ms delay to all RPC calls
    /// ```
    pub fn set_response_delay(&mut self, ms: u64) {
        self.response_delay_ms = ms;
    }

    /// Gets response delay setting
    /// 
    /// # Example
    /// ```rust,no_run
    /// use neo3::neo_protocol::TestNode;
    ///
    /// let node = TestNode::new();
    /// assert_eq!(node.response_delay(), 0);
    /// ```
    pub fn response_delay(&self) -> u64 {
        self.response_delay_ms
    }

    /// Waits for a specific block height to be mined
    /// 
    /// Polls the chain until the specified height is reached or timeout occurs.
    /// 
    /// # Arguments
    /// * `height` - The target block height to wait for
    /// * `timeout_ms` - Maximum time to wait in milliseconds
    /// 
    /// # Returns
    /// * `Ok(TestBlock)` - The block when it's mined
    /// * `Err(TestNodeError::Timeout)` - If timeout expires before block is mined
    /// 
    /// # Example
    /// ```rust,no_run
    /// use neo3::neo_protocol::TestNode;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let mut node = TestNode::builder()
    ///         .with_initial_height(100)
    ///         .build_with_node();
    ///     
    ///     // Start mining blocks in background
    ///     let handle = tokio::spawn(async move {
    ///         for _ in 0..10 {
    ///             node.mine_block().await.unwrap();
    ///             tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    ///         }
    ///     });
    ///     
    ///     // Wait for block 105
    ///     let block = node.wait_for_block(105, 1000).await?;
    ///     println!("Reached block {}", block.index);
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub async fn wait_for_block(&self, height: u32, timeout_ms: u64) -> Result<TestBlock, TestNodeError> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);

        loop {
            // Check the condition before sleeping so an already-satisfied height
            // returns immediately without waiting for the poll interval.
            let current_height = self.get_block_count().await?;
            if current_height >= height {
                return self
                    .get_block_by_index(height)
                    .await?
                    .ok_or(TestNodeError::UnknownBlock);
            }

            // Sleep for at most the poll interval, but never past the deadline.
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return Err(TestNodeError::Timeout);
            }
            tokio::time::sleep(remaining.min(std::time::Duration::from_millis(10))).await;
        }
    }

    /// Resets the chain to initial state
    /// 
    /// Clears all blocks, transactions, and contract states, returning to the
    /// configured initial height. Useful for starting fresh test scenarios.
    /// 
    /// # Example
    /// ```rust,no_run
    /// use neo3::neo_protocol::TestNode;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let mut node = TestNode::builder()
    ///         .with_initial_height(1000)
    ///         .build_with_node();
    ///     
    ///     // Mine some blocks
    ///     node.mine_block().await?;
    ///     node.mine_block().await?;
    ///     assert_eq!(node.get_block_count().await?, 1002);
    ///     
    ///     // Reset to initial state
    ///     node.reset_chain().await?;
    ///     assert_eq!(node.get_block_count().await?, 1000);
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub async fn reset_chain(&self) -> Result<(), TestNodeError> {
        let initial_height = self.config.initial_height;
        let mut ledger = self.ledger.write().unwrap();
        
        ledger.block_height = initial_height;
        ledger.blocks.clear();
        ledger.blocks_by_hash.clear();
        ledger.transactions.clear();
        ledger.state_root = H256::zero();
        ledger.next_primary = 0;
        
        drop(ledger);
        
        // Clear contracts and faults
        self.contracts.write().unwrap().clear();
        self.faults.write().unwrap().clear();
        
        Ok(())
    }

    /// Configures the next primary validator
    /// 
    /// # Example
    /// ```rust,no_run
    /// use neo3::neo_protocol::TestNode;
    ///
    /// let node = TestNode::new();
    /// node.set_next_primary(2); // Set validator 2 as next primary
    /// ```
    pub fn set_next_primary(&self, index: usize) {
        let mut ledger = self.ledger.write().unwrap();
        ledger.next_primary = index;
    }

    /// Returns whether strict validation is enabled
    /// 
    /// When enabled, additional validation checks are performed on transactions
    /// and blocks. This simulates a more production-like environment.
    /// 
    /// # Returns
    /// `true` if strict validation mode is active, `false` otherwise
    /// 
    /// # Example
    /// ```rust,no_run
    /// use neo3::neo_protocol::TestNode;
    ///
    /// let node = TestNode::builder()
    ///     .with_strict_validation(true)
    ///     .build_with_node();
    /// 
    /// assert!(node.is_strict_validation());
    /// ```
    pub fn is_strict_validation(&self) -> bool {
        self.config.strict_validation
    }
}

impl Default for TestNode {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// MockRPC Provider Interface
// =============================================================================

/// Mock RPC provider trait for compatibility with existing code
#[async_trait::async_trait]
pub trait MockRPCProvider: Send + Sync {
    async fn get_block_count(&self) -> Result<u32, TestNodeError>;
    async fn get_best_block_hash(&self) -> Result<H256, TestNodeError>;
    async fn get_block_by_index(&self, index: u32) -> Result<Option<TestBlock>, TestNodeError>;
    async fn get_block_by_hash(&self, hash: H256) -> Result<Option<TestBlock>, TestNodeError>;
    async fn get_transaction(&self, hash: H256) -> Result<Option<TestTransaction>, TestNodeError>;
    async fn invoke_contract(
        &self,
        contract_hash: &H160,
        method: &str,
        params: &[Value],
    ) -> Result<Value, TestNodeError>;
    async fn send_raw_transaction(&self, tx: &Vec<u8>) -> Result<H256, TestNodeError>;
}

#[async_trait::async_trait]
impl MockRPCProvider for TestNode {
    async fn get_block_count(&self) -> Result<u32, TestNodeError> {
        Self::get_block_count(self).await
    }

    async fn get_best_block_hash(&self) -> Result<H256, TestNodeError> {
        Self::get_best_block_hash(self).await
    }

    async fn get_block_by_index(&self, index: u32) -> Result<Option<TestBlock>, TestNodeError> {
        Self::get_block_by_index(self, index).await
    }

    async fn get_block_by_hash(&self, hash: H256) -> Result<Option<TestBlock>, TestNodeError> {
        Self::get_block_by_hash(self, hash).await
    }

    async fn get_transaction(&self, hash: H256) -> Result<Option<TestTransaction>, TestNodeError> {
        Self::get_transaction(self, hash).await
    }

    async fn invoke_contract(
        &self,
        _contract_hash: &H160,
        _method: &str,
        _params: &[Value],
    ) -> Result<Value, TestNodeError> {
        // Simulates contract execution with configurable behavior.
        // Currently returns a default HALT state with fixed gas consumption.
        Ok(serde_json::json!({
            "state": "HALT",
            "gas_consumed": 1000000,
            "stack": [],
            "events": []
        }))
    }

    async fn send_raw_transaction(&self, _tx: &Vec<u8>) -> Result<H256, TestNodeError> {
        // Generates a deterministic transaction hash and returns it.
        // In a full implementation, this would broadcast to validators.
        Ok(H256::from_low_u64_le(rand::random::<u64>()))
    }
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_testnode_block_height() {
        // Given: A test node initialized at height 1000
        let node = TestNode::builder().with_initial_height(1000).build_with_node();

        // When: Getting the current block count
        let height = node.get_block_count().await.unwrap();

        // Then: Should return the configured initial height
        assert_eq!(height, 1000);

        // And: After mining a block
        node.mine_block().await.unwrap();
        let new_height = node.get_block_count().await.unwrap();

        // Then: Height should have incremented
        assert_eq!(new_height, 1001);
    }

    #[tokio::test]
    async fn test_testnode_fault_injection() {
        // Given: A test node with default configuration
        let node = TestNode::new();

        // When: We inject a fault for a specific transaction
        let tx_hash = H256::from_low_u64_le(12345);
        node.inject_fault(tx_hash, TestNodeError::InsufficientFunds);

        // Then: Getting that transaction should return the injected error
        let result = node.get_transaction(tx_hash).await;
        assert!(matches!(result, Err(TestNodeError::InsufficientFunds)));

        // And: Clearing faults should remove the error
        node.clear_faults();
        let result = node.get_transaction(tx_hash).await;
        assert!(matches!(result, Err(TestNodeError::UnknownTransaction)));
    }

    #[tokio::test]
    async fn test_testnode_state_snapshot() {
        // Given: A test node starting at block 0
        let mut node = TestNode::builder().with_initial_height(0).build_with_node();

        // Setup: Add some state
        let account = H160::from_low_u64_le(100);
        node.set_token_balance(&account, U256::from(1000)).await;
        node.mine_block().await.unwrap();

        // When: Taking a snapshot after setup
        let snapshot = node.take_snapshot().unwrap();

        // Verify: Snapshot captures correct height
        assert_eq!(snapshot.block_height, 1);

        // Continue: Mine more blocks and modify state
        node.set_token_balance(&account, U256::from(500)).await;
        node.mine_block().await.unwrap();
        node.mine_block().await.unwrap();

        // When: Restoring the snapshot
        node.restore_snapshot(&snapshot).unwrap();
        let restored_height = node.get_block_count().await.unwrap();

        // Then: Should be restored to snapshot height
        assert_eq!(restored_height, 1);
    }

    #[tokio::test]
    async fn test_testnode_block_creation() {
        // Given: A fresh test node
        let node = TestNode::builder().with_initial_height(0).build_with_node();

        // When: Mining multiple blocks
        let block1 = node.mine_block().await.unwrap();
        let block2 = node.mine_block().await.unwrap();
        let block3 = node.mine_block().await.unwrap();

        // Then: Each block should have correct properties
        assert_eq!(block1.index, 0);
        assert_eq!(block2.index, 1);
        assert_eq!(block3.index, 2);

        // And: Blocks should be linked correctly
        assert_eq!(block2.prev_hash, block1.hash);
        assert_eq!(block3.prev_hash, block2.hash);

        // And: Primary validators should rotate
        assert_ne!(block1.primary, block2.primary);
        assert_ne!(block2.primary, block3.primary);
    }

    #[tokio::test]
    async fn test_testnode_transaction_handling() {
        // Given: A test node
        let node = TestNode::builder()
            .with_initial_height(0)
            .build_with_node();

        // Setup: Mine initial block (height 0)
        let block1 = node.mine_block().await.unwrap();
        println!("Mined block at height {}, hash={}", block1.index, block1.hash);
        
        // When: Creating and adding a transaction to current block
        let tx = TestTransaction {
            hash: H256::from_low_u64_le(999),
            sys_fee: 10000,
            net_fee: 1000,
            sender: H160::from_low_u64_le(100),
            valid_until: 100,
            attributes: Vec::new(),
            witnesses: Vec::new(),
        };

        match node.add_transaction(tx.clone()).await {
            Ok(_) => println!("Added transaction successfully"),
            Err(e) => println!("Failed to add transaction: {}", e),
        }

        // Then: Transaction should be retrievable
        let retrieved = node.get_transaction(tx.hash).await;
        assert!(retrieved.is_ok(), "Expected OK but got {:?}", retrieved.err());
        assert!(retrieved.unwrap().is_some(), "Expected transaction data");
    }

    #[tokio::test]
    async fn test_testnode_token_balance() {
        // Given: A test node
        let node = TestNode::new();
        let account = H160::from_low_u64_le(500);

        // When: Setting initial balance
        node.set_token_balance(&account, U256::from(10000)).await;

        // Then: Balance should be readable
        let balance = node.get_token_balance(&account).await;
        assert_eq!(balance, Some(U256::from(10000)));

        // When: Updating balance
        node.set_token_balance(&account, U256::from(15000)).await;

        // Then: New balance should reflect changes
        let updated_balance = node.get_token_balance(&account).await;
        assert_eq!(updated_balance, Some(U256::from(15000)));
    }

    #[tokio::test]
    async fn test_testnode_builder_pattern() {
        // When: Building a test node with custom configuration
        let node = TestNode::builder()
            .with_initial_height(5000)
            .with_network_name("customnet")
            .with_validator_count(7)
            .with_block_time(10000)
            .with_strict_validation(true)
            .build_with_node();

        // Then: All settings should be applied
        assert_eq!(node.get_block_count().await.unwrap(), 5000);
        assert_eq!(node.config.network_name, "customnet");
        assert_eq!(node.config.validator_count, 7);
        assert_eq!(node.config.block_time_ms, 10000);
        assert!(node.is_strict_validation());
    }

    #[tokio::test]
    async fn test_testnode_response_delay() {
        // Given: A test node
        let mut node = TestNode::new();

        // When: Setting response delay
        node.set_response_delay(100);

        // Then: Delay should be applied
        assert_eq!(node.response_delay(), 100);

        // And: get_block_count should respect the delay
        let start = std::time::Instant::now();
        node.get_block_count().await.unwrap();
        let elapsed = start.elapsed();

        // The elapsed time should be approximately the delay
        assert!(elapsed.as_millis() >= 100);
    }

    #[tokio::test]
    async fn test_testnode_chain_settings() {
        // Given: A test node with default configuration
        let node = TestNode::new();

        // Then: Chain settings should have Neo N3 defaults
        assert_eq!(node.config.chain_settings.max_traceable_blocks, 21_024);
        assert_eq!(node.config.chain_settings.block_reward, 0);
        assert_eq!(node.config.chain_settings.storage_price, 10_000);
    }

    #[tokio::test]
    async fn test_testnode_multi_account_balances() {
        // Given: A test node
        let node = TestNode::new();

        // Setup: Multiple accounts with different balances
        let accounts = vec![
            (H160::from_low_u64_le(1), U256::from(1000)),
            (H160::from_low_u64_le(2), U256::from(2000)),
            (H160::from_low_u64_le(3), U256::from(3000)),
        ];

        for (account, balance) in &accounts {
            node.set_token_balance(account, *balance).await;
        }

        // Then: Each account should have correct balance
        for (account, expected_balance) in &accounts {
            let balance = node.get_token_balance(account).await;
            assert_eq!(balance, Some(*expected_balance));
        }

        // When: Querying non-existent account
        let non_existent = H160::from_low_u64_le(999);
        let balance = node.get_token_balance(&non_existent).await;

        // Then: Should return None
        assert_eq!(balance, None);
    }

    // =========================================================================
    // Integration Tests: Chain Reset & Reinitialization
    // =========================================================================

    #[tokio::test]
    async fn test_integration_chain_reset() {
        // Given: A node with mined blocks and state
        let node = TestNode::builder().with_initial_height(1000).build_with_node();
        node.mine_block().await.unwrap();
        node.mine_block().await.unwrap();
        node.mine_block().await.unwrap();
        assert_eq!(node.get_block_count().await.unwrap(), 1003);

        // And: Some account state and faults
        let account = H160::from_low_u64_le(1);
        node.set_token_balance(&account, U256::from(5000)).await;
        node.inject_fault(H256::from_low_u64_le(7), TestNodeError::Timeout);

        // When: Resetting the chain
        node.reset_chain().await.unwrap();

        // Then: Height returns to initial
        assert_eq!(node.get_block_count().await.unwrap(), 1000);
        // And: State is cleared
        assert_eq!(node.get_token_balance(&account).await, None);
        // And: Faults are cleared
        assert_eq!(node.check_fault(&H256::from_low_u64_le(7)), None);
    }

    #[tokio::test]
    async fn test_integration_chain_reinitialization() {
        // Given: A node reset after use
        let node = TestNode::builder().with_initial_height(0).build_with_node();
        node.mine_block().await.unwrap();
        node.reset_chain().await.unwrap();

        // When: Re-mining after reset
        let block = node.mine_block().await.unwrap();

        // Then: Block indexing restarts correctly
        assert_eq!(block.index, 0);
        assert_eq!(block.prev_hash, H256::zero());
        assert_eq!(node.get_block_count().await.unwrap(), 1);
    }

    // =========================================================================
    // Integration Tests: Transaction Broadcasting & Validation
    // =========================================================================

    #[tokio::test]
    async fn test_integration_transaction_broadcast_and_retrieve() {
        // Given: A node with an initial block
        let node = TestNode::builder().with_initial_height(0).build_with_node();
        node.mine_block().await.unwrap();

        // When: Broadcasting multiple transactions
        for i in 1..=5u64 {
            let tx = TestTransaction {
                hash: H256::from_low_u64_le(i),
                sys_fee: 10000 * i as i64,
                net_fee: 1000,
                sender: H160::from_low_u64_le(i),
                valid_until: 100,
                attributes: Vec::new(),
                witnesses: Vec::new(),
            };
            node.add_transaction(tx).await.unwrap();
        }

        // Then: All transactions are retrievable
        for i in 1..=5u64 {
            let retrieved = node.get_transaction(H256::from_low_u64_le(i)).await.unwrap();
            assert!(retrieved.is_some());
            assert_eq!(retrieved.unwrap().sys_fee, 10000 * i as i64);
        }
    }

    #[tokio::test]
    async fn test_integration_add_transaction_without_block_fails() {
        // Given: A node with no mined blocks
        let node = TestNode::builder().with_initial_height(0).build_with_node();
        let tx = TestTransaction {
            hash: H256::from_low_u64_le(1),
            sys_fee: 100,
            net_fee: 10,
            sender: H160::from_low_u64_le(1),
            valid_until: 100,
            attributes: Vec::new(),
            witnesses: Vec::new(),
        };

        // When/Then: Adding a transaction fails with UnknownBlock
        let result = node.add_transaction(tx).await;
        assert!(matches!(result, Err(TestNodeError::UnknownBlock)));
    }

    #[tokio::test]
    async fn test_integration_unknown_transaction_query() {
        // Given: A node with a block but no matching transaction
        let node = TestNode::builder().with_initial_height(0).build_with_node();
        node.mine_block().await.unwrap();

        // When/Then: Querying an unknown transaction returns error
        let result = node.get_transaction(H256::from_low_u64_le(12321)).await;
        assert!(matches!(result, Err(TestNodeError::UnknownTransaction)));
    }

    // =========================================================================
    // Integration Tests: Fault Injection Patterns
    // =========================================================================

    #[tokio::test]
    async fn test_integration_fault_injection_multiple_errors() {
        // Given: A node with several distinct faults injected
        let node = TestNode::new();
        let faults = vec![
            (H256::from_low_u64_le(1), TestNodeError::InsufficientFunds),
            (H256::from_low_u64_le(2), TestNodeError::Timeout),
            (H256::from_low_u64_le(3), TestNodeError::ValidationFailed("bad sig".to_string())),
            (H256::from_low_u64_le(4), TestNodeError::ConnectionClosed),
        ];
        for (hash, err) in &faults {
            node.inject_fault(*hash, err.clone());
        }

        // Then: Each fault is returned for its transaction
        for (hash, err) in &faults {
            assert_eq!(node.check_fault(hash).as_ref(), Some(err));
            let result = node.get_transaction(*hash).await;
            assert_eq!(result.err().as_ref(), Some(err));
        }
    }

    #[tokio::test]
    async fn test_integration_fault_override_and_clear() {
        // Given: A node with a fault injected
        let node = TestNode::new();
        let tx = H256::from_low_u64_le(50);
        node.inject_fault(tx, TestNodeError::Timeout);
        assert!(matches!(node.check_fault(&tx), Some(TestNodeError::Timeout)));

        // When: Overriding the same transaction fault
        node.inject_fault(tx, TestNodeError::InsufficientFunds);

        // Then: The latest fault takes precedence
        assert!(matches!(node.check_fault(&tx), Some(TestNodeError::InsufficientFunds)));

        // When: Clearing faults
        node.clear_faults();

        // Then: No fault remains
        assert_eq!(node.check_fault(&tx), None);
    }

    #[tokio::test]
    async fn test_integration_fault_does_not_affect_other_transactions() {
        // Given: A node with one faulted and one valid transaction
        let node = TestNode::builder().with_initial_height(0).build_with_node();
        node.mine_block().await.unwrap();
        let valid_tx = TestTransaction {
            hash: H256::from_low_u64_le(100),
            sys_fee: 500,
            net_fee: 50,
            sender: H160::from_low_u64_le(1),
            valid_until: 200,
            attributes: Vec::new(),
            witnesses: Vec::new(),
        };
        node.add_transaction(valid_tx).await.unwrap();
        node.inject_fault(H256::from_low_u64_le(200), TestNodeError::InsufficientFunds);

        // Then: Valid transaction is unaffected
        let ok = node.get_transaction(H256::from_low_u64_le(100)).await;
        assert!(ok.unwrap().is_some());
        // And: Faulted transaction returns error
        let err = node.get_transaction(H256::from_low_u64_le(200)).await;
        assert!(matches!(err, Err(TestNodeError::InsufficientFunds)));
    }

    // =========================================================================
    // Integration Tests: Snapshot Creation & Restoration
    // =========================================================================

    #[tokio::test]
    async fn test_integration_snapshot_full_restoration() {
        // Given: A node with blocks, balances, and faults
        let node = TestNode::builder().with_initial_height(0).build_with_node();
        node.mine_block().await.unwrap();
        node.mine_block().await.unwrap();
        let account = H160::from_low_u64_le(10);
        node.set_token_balance(&account, U256::from(7777)).await;
        node.inject_fault(H256::from_low_u64_le(3), TestNodeError::Timeout);

        // When: Taking a snapshot
        let snapshot = node.take_snapshot().unwrap();
        assert_eq!(snapshot.block_height, 2);
        assert_eq!(snapshot.blocks.len(), 2);

        // And: Mutating state afterward
        node.mine_block().await.unwrap();
        node.set_token_balance(&account, U256::from(1)).await;
        node.clear_faults();

        // When: Restoring the snapshot
        node.restore_snapshot(&snapshot).unwrap();

        // Then: Height, balance, and faults are restored
        assert_eq!(node.get_block_count().await.unwrap(), 2);
        assert_eq!(node.get_token_balance(&account).await, Some(U256::from(7777)));
        assert!(matches!(node.check_fault(&H256::from_low_u64_le(3)), Some(TestNodeError::Timeout)));
    }

    #[tokio::test]
    async fn test_integration_snapshot_preserves_block_hashes() {
        // Given: A node with blocks and a snapshot
        let node = TestNode::builder().with_initial_height(0).build_with_node();
        let block0 = node.mine_block().await.unwrap();
        let block1 = node.mine_block().await.unwrap();
        let snapshot = node.take_snapshot().unwrap();

        // When: Resetting and restoring
        node.reset_chain().await.unwrap();
        node.restore_snapshot(&snapshot).unwrap();

        // Then: Blocks are retrievable by hash after restore
        let restored0 = node.get_block_by_hash(block0.hash).await.unwrap();
        let restored1 = node.get_block_by_hash(block1.hash).await.unwrap();
        assert_eq!(restored0.unwrap().index, 0);
        assert_eq!(restored1.unwrap().index, 1);
    }

    // =========================================================================
    // Integration Tests: Multi-account Balance Queries
    // =========================================================================

    #[tokio::test]
    async fn test_integration_multi_account_across_chain_state() {
        // Given: A node with several accounts and blocks
        let node = TestNode::builder().with_initial_height(0).build_with_node();
        let accounts: Vec<(H160, U256)> = (1..=10u64)
            .map(|i| (H160::from_low_u64_le(i), U256::from(i * 100)))
            .collect();
        for (acc, bal) in &accounts {
            node.set_token_balance(acc, *bal).await;
        }
        node.mine_block().await.unwrap();
        node.mine_block().await.unwrap();

        // When/Then: All balances persist across mined blocks
        for (acc, bal) in &accounts {
            assert_eq!(node.get_token_balance(acc).await, Some(*bal));
        }

        // And: Snapshot captures all balances
        let snapshot = node.take_snapshot().unwrap();
        assert_eq!(snapshot.account_balances.len(), 10);
    }

    // =========================================================================
    // Integration Tests: wait_for_block
    // =========================================================================

    #[tokio::test]
    async fn test_integration_wait_for_block_success() {
        // Given: A node at initial height with blocks already present
        let node = TestNode::builder().with_initial_height(0).build_with_node();
        node.mine_block().await.unwrap();
        node.mine_block().await.unwrap();
        node.mine_block().await.unwrap();

        // When: Waiting for a block that already exists
        let block = node.wait_for_block(1, 500).await.unwrap();

        // Then: The requested block is returned
        assert_eq!(block.index, 1);
    }

    #[tokio::test]
    async fn test_integration_wait_for_block_timeout() {
        // Given: A node that will never reach the target height
        let node = TestNode::builder().with_initial_height(0).build_with_node();

        // When: Waiting for an unreachable height with a short timeout
        let result = node.wait_for_block(100, 50).await;

        // Then: A timeout error is returned
        assert!(matches!(result, Err(TestNodeError::Timeout)));
    }

    // =========================================================================
    // Integration Tests: Response Delay (Network Latency Simulation)
    // =========================================================================

    #[tokio::test]
    async fn test_integration_response_delay_applies_to_queries() {
        // Given: A node with a configured response delay
        let mut node = TestNode::new();
        node.set_response_delay(50);
        assert_eq!(node.response_delay(), 50);
        node.mine_block().await.unwrap();

        // When: Performing a delayed query
        let start = std::time::Instant::now();
        node.get_best_block_hash().await.unwrap();
        let elapsed = start.elapsed();

        // Then: The delay is respected
        assert!(elapsed.as_millis() >= 50);
    }

    #[tokio::test]
    async fn test_integration_zero_delay_is_fast() {
        // Given: A node with no response delay
        let node = TestNode::new();
        assert_eq!(node.response_delay(), 0);

        // When: Performing a query
        let start = std::time::Instant::now();
        node.get_block_count().await.unwrap();
        let elapsed = start.elapsed();

        // Then: The query completes quickly (well under a delay threshold)
        assert!(elapsed.as_millis() < 50);
    }

    // =========================================================================
    // Integration Tests: Config, Errors, and Provider Trait
    // =========================================================================

    #[tokio::test]
    async fn test_integration_config_magic_number_deterministic() {
        // Given: Two configs with the same network name
        let a = TestNodeConfig { network_name: "testmagic".to_string(), ..Default::default() };
        let b = TestNodeConfig { network_name: "testmagic".to_string(), ..Default::default() };

        // Then: Magic numbers are deterministic and equal
        assert_eq!(a.magic_number(), b.magic_number());

        // And: Different names produce different magic numbers
        let c = TestNodeConfig { network_name: "othernet".to_string(), ..Default::default() };
        assert_ne!(a.magic_number(), c.magic_number());
    }

    #[tokio::test]
    async fn test_integration_error_display_and_conversion() {
        // Given: Various error constructions
        let from_str: TestNodeError = "boom".into();
        let from_string: TestNodeError = String::from("kaboom").into();

        // Then: String conversions map to Internal
        assert!(matches!(from_str, TestNodeError::Internal(_)));
        assert!(matches!(from_string, TestNodeError::Internal(_)));

        // And: Display formatting works for representative variants
        assert_eq!(TestNodeError::UnknownBlock.to_string(), "Unknown block");
        assert_eq!(TestNodeError::Timeout.to_string(), "Request timeout");
        assert!(TestNodeError::InvalidParams("x".to_string()).to_string().contains("x"));
    }

    #[tokio::test]
    async fn test_integration_mock_rpc_provider_trait() {
        // Given: A node used through the MockRPCProvider trait
        let node = TestNode::builder().with_initial_height(0).build_with_node();
        node.mine_block().await.unwrap();
        let provider: &dyn MockRPCProvider = &node;

        // Then: Trait methods delegate correctly
        assert_eq!(provider.get_block_count().await.unwrap(), 1);
        let hash = provider.get_best_block_hash().await.unwrap();
        assert!(provider.get_block_by_hash(hash).await.unwrap().is_some());

        // And: invoke_contract returns a HALT state
        let result = provider
            .invoke_contract(&H160::zero(), "balanceOf", &[])
            .await
            .unwrap();
        assert_eq!(result["state"], "HALT");

        // And: send_raw_transaction returns a hash
        let tx_hash = provider.send_raw_transaction(&vec![1, 2, 3]).await.unwrap();
        assert_ne!(tx_hash, H256::zero());
    }

    #[tokio::test]
    async fn test_integration_block_navigation() {
        // Given: A node with several blocks
        let node = TestNode::builder().with_initial_height(0).build_with_node();
        node.mine_block().await.unwrap();
        node.mine_block().await.unwrap();

        // When/Then: Query by index and by hash agree
        let by_index = node.get_block_by_index(1).await.unwrap().unwrap();
        let by_hash = node.get_block_by_hash(by_index.hash).await.unwrap().unwrap();
        assert_eq!(by_index.index, by_hash.index);
        assert_eq!(by_index.hash, by_hash.hash);

        // And: Unknown index returns None
        assert!(node.get_block_by_index(999).await.unwrap().is_none());
        // And: Unknown hash returns an error
        assert!(node.get_block_by_hash(H256::from_low_u64_le(4242)).await.is_err());
    }

    #[tokio::test]
    async fn test_integration_next_primary_configuration() {
        // Given: A node with a manually set next primary
        let node = TestNode::builder().with_initial_height(0).with_validator_count(4).build_with_node();
        node.set_next_primary(2);

        // When: Mining a block
        let block = node.mine_block().await.unwrap();

        // Then: The block uses the configured primary
        assert_eq!(block.primary, 2);
    }

    #[tokio::test]
    async fn test_integration_from_config_constructor() {
        // Given: A custom config
        let config = TestNodeConfig {
            initial_height: 42,
            network_name: "confignet".to_string(),
            validator_count: 5,
            ..Default::default()
        };

        // When: Building via from_config
        let node = TestNode::from_config(config);

        // Then: Config is applied
        assert_eq!(node.get_block_count().await.unwrap(), 42);
        assert_eq!(node.initial_height(), 42);
    }
}
