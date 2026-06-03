use std::sync::Arc;
use std::collections::HashMap;
use tokio::time::{self, Duration};
use tokio_tungstenite::connect_async;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{info, warn, debug, error};

use crate::SharedState;

const BINANCE_WS_BASE: &str = "wss://stream.binance.com:9443/ws";
const OKX_WS_BASE: &str = "wss://ws.okx.com:8443/ws/v5/public";
const RECONNECT_DELAY_SECS: u64 = 5;
const PRICE_CACHE_TTL_SECS: u64 = 60;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CexPrice {
    pub symbol: String,
    pub bid: f64,
    pub ask: f64,
    pub timestamp: u64,
    pub exchange: CexExchange,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CexExchange {
    Binance,
    OKX,
}

#[derive(Debug, Serialize, Deserialize)]
struct BinanceTicker {
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "b")]
    bid: String,
    #[serde(rename = "a")]
    ask: String,
    #[serde(rename = "E")]
    event_time: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct OkxTicker {
    arg: OkxArg,
    data: Vec<OkxTickerData>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OkxArg {
    channel: String,
    inst_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OkxTickerData {
    #[serde(rename = "instId")]
    inst_id: String,
    #[serde(rename = "bidPx")]
    bid_px: String,
    #[serde(rename = "askPx")]
    ask_px: String,
    #[serde(rename = "ts")]
    timestamp: String,
}

pub struct CexHedger {
    pub state: Arc<SharedState>,
    pub binance_subscriptions: Vec<String>,
    pub okx_subscriptions: Vec<String>,
}

impl CexHedger {
    pub fn new(state: Arc<SharedState>) -> Self {
        Self {
            state,
            binance_subscriptions: vec![
                "ethusdt@ticker".to_string(),
                "btcusdt@ticker".to_string(),
                "linkusdt@ticker".to_string(),
                "uniusdt@ticker".to_string(),
                "aaveusdt@ticker".to_string(),
            ],
            okx_subscriptions: vec![
                "ETH-USDT".to_string(),
                "BTC-USDT".to_string(),
                "LINK-USDT".to_string(),
                "UNI-USDT".to_string(),
                "AAVE-USDT".to_string(),
            ],
        }
    }

    pub async fn run_binance_stream(&self) -> anyhow::Result<()> {
        let streams = self.binance_subscriptions.join("/");
        let url = format!("{}/{}", BINANCE_WS_BASE, streams);

        loop {
            match connect_async(&url).await {
                Ok((ws, _response)) => {
                    info!("Connected to Binance WebSocket stream");
                    let (_, mut read) = ws.split();

                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                                if let Ok(ticker) = serde_json::from_str::<BinanceTicker>(&text) {
                                    self.handle_binance_ticker(ticker);
                                }
                            }
                            Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => {
                                warn!("Binance WS closed, reconnecting...");
                                break;
                            }
                            Err(e) => {
                                warn!("Binance WS error: {}, reconnecting...", e);
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    warn!("Binance WS connection failed: {}", e);
                    time::sleep(Duration::from_secs(RECONNECT_DELAY_SECS)).await;
                }
            }
        }
    }

    pub async fn run_okx_stream(&self) -> anyhow::Result<()> {
        loop {
            match connect_async(OKX_WS_BASE).await {
                Ok((ws, _response)) => {
                    info!("Connected to OKX WebSocket stream");
                    let (mut write, mut read) = ws.split();

                    let subscribe_msg = serde_json::json!({
                        "op": "subscribe",
                        "args": self.okx_subscriptions.iter().map(|s| {
                            serde_json::json!({
                                "channel": "tickers",
                                "instId": s
                            })
                        }).collect::<Vec<_>>()
                    });

                    if let Ok(msg) = serde_json::to_string(&subscribe_msg) {
                        let _ = write.send(tokio_tungstenite::tungstenite::Message::Text(msg.into())).await;
                    }

                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                                if let Ok(ticker) = serde_json::from_str::<OkxTicker>(&text) {
                                    self.handle_okx_ticker(ticker);
                                }
                            }
                            Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => {
                                warn!("OKX WS closed, reconnecting...");
                                break;
                            }
                            Err(e) => {
                                warn!("OKX WS error: {}, reconnecting...", e);
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    warn!("OKX WS connection failed: {}", e);
                    time::sleep(Duration::from_secs(RECONNECT_DELAY_SECS)).await;
                }
            }
        }
    }

    fn handle_binance_ticker(&self, ticker: BinanceTicker) {
        let symbol = ticker.symbol.trim_end_matches("@ticker").to_string();
        let mid_price = (ticker.bid.parse::<f64>().unwrap_or(0.0)
            + ticker.ask.parse::<f64>().unwrap_or(0.0)) / 2.0;

        let mut prices = self.state.cex_prices.blocking_write();
        prices.insert(format!("BINANCE:{}", symbol), mid_price);

        debug!("Binance {}: bid={} ask={} mid={}", symbol, ticker.bid, ticker.ask, mid_price);
    }

    fn handle_okx_ticker(&self, ticker: OkxTicker) {
        if let Some(data) = ticker.data.first() {
            let mid_price = (data.bid_px.parse::<f64>().unwrap_or(0.0)
                + data.ask_px.parse::<f64>().unwrap_or(0.0)) / 2.0;

            let mut prices = self.state.cex_prices.blocking_write();
            prices.insert(format!("OKX:{}", data.inst_id), mid_price);

            debug!("OKX {}: bid={} ask={} mid={}", data.inst_id, data.bid_px, data.ask_px, mid_price);
        }
    }

    pub async fn submit_delta_hedge_order(
        &self,
        symbol: &str,
        side: &str,
        quantity: f64,
        exchange: CexExchange,
    ) -> anyhow::Result<()> {
        match exchange {
            CexExchange::Binance => {
                let api_key = self.state.config.binance_api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Binance API key not configured"))?;
                let secret = self.state.config.binance_secret
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Binance secret not configured"))?;

                debug!("Submitting Binance {} order: {} {} (key={})", side, quantity, symbol, api_key);
            }
            CexExchange::OKX => {
                let api_key = self.state.config.okx_api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("OKX API key not configured"))?;
                let secret = self.state.config.okx_secret
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("OKX secret not configured"))?;

                debug!("Submitting OKX {} order: {} {} (key={})", side, quantity, symbol, api_key);
            }
        }

        Ok(())
    }

    pub fn calculate_hedge_quantity(
        dex_amount: f64,
        dex_price: f64,
        cex_price: f64,
    ) -> f64 {
        let notional = dex_amount * dex_price;
        notional / cex_price
    }
}

