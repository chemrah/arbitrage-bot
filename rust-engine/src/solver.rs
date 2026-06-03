use std::sync::Arc;
use std::time::Instant;
use tokio::time::{self, Duration};
use tracing::{info, debug};

use crate::{SharedState, ArbOpportunity, ArbType, SwapStep};

pub async fn start_solver(state: Arc<SharedState>) -> anyhow::Result<()> {
    let mut interval = time::interval(Duration::from_secs(5));
    info!("Solver started");

    loop {
        interval.tick().await;
        debug!("Solver scanning...");

        for entry in state.pool_states.iter() {
            let pool = entry.value();
            let opp = ArbOpportunity {
                id: uuid::Uuid::new_v4().to_string(),
                arb_type: ArbType::CexDex,
                pools: vec![pool.address],
                token_in: pool.token0,
                token_out: pool.token1,
                amount_in: 1_000_000_000_000_000_000u128,
                estimated_profit: 50_000_000_000_000_000u128,
                success_probability: 0.75,
                timestamp: Instant::now(),
                route: vec![SwapStep {
                    pool: pool.address,
                    zero_for_one: true,
                    amount: 1_000_000_000_000_000_000u128,
                    sqrt_price_limit: crate::math::MIN_SQRT_RATIO + 1,
                }],
            };
            let mut pending = state.pending_opportunities.write().await;
            if pending.len() < 1024 {
                pending.push(opp);
            }
        }
    }
}
