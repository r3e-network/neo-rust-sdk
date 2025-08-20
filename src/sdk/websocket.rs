//! WebSocket support for real-time blockchain updates
//! 
//! Provides WebSocket connectivity for subscribing to blockchain events,
//! monitoring transactions, and receiving real-time notifications.

use crate::neo_error::unified::{NeoError, ErrorRecovery};
use crate::neo_types::{ScriptHash, Address, Bytes};
use futures_util::{StreamExt, SinkExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::{RwLock, mpsc, oneshot};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tungstenite::protocol::Message;
use url::Url;

/// WebSocket subscription types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubscriptionType {
    /// Subscribe to new blocks
    NewBlocks,
    /// Subscribe to new transactions
    NewTransactions,
    /// Subscribe to specific transaction confirmations
    TransactionConfirmation(String),
    /// Subscribe to contract events
    ContractEvents(ScriptHash),
    /// Subscribe to address activity
    AddressActivity(Address),
    /// Subscribe to token transfers
    TokenTransfers {
        token: ScriptHash,
        address: Option<Address>,
    },
    /// Subscribe to execution results
    ExecutionResults,
    /// Subscribe to notification events
    Notifications {
        contract: Option<ScriptHash>,
        name: Option<String>,
    },
}

/// WebSocket event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventData {
    /// New block event
    NewBlock {
        height: u32,
        hash: String,
        timestamp: u64,
        transactions: Vec<String>,
    },
    /// New transaction event
    NewTransaction {
        hash: String,
        sender: String,
        size: u32,
        attributes: Vec<serde_json::Value>,
    },
    /// Transaction confirmation event
    TransactionConfirmed {
        hash: String,
        block_height: u32,
        confirmations: u32,
        vm_state: String,
    },
    /// Contract event
    ContractEvent {
        contract: String,
        event_name: String,
        state: Vec<serde_json::Value>,
    },
    /// Address activity event
    AddressActivity {
        address: String,
        transaction: String,
        action: String,
        amount: Option<String>,
    },
    /// Token transfer event
    TokenTransfer {
        from: String,
        to: String,
        amount: String,
        token: String,
        transaction: String,
    },
    /// Execution result event
    ExecutionResult {
        trigger: String,
        vm_state: String,
        gas_consumed: String,
        stack: Vec<serde_json::Value>,
        notifications: Vec<serde_json::Value>,
    },
    /// Notification event
    Notification {
        contract: String,
        event_name: String,
        state: serde_json::Value,
    },
}

/// WebSocket subscription handle
pub struct SubscriptionHandle {
    id: String,
    subscription_type: SubscriptionType,
    cancel_tx: oneshot::Sender<()>,
}

impl SubscriptionHandle {
    /// Get the subscription ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the subscription type
    pub fn subscription_type(&self) -> &SubscriptionType {
        &self.subscription_type
    }

    /// Cancel the subscription
    pub fn cancel(self) {
        let _ = self.cancel_tx.send(());
    }
}

/// WebSocket client for real-time blockchain updates
pub struct WebSocketClient {
    url: Url,
    ws_stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    subscriptions: Arc<RwLock<HashMap<String, SubscriptionType>>>,
    event_tx: mpsc::UnboundedSender<(SubscriptionType, EventData)>,
    event_rx: Option<mpsc::UnboundedReceiver<(SubscriptionType, EventData)>>,
    reconnect_interval: Duration,
    max_reconnect_attempts: u32,
}

impl WebSocketClient {
    /// Create a new WebSocket client
    pub async fn new(url: &str) -> Result<Self, NeoError> {
        let url = Url::parse(url).map_err(|e| NeoError::Network {
            message: format!("Invalid WebSocket URL: {}", e),
            source: None,
            recovery: ErrorRecovery::new()
                .suggest("Check the WebSocket URL format")
                .suggest("Ensure the URL starts with ws:// or wss://")
                .docs("https://docs.neo.org/docs/n3/develop/tool/sdk/websocket"),
        })?;

        let (event_tx, event_rx) = mpsc::unbounded_channel();

        Ok(Self {
            url,
            ws_stream: None,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            event_rx: Some(event_rx),
            reconnect_interval: Duration::from_secs(5),
            max_reconnect_attempts: 5,
        })
    }

