use std::sync::Arc;
use tokio::time::{self, Duration};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{info, warn, debug, error};

use std::time::Instant;
use crate::{
    SharedState, ArbOpportunity, ArbType, SwapStep, PoolState,
};

const COWSWAP_API_BASE: &str = "https://api.cow.fi/mainnet/api/v1";
const UNISWAPX_API_BASE: &str = "https://api.uniswap.org/v2";
const SOLVER_INTERVAL_SECS: u64 = 2;
const ORDER_TTL_SECS: u64 = 60;

#[derive(Debug, Serialize, Deserialize)]
struct CowSwapOrder {
    #[serde(rename = "uid")]
    uid: String,
    #[serde(rename = "sellToken")]
    sell_token: String,
    #[serde(rename = "buyToken")]
    buy_token: String,
    #[serde(rename = "sellAmount")]
    sell_amount: String,
    #[serde(rename = "buyAmount")]
    buy_amount: String,
    #[serde(rename = "validTo")]
    valid_to: u64,
    #[serde(rename = "partiallyFillable")]
    partially_fillable: bool,
    #[serde(rename = "signature")]
    signature: Option<String>,
    owner: String,
    #[serde(rename = "receiver")]
    receiver: Option<String>,
    #[serde(rename = "appData")]
    app_data: String,
    #[serde(rename = "feeAmount")]
    fee_amount: String,
    #[serde(rename = "kind")]
    kind: String,
    #[serde(rename = "sellTokenBalance")]
    sell_token_balance: String,
    #[serde(rename = "buyTokenBalance")]
    buy_token_balance: String,
    #[serde(rename = "created")]
    created: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CowSwapApiResponse {
    orders: Option<Vec<CowSwapOrder>>,
    #[serde(rename = "order")]
    order: Option<CowSwapOrder>,
    #[serde(flatten)]
    extra: std::collections::HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct UniswapXOrder {
    #[serde(rename = "orderHash")]
    order_hash: String,
    #[serde(rename = "input")]
    input: UniswapXToken,
    #[serde(rename = "outputs")]
    outputs: Vec<UniswapXToken>,
    #[serde(rename = "signature")]
    signature: Option<String>,
    #[serde(rename = "nonce")]
    nonce: String,
    #[serde(rename = "deadline")]
    deadline: u64,
    #[serde(rename = "reactor")]
    reactor: String,
    #[serde(rename = "swapper")]
    swapper: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct UniswapXToken {
    token: String,
    amount: String,
    #[serde(rename = "recipient")]
    recipient: Option<String>,
}

pub struct SolverEngine {
    state: Arc<SharedState>,
    http_client: Client,
}

impl SolverEngine {
    pub fn new(state: Arc<SharedState>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("arb-engine/0.1")
            .build()
            .expect("Failed to build HTTP client");

        Self {
            state,
            http_client: client,
        }
    }

    pub async fn poll_cowswap_orders(&self) -> anyhow::Result<Vec<CowSwapOrder>> {
        let url = format!("{}/orders?limit=50&offset=0", COWSWAP_API_BASE);
        let resp = self.http_client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let text = resp.text().await?;
        let api_resp: CowSwapApiResponse = serde_json::from_str(&text)
            .unwrap_or(CowSwapApiResponse {
                orders: None,
                order: None,
                extra: std::collections::HashMap::new(),
            });

        Ok(api_resp.orders.unwrap_or_default())
    }

    pub async fn poll_uniswapx_orders(&self) -> anyhow::Result<Vec<UniswapXOrder>> {
        let url = format!("{}/orders", UNISWAPX_API_BASE);
        let resp = self.http_client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let orders: Vec<UniswapXOrder> = resp.json().await.unwrap_or_default();
        Ok(orders)
    }

    pub fn match_order_to_liquidity(
        &self,
        sell_token: &str,
        buy_token: &str,
        sell_amount: u128,
    ) -> Option<ArbOpportunity> {
        let now = Instant::now();

        for pool_entry in self.state.pool_states.iter() {
            let pool = pool_entry.value();
            let pool_sell = format!("{:?}", pool.token0).to_lowercase();
            let pool_buy = format!("{:?}", pool.token1).to_lowercase();

            let order_sell = sell_token.to_lowercase();
            let order_buy = buy_token.to_lowercase();

            if (pool_sell.contains(&order_sell) || pool_sell.contains(&order_buy))
                && (pool_buy.contains(&order_sell) || pool_buy.contains(&order_buy))
            {
                return Some(ArbOpportunity {
                    id: uuid::Uuid::new_v4().to_string(),
                    arb_type: ArbType::CexDex,
                    pools: vec![pool.address],
                    token_in: pool.token0,
                    token_out: pool.token1,
                    amount_in: sell_amount,
                    estimated_profit: sell_amount / 1000,
                    success_probability: 0.6,
                    timestamp: now,
                    route: vec![SwapStep {
                        pool: pool.address,
                        zero_for_one: true,
                        amount: sell_amount,
                        sqrt_price_limit: 4295128740,
                    }],
                    simulation_result: None,
                });
            }
        }

        None
    }

    pub async fn solve_intents(&self) -> anyhow::Result<Vec<ArbOpportunity>> {
        let mut opportunities = Vec::new();

        let cow_orders = self.poll_cowswap_orders().await.unwrap_or_default();
        for order in &cow_orders {
            let sell_amount: u128 = order.sell_amount.parse().unwrap_or(0);
            if let Some(opp) = self.match_order_to_liquidity(
                &order.sell_token,
                &order.buy_token,
                sell_amount,
            ) {
                opportunities.push(opp);
            }
        }

        let uniswapx_orders = self.poll_uniswapx_orders().await.unwrap_or_default();
        for order in &uniswapx_orders {
            let sell_amount: u128 = order.input.amount.parse().unwrap_or(0);
            if let Some(opp) = self.match_order_to_liquidity(
                &order.input.token,
                &order.outputs.first().map(|o| o.token.clone()).unwrap_or_default(),
                sell_amount,
            ) {
                opportunities.push(opp);
            }
        }

        Ok(opportunities)
    }
}

pub async fn start_solver(state: Arc<SharedState>) -> anyhow::Result<()> {
    let solver = SolverEngine::new(state.clone());
    let mut interval = time::interval(Duration::from_secs(SOLVER_INTERVAL_SECS));

    info!("Solver engine started. Polling CoW Swap & UniswapX intents...");

    loop {
        interval.tick().await;

        match solver.solve_intents().await {
            Ok(opportunities) => {
                for opp in opportunities {
                    debug!("Intent-matched opportunity: {} | profit={}",
                        opp.id, opp.estimated_profit);

                    let mut pending = state.pending_opportunities.write().await;
                    if pending.len() < 1024 {
                        pending.push(opp);
                    }
                }
            }
            Err(e) => {
                debug!("Solver cycle error: {}", e);
            }
        }
    }
}
