use std::sync::Arc;
use std::time::Instant;
use alloy::primitives::Address;
use alloy::providers::Provider;
use alloy::transports::ws::WsConnect;
use futures::StreamExt;
use tracing::{info, warn, debug};

use crate::{SharedState, MempoolTx};

pub async fn start_listener(state: Arc<SharedState>) -> anyhow::Result<()> {
    let url = &state.config.ws_rpc;
    info!("Connecting to WebSocket: {}", url);

    loop {
        match alloy::providers::ProviderBuilder::new()
            .on_ws(WsConnect::new(url))
            .await
        {
            Ok(provider) => {
                info!("WebSocket connected. Subscribing to pending txns...");
                match provider.subscribe_pending_transactions().await {
                    Ok(sub) => {
                        let mut stream = sub.into_stream();
                        while let Some(tx) = stream.next().await {
                            if let Some(to) = tx.to {
                                let txn = MempoolTx {
                                    tx_hash: hex::encode(tx.hash.0),
                                    to,
                                    value: tx.value.to(),
                                    timestamp: Instant::now(),
                                };
                                state.mempool_txns.insert(txn.tx_hash.clone(), txn);
                            }
                        }
                    }
                    Err(e) => warn!("Subscription failed: {}", e),
                }
            }
            Err(e) => {
                warn!("WS connect failed (retrying): {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }
}
