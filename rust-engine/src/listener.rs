use std::sync::Arc;
use tokio::time::{timeout, Duration};
use futures::StreamExt;
use alloy::primitives::{Address, U256, Log};
use alloy::providers::Provider;
use alloy::transports::ws::WsConnect;
use tracing::{info, warn, error, debug};

use crate::{SharedState, MempoolTx, PoolState, Instant};

const UNISWAP_V3_POOL_ABI: &str = "Swap(address indexed sender, address indexed recipient, int256 amount0, int256 amount1, uint160 sqrtPriceX96, uint128 liquidity, int24 tick)";
const EVENT_TOPIC_SWAP_V3: &str =
    "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67";

const POLL_INTERVAL_MS: u64 = 50;
const CONNECT_TIMEOUT_SECS: u64 = 10;

pub async fn start_listener(state: Arc<SharedState>) -> anyhow::Result<()> {
    info!("Connecting to WebSocket RPC: {}", state.config.ws_rpc);

    let ws = WsConnect::new(&state.config.ws_rpc);
    let provider = match Provider::builder()
        .on_ws(ws)
        .build()
        .await
    {
        Ok(p) => p,
        Err(e) => {
            warn!("WebSocket connection failed (will poll HTTP): {}", e);
            return start_http_polling(state).await;
        }
    };

    info!("WebSocket connected. Subscribing to pending transactions...");

    loop {
        match provider.subscribe_full_pending_transactions().await {
            Ok(sub) => {
                let mut stream = sub.into_stream();
                info!("Subscribed to pending txns. Processing...");

                while let Some(tx) = stream.next().await {
                    state.metrics.events_received.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if let Some(to) = tx.to {
                        let tx_info = MempoolTx {
                            tx_hash: hex::encode(tx.hash.0),
                            to,
                            value: tx.value.to(),
                            gas_price: tx.gas_price.map(|g| g.to()).unwrap_or(0),
                            timestamp: Instant::now(),
                            data: tx.input.input.clone().unwrap_or_default().into(),
                            pool: None,
                        };
                        state.mempool_txns.insert(tx_info.tx_hash.clone(), tx_info);
                    }
                }
            }
            Err(e) => {
                warn!("Subscription failed, reconnecting: {}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

async fn start_http_polling(state: Arc<SharedState>) -> anyhow::Result<()> {
    info!("Starting HTTP polling fallback for mempool events");

    let http_provider = alloy::providers::ProviderBuilder::new()
        .on_http(state.config.http_rpc.parse()?)?;

    let mut last_block: u64 = 0;
    loop {
        tokio::time::sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;

        match http_provider.get_block_number().await {
            Ok(block_num) => {
                if block_num > last_block {
                    last_block = block_num;
                    debug!("New block detected: {}", block_num);
                }
            }
            Err(e) => debug!("Poll error: {}", e),
        }
    }
}

pub fn extract_pool_swap_data(log: &Log) -> Option<PoolState> {
    if log.topics().len() < 4 { return None; }

    let topic0 = log.topics()[0];
    let expected: alloy::primitives::FixedBytes<32> = EVENT_TOPIC_SWAP_V3.parse().ok()?;

    if topic0 != expected { return None; }

    Some(PoolState {
        address: log.address(),
        token0: Address::ZERO,
        token1: Address::ZERO,
        fee: 0,
        sqrt_price_x96: 0,
        tick: 0,
        liquidity: 0,
        last_updated: Instant::now(),
    })
}