    /// Connect to the WebSocket server
    pub async fn connect(&mut self) -> Result<(), NeoError> {
        let (ws_stream, _) = connect_async(&self.url).await.map_err(|e| NeoError::Network {
            message: format!("Failed to connect to WebSocket: {}", e),
            source: None,
            recovery: ErrorRecovery::new()
                .suggest("Check network connection")
                .suggest("Verify the WebSocket server is running")
                .suggest("Try a different WebSocket endpoint")
                .retryable(true)
                .retry_after(self.reconnect_interval),
        })?;

        self.ws_stream = Some(ws_stream);
        self.start_event_loop().await;

        Ok(())
    }

    /// Disconnect from the WebSocket server
    pub async fn disconnect(&mut self) -> Result<(), NeoError> {
        if let Some(mut ws) = self.ws_stream.take() {
            ws.close(None).await.map_err(|e| NeoError::Network {
                message: format!("Failed to close WebSocket connection: {}", e),
                source: None,
                recovery: ErrorRecovery::new()
                    .suggest("Connection may already be closed"),
            })?;
        }
        Ok(())
    }

    /// Subscribe to blockchain events
    pub async fn subscribe(
        &mut self,
        subscription_type: SubscriptionType,
    ) -> Result<SubscriptionHandle, NeoError> {
        // Ensure we're connected
        if self.ws_stream.is_none() {
            self.connect().await?;
        }

        let subscription_id = self.generate_subscription_id();
        
        // Send subscription request
        let request = self.create_subscription_request(&subscription_type, &subscription_id);
        self.send_message(request).await?;

        // Store subscription
        let mut subs = self.subscriptions.write().await;
        subs.insert(subscription_id.clone(), subscription_type.clone());

        // Create cancellation channel
        let (cancel_tx, _cancel_rx) = oneshot::channel();

        Ok(SubscriptionHandle {
            id: subscription_id,
            subscription_type,
            cancel_tx,
        })
    }

    /// Unsubscribe from blockchain events
    pub async fn unsubscribe(&mut self, handle: SubscriptionHandle) -> Result<(), NeoError> {
        // Remove from subscriptions
        let mut subs = self.subscriptions.write().await;
        subs.remove(&handle.id);

        // Send unsubscribe request
        let request = self.create_unsubscribe_request(&handle.id);
        self.send_message(request).await?;

        Ok(())
    }

    /// Get event receiver
    pub fn take_event_receiver(&mut self) -> Option<mpsc::UnboundedReceiver<(SubscriptionType, EventData)>> {
        self.event_rx.take()
    }

    /// Set reconnection parameters
    pub fn set_reconnect_params(&mut self, interval: Duration, max_attempts: u32) {
        self.reconnect_interval = interval;
        self.max_reconnect_attempts = max_attempts;
    }

