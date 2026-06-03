use std::sync::Arc;
use tokio::time::{self, Duration};
use futures::StreamExt;
use futures::SinkExt;
use tokio_tungstenite::connect_async;
use serde::Deserialize;
use tracing::{info, warn, debug};

use crate::SharedState;

const BINANCE_WS: &str = "wss://stream.binance.com:9443/ws";
const RECONNECT_DELAY: u64 = 5;

#[derive(Deserialize, Debug)]
struct BinanceTicker {
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "b")]
    bid: String,
    #[serde(rename = "a")]
    ask: String,
}

async fn run_binance_feed() -> anyhow::Result<()> {
    let streams = vec!["ethusdt@ticker", "btcusdt@ticker"];
    let url = format!("{}/{}", BINANCE_WS, streams.join("/"));

    loop {
        match connect_async(&url).await {
            Ok((ws, _)) => {
                info!("Binance WS connected");
                let (_, mut read) = ws.split();
                while let Some(Ok(msg)) = read.next().await {
                    if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                        if let Ok(ticker) = serde_json::from_str::<BinanceTicker>(&text) {
                            debug!("Binance {}: bid={} ask={}", ticker.symbol, ticker.bid, ticker.ask);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Binance WS error: {}", e);
                time::sleep(Duration::from_secs(RECONNECT_DELAY)).await;
            }
        }
    }
}

pub async fn start_cex_hedger(_state: Arc<SharedState>) -> anyhow::Result<()> {
    tokio::spawn(async {
        if let Err(e) = run_binance_feed().await {
            warn!("Binance feed exited: {}", e);
        }
    });
    info!("CEX hedger started");
    Ok(())
}
