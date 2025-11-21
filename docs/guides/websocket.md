# NeoRust WebSocket Guide (v0.5.1)

Real-time blockchain events are available through `neo3::sdk::websocket`. The client handles reconnection and subscription tracking.

## Quickstart

```rust,no_run
use neo3::sdk::websocket::{SubscriptionType, WebSocketClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut ws = WebSocketClient::new("wss://testnet1.neo.org/ws").await?;
    ws.connect().await?;

    // Subscribe to new blocks
    let _handle = ws.subscribe(SubscriptionType::NewBlocks).await?;

    if let Some(mut rx) = ws.take_event_receiver() {
        while let Some((_sub, event)) = rx.recv().await {
            println!("Event: {:?}", event);
        }
    }
    Ok(())
}
```

## Tips
- Use secure endpoints in production (`wss://`).
- Run event processing in a dedicated task to avoid blocking.
- Re-subscribe after connection loss if you manage subscriptions manually.