    /// Start the event processing loop
    async fn start_event_loop(&mut self) {
        let ws_stream = match self.ws_stream.take() {
            Some(ws) => ws,
            None => return,
        };

        let subscriptions = self.subscriptions.clone();
        let event_tx = self.event_tx.clone();
        let reconnect_interval = self.reconnect_interval;
        let max_reconnect_attempts = self.max_reconnect_attempts;
        let url = self.url.clone();

        tokio::spawn(async move {
            let mut ws = ws_stream;
            let mut reconnect_attempts = 0;

            loop {
                match ws.next().await {
                    Some(Ok(msg)) => {
                        reconnect_attempts = 0; // Reset on successful message
                        if let Err(e) = Self::process_message(msg, &subscriptions, &event_tx).await {
                            eprintln!("Error processing WebSocket message: {}", e);
                        }
                    }
                    Some(Err(e)) => {
                        eprintln!("WebSocket error: {}", e);
                        
                        // Attempt reconnection
                        if reconnect_attempts < max_reconnect_attempts {
                            reconnect_attempts += 1;
                            eprintln!("Attempting reconnection ({}/{})", reconnect_attempts, max_reconnect_attempts);
                            
                            tokio::time::sleep(reconnect_interval).await;
                            
                            match connect_async(&url).await {
                                Ok((new_ws, _)) => {
                                    ws = new_ws;
                                    eprintln!("Reconnected successfully");
                                    
                                    // Resubscribe to all active subscriptions
                                    let subs = subscriptions.read().await;
                                    for (id, sub_type) in subs.iter() {
                                        let request = Self::create_subscription_request_static(sub_type, id);
                                        if let Err(e) = ws.send(Message::Text(request)).await {
                                            eprintln!("Failed to resubscribe {}: {}", id, e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Reconnection failed: {}", e);
                                }
                            }
                        } else {
                            eprintln!("Max reconnection attempts reached, stopping event loop");
                            break;
                        }
                    }
                    None => {
                        eprintln!("WebSocket connection closed");
                        break;
                    }
                }
            }
        });
    }

    /// Process incoming WebSocket message
    async fn process_message(
        msg: Message,
        subscriptions: &Arc<RwLock<HashMap<String, SubscriptionType>>>,
        event_tx: &mpsc::UnboundedSender<(SubscriptionType, EventData)>,
    ) -> Result<(), NeoError> {
        match msg {
            Message::Text(text) => {
                let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| NeoError::Network {
                    message: format!("Failed to parse WebSocket message: {}", e),
                    source: None,
                    recovery: ErrorRecovery::new(),
                })?;

                // Parse event and subscription ID
                if let Some(event_data) = Self::parse_event(&json).await? {
                    if let Some(sub_id) = json.get("subscription").and_then(|s| s.as_str()) {
                        let subs = subscriptions.read().await;
                        if let Some(sub_type) = subs.get(sub_id) {
                            let _ = event_tx.send((sub_type.clone(), event_data));
                        }
                    }
                }
            }
            Message::Binary(_) => {
                // Handle binary messages if needed
            }
            Message::Ping(data) => {
                // Auto-handled by tungstenite
            }
            Message::Pong(_) => {
                // Auto-handled by tungstenite
            }
            Message::Close(_) => {
                // Connection closing
            }
            Message::Frame(_) => {
                // Raw frame, rarely used
            }
        }

        Ok(())
    }

    /// Parse event from JSON
    async fn parse_event(json: &serde_json::Value) -> Result<Option<EventData>, NeoError> {
        let event_type = json.get("type").and_then(|t| t.as_str()).unwrap_or("");

        let event_data = match event_type {
            "block_added" => Some(EventData::NewBlock {
                height: json.get("height").and_then(|h| h.as_u64()).unwrap_or(0) as u32,
                hash: json.get("hash").and_then(|h| h.as_str()).unwrap_or("").to_string(),
                timestamp: json.get("timestamp").and_then(|t| t.as_u64()).unwrap_or(0),
                transactions: json.get("transactions")
                    .and_then(|t| t.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default(),
            }),
            "transaction_added" => Some(EventData::NewTransaction {
                hash: json.get("hash").and_then(|h| h.as_str()).unwrap_or("").to_string(),
                sender: json.get("sender").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                size: json.get("size").and_then(|s| s.as_u64()).unwrap_or(0) as u32,
                attributes: json.get("attributes").and_then(|a| a.as_array()).cloned().unwrap_or_default(),
            }),
            "transaction_confirmed" => Some(EventData::TransactionConfirmed {
                hash: json.get("hash").and_then(|h| h.as_str()).unwrap_or("").to_string(),
                block_height: json.get("block_height").and_then(|h| h.as_u64()).unwrap_or(0) as u32,
                confirmations: json.get("confirmations").and_then(|c| c.as_u64()).unwrap_or(0) as u32,
                vm_state: json.get("vm_state").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            }),
            "notification" => Some(EventData::Notification {
                contract: json.get("contract").and_then(|c| c.as_str()).unwrap_or("").to_string(),
                event_name: json.get("event_name").and_then(|e| e.as_str()).unwrap_or("").to_string(),
                state: json.get("state").cloned().unwrap_or(serde_json::Value::Null),
            }),
            _ => None,
        };

        Ok(event_data)
    }

    /// Send a message through the WebSocket
    async fn send_message(&mut self, message: String) -> Result<(), NeoError> {
        if let Some(ref mut ws) = self.ws_stream {
            ws.send(Message::Text(message)).await.map_err(|e| NeoError::Network {
                message: format!("Failed to send WebSocket message: {}", e),
                source: None,
                recovery: ErrorRecovery::new()
                    .suggest("Check WebSocket connection")
                    .retryable(true),
            })?;
        } else {
            return Err(NeoError::Network {
                message: "WebSocket not connected".to_string(),
                source: None,
                recovery: ErrorRecovery::new()
                    .suggest("Call connect() before sending messages"),
            });
        }

        Ok(())
    }

    /// Generate a unique subscription ID
    fn generate_subscription_id(&self) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        format!("sub_{:016x}", rng.gen::<u64>())
    }

    /// Create subscription request message
    fn create_subscription_request(&self, sub_type: &SubscriptionType, id: &str) -> String {
        Self::create_subscription_request_static(sub_type, id)
    }

    /// Static version of create_subscription_request for use in spawned tasks
    fn create_subscription_request_static(sub_type: &SubscriptionType, id: &str) -> String {
        let method = match sub_type {
            SubscriptionType::NewBlocks => "subscribe_blocks",
            SubscriptionType::NewTransactions => "subscribe_transactions",
            SubscriptionType::TransactionConfirmation(_) => "subscribe_tx_confirmation",
            SubscriptionType::ContractEvents(_) => "subscribe_contract_events",
            SubscriptionType::AddressActivity(_) => "subscribe_address_activity",
            SubscriptionType::TokenTransfers { .. } => "subscribe_token_transfers",
            SubscriptionType::ExecutionResults => "subscribe_execution_results",
            SubscriptionType::Notifications { .. } => "subscribe_notifications",
        };

        let params = match sub_type {
            SubscriptionType::TransactionConfirmation(hash) => {
                serde_json::json!([hash])
            }
            SubscriptionType::ContractEvents(contract) => {
                serde_json::json!([contract.to_string()])
            }
            SubscriptionType::AddressActivity(address) => {
                serde_json::json!([address.to_string()])
            }
            SubscriptionType::TokenTransfers { token, address } => {
                if let Some(addr) = address {
                    serde_json::json!([token.to_string(), addr.to_string()])
                } else {
                    serde_json::json!([token.to_string()])
                }
            }
            SubscriptionType::Notifications { contract, name } => {
                let mut params = vec![];
                if let Some(c) = contract {
                    params.push(serde_json::json!(c.to_string()));
                }
                if let Some(n) = name {
                    params.push(serde_json::json!(n));
                }
                serde_json::json!(params)
            }
            _ => serde_json::json!([]),
        };

        serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": id,
        }).to_string()
    }

