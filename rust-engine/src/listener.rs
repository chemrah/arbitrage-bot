use std::sync::Arc;
use std::time::Instant;
use futures::{StreamExt, SinkExt};
use serde_json::json;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};

use crate::{SharedState, MempoolTx};

pub async fn start_listener(state: Arc<SharedState>) -> anyhow::Result<()> {
    let url = &state.config.ws_rpc;
    info!("Connecting to WebSocket RPC: {}", url);

    loop {
        match connect_async(url).await {
            Ok((ws, _)) => {
                info!("WS connected. Subscribing to newPendingTransactions...");
                let (mut write, mut read) = ws.split();

                let sub_req = json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "eth_subscribe",
                    "params": ["newPendingTransactions"]
                });
                let _ = write.send(Message::Text(sub_req.to_string().into())).await;

                while let Some(Ok(msg)) = read.next().await {
                    if let Message::Text(text) = msg {
                        if text.contains("\"result\"") && !text.contains("\"method\"") {
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                                if let Some(tx_hash) = v["params"]["result"].as_str().or_else(|| v["result"].as_str()) {
                                    let entry = MempoolTx {
                                        tx_hash: tx_hash.to_string(),
                                        to: alloy::primitives::Address::ZERO,
                                        value: 0,
                                        timestamp: Instant::now(),
                                    };
                                    state.mempool_txns.insert(entry.tx_hash.clone(), entry);
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                warn!("WS connect failed (retry 2s): {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }
}