pub async fn start_cex_hedger(state: Arc<SharedState>) -> anyhow::Result<()> {
    let config = state.config.clone();

    let binance_handle = tokio::spawn(async move {
        run_binance_feed().await
    });

    let okx_handle = tokio::spawn(async move {
        run_okx_feed().await
    });

    tokio::try_join!(binance_handle, okx_handle)?;
    Ok(())
}

async fn run_binance_feed() -> anyhow::Result<()> {
    let streams = vec![
        "ethusdt@ticker", "btcusdt@ticker", "linkusdt@ticker",
    ];
    let url = format!("{}/{}", BINANCE_WS_BASE, streams.join("/"));

    loop {
        match connect_async(&url).await {
            Ok((ws, _)) => {
                info!("Connected to Binance WS");
                let (_, mut read) = ws.split();
                while let Some(msg) = read.next().await {
                    if let Ok(tokio_tungstenite::tungstenite::Message::Text(text)) = msg {
                        if let Ok(ticker) = serde_json::from_str::<BinanceTicker>(&text) {
                            debug!("Binance {}: b={} a={}", ticker.symbol, ticker.bid, ticker.ask);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Binance WS error: {}", e);
                time::sleep(Duration::from_secs(RECONNECT_DELAY_SECS)).await;
            }
        }
    }
}

async fn run_okx_feed() -> anyhow::Result<()> {
    let subscriptions = vec!["ETH-USDT", "BTC-USDT", "LINK-USDT"];

    loop {
        match connect_async(OKX_WS_BASE).await {
            Ok((ws, _)) => {
                info!("Connected to OKX WS");
                let (mut write, mut read) = ws.split();

                let subscribe_msg = serde_json::json!({
                    "op": "subscribe",
                    "args": subscriptions.iter().map(|s| {
                        serde_json::json!({"channel": "tickers", "instId": s})
                    }).collect::<Vec<_>>()
                });

                let _ = write.send(
                    tokio_tungstenite::tungstenite::Message::Text(
                        serde_json::to_string(&subscribe_msg)?.into()
                    )
                ).await;

                while let Some(msg) = read.next().await {
                    if let Ok(tokio_tungstenite::tungstenite::Message::Text(text)) = msg {
                        debug!("OKX data: {}", &text[..text.len().min(200)]);
                    }
                }
            }
            Err(e) => {
                warn!("OKX WS error: {}", e);
                time::sleep(Duration::from_secs(RECONNECT_DELAY_SECS)).await;
            }
        }
    }
}
