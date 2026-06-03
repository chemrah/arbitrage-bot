use std::sync::Arc;
use std::time::Instant;
use alloy::providers::Provider;
use alloy::transports::ws::WsConnect;
use futures::StreamExt;
use tracing::{info, warn};

use crate::{SharedState, MempoolTx};

pub async fn start_listener(state: Arc<SharedState>) -> anyhow::Result<()> {
    let url = &state.config.ws_rpc;
    info!("Connecting to WebSocket RPC: {}", url);

    loop {
        let ws = WsConnect::new(url);
        match alloy::providers::ProviderBuilder::new()
            .on_ws(ws)
            .await
        {
            Ok(provider) => {
                info!("WS connected. Subscribing to pending transactions...");
                match provider.subscribe_pending_transactions().await {
                    Ok(sub) => {
                        let mut stream = sub.into_stream();
                        while let Some(tx_hash) = stream.next().await {
                            if let Ok(Some(tx)) = provider.get_transaction_by_hash(tx_hash).await {
                                if let Some(to) = tx.to {
                                    let entry = MempoolTx {
                                        tx_hash: hex::encode(tx_hash.0),
                                        to,
                                        value: tx.value.to(),
                                        timestamp: Instant::now(),
                                    };
                                    state.mempool_txns.insert(entry.tx_hash.clone(), entry);
                                }
                            }
                        }
                    }
                    Err(e) => warn!("Subscription error: {}", e),
                }
            }
            Err(e) => {
                warn!("WS connect failed (retry in 2s): {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }
}