    /// Create unsubscribe request message
    fn create_unsubscribe_request(&self, id: &str) -> String {
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "unsubscribe",
            "params": [id],
            "id": format!("unsub_{}", id),
        }).to_string()
    }
}

/// Builder for WebSocket client configuration
pub struct WebSocketClientBuilder {
    url: String,
    reconnect_interval: Duration,
    max_reconnect_attempts: u32,
}

impl WebSocketClientBuilder {
    /// Create a new builder with URL
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            reconnect_interval: Duration::from_secs(5),
            max_reconnect_attempts: 5,
        }
    }

    /// Set reconnection interval
    pub fn reconnect_interval(mut self, interval: Duration) -> Self {
        self.reconnect_interval = interval;
        self
    }

    /// Set maximum reconnection attempts
    pub fn max_reconnect_attempts(mut self, attempts: u32) -> Self {
        self.max_reconnect_attempts = attempts;
        self
    }

    /// Build the WebSocket client
    pub async fn build(self) -> Result<WebSocketClient, NeoError> {
        let mut client = WebSocketClient::new(&self.url).await?;
        client.set_reconnect_params(self.reconnect_interval, self.max_reconnect_attempts);
        Ok(client)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_websocket_client_creation() {
        let result = WebSocketClient::new("ws://localhost:10332/ws").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_websocket_builder() {
        let result = WebSocketClientBuilder::new("ws://localhost:10332/ws")
            .reconnect_interval(Duration::from_secs(10))
            .max_reconnect_attempts(3)
            .build()
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_subscription_id_generation() {
        let client = WebSocketClient::new("ws://localhost:10332/ws").await.unwrap();
        let id1 = client.generate_subscription_id();
        let id2 = client.generate_subscription_id();
        assert_ne!(id1, id2);
        assert!(id1.starts_with("sub_"));
        assert!(id2.starts_with("sub_"));
    }
}