mod listener;
mod math;
mod simulator;
mod bundler;
mod cex_hedger;
mod solver;

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tokio::time::{self, Duration, Instant};
use clap::Parser;
use tracing::{info, warn, error, debug};
use tracing_subscriber::EnvFilter;
use dashmap::DashMap;
use alloy::primitives::Address;

#[derive(Parser, Debug)]
#[command(name = "arb-engine", about = "MEV Arbitrage Engine for Uniswap V3/V4")]
struct Args {
    #[arg(long, default_value = "ws://127.0.0.1:8546")]
    ws_rpc: String,

    #[arg(long, default_value = "http://127.0.0.1:8545")]
    http_rpc: String,

    #[arg(long, default_value = "0x0000000000000000000000000000000000000000")]
    executor_address: String,

    #[arg(long, default_value = "0x0000000000000000000000000000000000000000")]
    executor_private_key: String,

    #[arg(long, default_value = "35")]
    bribe_percent: u64,

    #[arg(long, default_value = "false")]
    auto_pilot: bool,

    #[arg(long)]
    flashbots_endpoint: Option<String>,

    #[arg(long)]
    beaver_build_endpoint: Option<String>,

    #[arg(long)]
    titan_endpoint: Option<String>,

    #[arg(long)]
    builder069_endpoint: Option<String>,

    #[arg(long)]
    binance_api_key: Option<String>,

    #[arg(long)]
    binance_secret: Option<String>,

    #[arg(long)]
    okx_api_key: Option<String>,

    #[arg(long)]
    okx_secret: Option<String>,

    #[arg(long)]
    okx_passphrase: Option<String>,
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub ws_rpc: String,
    pub http_rpc: String,
    pub executor_address: Address,
    pub executor_private_key: String,
    pub bribe_percent: u64,
    pub auto_pilot: bool,
    pub flashbots_endpoint: Option<String>,
    pub beaver_build_endpoint: Option<String>,
    pub titan_endpoint: Option<String>,
    pub builder069_endpoint: Option<String>,
    pub binance_api_key: Option<String>,
    pub binance_secret: Option<String>,
    pub okx_api_key: Option<String>,
    pub okx_secret: Option<String>,
    pub okx_passphrase: Option<String>,
}

pub struct SharedState {
    pub config: AppConfig,
    pub mempool_txns: DashMap<String, MempoolTx>,
    pub pool_states: DashMap<Address, PoolState>,
    pub pending_opportunities: RwLock<Vec<ArbOpportunity>>,
    pub cex_prices: RwLock<HashMap<String, f64>>,
    pub metrics: Arc<MetricsCollector>,
}

#[derive(Clone, Debug)]
pub struct MempoolTx {
    pub tx_hash: String,
    pub to: Address,
    pub value: u128,
    pub gas_price: u128,
    pub timestamp: Instant,
    pub data: Vec<u8>,
    pub pool: Option<Address>,
}

#[derive(Clone, Debug)]
pub struct PoolState {
    pub address: Address,
    pub token0: Address,
    pub token1: Address,
    pub fee: u32,
    pub sqrt_price_x96: u128,
    pub tick: i32,
    pub liquidity: u128,
    pub last_updated: Instant,
}

#[derive(Clone, Debug)]
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
    pub simulation_result: Option<SimulationResult>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ArbType {
    TriangularV3,
    CrossPoolV3V4,
    CexDex,
    JitLiquidity,
}

#[derive(Clone, Debug)]
pub struct SwapStep {
    pub pool: Address,
    pub zero_for_one: bool,
    pub amount: u128,
    pub sqrt_price_limit: u128,
}

#[derive(Clone, Debug)]
pub struct SimulationResult {
    pub profitable: bool,
    pub profit: u128,
    pub gas_used: u64,
    pub tip_amount: u128,
}

pub struct MetricsCollector {
    pub events_received: std::sync::atomic::AtomicU64,
    pub simulations_run: std::sync::atomic::AtomicU64,
    pub profitable_ops: std::sync::atomic::AtomicU64,
    pub bundles_submitted: std::sync::atomic::AtomicU64,
    pub bundles_included: std::sync::atomic::AtomicU64,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            events_received: std::sync::atomic::AtomicU64::new(0),
            simulations_run: std::sync::atomic::AtomicU64::new(0),
            profitable_ops: std::sync::atomic::AtomicU64::new(0),
            bundles_submitted: std::sync::atomic::AtomicU64::new(0),
            bundles_included: std::sync::atomic::AtomicU64::new(0),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env()
            .add_directive(tracing::Level::INFO.into()))
        .json()
        .init();

    let args = Args::parse();
    info!("Starting Arbitrage Engine");

    let config = AppConfig {
        ws_rpc: args.ws_rpc.clone(),
        http_rpc: args.http_rpc.clone(),
        executor_address: args.executor_address.parse()
            .unwrap_or_else(|_| Address::ZERO),
        executor_private_key: args.executor_private_key.clone(),
        bribe_percent: args.bribe_percent,
        auto_pilot: args.auto_pilot,
        flashbots_endpoint: args.flashbots_endpoint.clone(),
        beaver_build_endpoint: args.beaver_build_endpoint.clone(),
        titan_endpoint: args.titan_endpoint.clone(),
        builder069_endpoint: args.builder069_endpoint.clone(),
        binance_api_key: args.binance_api_key.clone(),
        binance_secret: args.binance_secret.clone(),
        okx_api_key: args.okx_api_key.clone(),
        okx_secret: args.okx_secret.clone(),
        okx_passphrase: args.okx_passphrase.clone(),
    };

    let state = Arc::new(SharedState {
        config,
        mempool_txns: DashMap::new(),
        pool_states: DashMap::new(),
        pending_opportunities: RwLock::new(Vec::with_capacity(1024)),
        cex_prices: RwLock::new(HashMap::new()),
        metrics: Arc::new(MetricsCollector::new()),
    });

    let listener_state = state.clone();
    let listener_handle = tokio::spawn(async move {
        if let Err(e) = listener::start_listener(listener_state).await {
            error!("Listener crashed: {}", e);
        }
    });

    let solver_state = state.clone();
    let solver_handle = tokio::spawn(async move {
        if let Err(e) = solver::start_solver(solver_state).await {
            error!("Solver crashed: {}", e);
        }
    });

    let cex_state = state.clone();
    let cex_handle = tokio::spawn(async move {
        if let Err(e) = cex_hedger::start_cex_hedger(cex_state).await {
            error!("CEX Hedger crashed: {}", e);
        }
    });

    let scan_state = state.clone();
    let scan_handle = tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_millis(200));
        loop {
            interval.tick().await;
            scan_state.metrics.events_received.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if scan_state.config.auto_pilot {
                if let Err(e) = bundler::auto_execute_opportunities(&scan_state).await {
                    debug!("Auto-execute cycle: {}", e);
                }
            }
        }
    });

    info!("Engine initialized. Listening for events...");

    tokio::select! {
        _ = listener_handle => warn!("Listener task exited"),
        _ = solver_handle => warn!("Solver task exited"),
        _ = cex_handle => warn!("CEX hedger task exited"),
        _ = scan_handle => warn!("Scan task exited"),
    }

    Ok(())
}
