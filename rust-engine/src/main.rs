mod listener;
mod math;
mod simulator;
mod bundler;
mod cex_hedger;
mod solver;

use std::sync::Arc;
use std::collections::HashMap;
use std::time::Instant;
use dashmap::DashMap;
use alloy::primitives::Address;
use clap::Parser;
use tracing::{info, error, debug};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "arb-engine", about = "MEV Arbitrage Engine")]
struct Args {
    #[arg(long, default_value = "ws://127.0.0.1:8546")]
    ws_rpc: String,
    #[arg(long, default_value = "http://127.0.0.1:8545")]
    http_rpc: String,
    #[arg(long, default_value = "0x0000000000000000000000000000000000000000")]
    executor_address: String,
    #[arg(long, default_value = "0x0000000000000000000000000000000000000000")]
    executor_private_key: String,
    #[arg(long, default_value_t = 35)]
    bribe_percent: u64,
    #[arg(long, default_value_t = false)]
    auto_pilot: bool,
}

#[derive(Clone)]
pub struct AppConfig {
    pub ws_rpc: String,
    pub http_rpc: String,
    pub executor_address: Address,
    pub executor_private_key: String,
    pub bribe_percent: u64,
    pub auto_pilot: bool,
}

pub struct SharedState {
    pub config: AppConfig,
    pub mempool_txns: DashMap<String, MempoolTx>,
    pub pool_states: DashMap<Address, PoolState>,
    pub pending_opportunities: tokio::sync::RwLock<Vec<ArbOpportunity>>,
    pub cex_prices: tokio::sync::RwLock<HashMap<String, f64>>,
}

#[derive(Clone)]
pub struct MempoolTx {
    pub tx_hash: String,
    pub to: Address,
    pub value: u128,
    pub timestamp: Instant,
}

#[derive(Clone)]
pub struct PoolState {
    pub address: Address,
    pub token0: Address,
    pub token1: Address,
    pub sqrt_price_x96: u128,
    pub tick: i32,
    pub liquidity: u128,
    pub last_updated: Instant,
}

#[derive(Clone)]
pub struct ArbOpportunity {
    pub id: String,
    pub arb_type: ArbType,
    pub pools: Vec<Address>,
    pub token_in: Address,
    pub token_out: Address,
    pub amount_in: u128,
    pub estimated_profit: u128,
    pub success_probability: f64,
    pub timestamp: Instant,
    pub route: Vec<SwapStep>,
}

#[derive(Clone, Debug)]
pub struct SwapStep {
    pub pool: Address,
    pub zero_for_one: bool,
    pub amount: u128,
    pub sqrt_price_limit: u128,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ArbType {
    TriangularV3,
    CrossPoolV3V4,
    CexDex,
    JitLiquidity,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    let args = Args::parse();
    info!("Starting Arbitrage Engine");

    let config = AppConfig {
        ws_rpc: args.ws_rpc,
        http_rpc: args.http_rpc,
        executor_address: args.executor_address.parse().unwrap_or(Address::ZERO),
        executor_private_key: args.executor_private_key,
        bribe_percent: args.bribe_percent,
        auto_pilot: args.auto_pilot,
    };

    let state = Arc::new(SharedState {
        config,
        mempool_txns: DashMap::new(),
        pool_states: DashMap::new(),
        pending_opportunities: tokio::sync::RwLock::new(Vec::with_capacity(1024)),
        cex_prices: tokio::sync::RwLock::new(HashMap::new()),
    });

    let s = state.clone();
    tokio::spawn(async move { listener::start_listener(s).await });

    let s = state.clone();
    tokio::spawn(async move { solver::start_solver(s).await });

    let s = state.clone();
    tokio::spawn(async move { cex_hedger::start_cex_hedger(s).await });

    info!("Engine initialized. Listening...");
    tokio::signal::ctrl_c().await?;
    info!("Shutting down.");
    Ok(())
}
