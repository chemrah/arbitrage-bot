use revm::db::{CacheDB, EmptyDB};
use revm::primitives::Address;
use tracing::debug;

use crate::{SharedState, ArbOpportunity, SwapStep};

pub struct Simulator {
    pub db: CacheDB<EmptyDB>,
}

impl Simulator {
    pub fn new() -> Self {
        Self {
            db: CacheDB::new(EmptyDB::new()),
        }
    }

    pub fn simulate_swap(
        &mut self,
        _pool: Address,
        _zero_for_one: bool,
        _amount: u128,
    ) -> Result<(), String> {
        // In production, use revm to execute the swap against a local fork.
        // For now, return success (placeholder for real EVM call).
        Ok(())
    }

    pub fn is_profitable(&self, opportunity: &ArbOpportunity) -> bool {
        let gas_price: u128 = 50_000_000_000; // 50 gwei
        let gas_estimate: u64 = 90_000 * opportunity.route.len() as u64;
        let gas_cost = (gas_estimate as u128) * gas_price;
        opportunity.estimated_profit > gas_cost && opportunity.estimated_profit > 1000
    }
}

pub fn scan_for_opportunities(state: &SharedState) -> Vec<ArbOpportunity> {
    let now = std::time::Instant::now();
    let mut opportunities = Vec::new();

    for entry in state.pool_states.iter() {
        let pool = entry.value();
        opportunities.push(ArbOpportunity {
            id: uuid::Uuid::new_v4().to_string(),
            arb_type: crate::ArbType::CexDex,
            pools: vec![pool.address],
            token_in: pool.token0,
            token_out: pool.token1,
            amount_in: 1_000_000_000_000_000_000u128,
            estimated_profit: 50_000_000_000_000_000u128,
            success_probability: 0.75,
            timestamp: now,
            route: vec![SwapStep {
                pool: pool.address,
                zero_for_one: true,
                amount: 1_000_000_000_000_000_000u128,
                sqrt_price_limit: crate::math::MIN_SQRT_RATIO + 1,
            }],
        });
    }

    opportunities
}
