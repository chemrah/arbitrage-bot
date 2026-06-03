use std::sync::Arc;
use std::collections::BTreeMap;
use revm::{
    db::{CacheDB, EmptyDB},
    primitives::{
        AccountInfo, Address, Bytes, Bytecode,
        TxKind, U256 as RevmU256, SpecId,
    },
    Evm,
};
use tracing::{info, debug, warn};

use crate::{
    SharedState, ArbOpportunity, SwapStep, SimulationResult, ArbType,
    math,
};

pub struct Simulator {
    pub evm: Evm<CacheDB<EmptyDB>>,
}

impl Simulator {
    pub fn new() -> Self {
        let db = CacheDB::new(EmptyDB::new());
        let evm = Evm::builder()
            .with_db(db)
            .with_spec_id(SpecId::CANCUN)
            .build();

        Self { evm }
    }

    pub fn with_env(&mut self, block_number: u64, timestamp: u64, gas_price: u64) -> &mut Self {
        let env = self.evm.env_mut();
        env.block.number = RevmU256::from(block_number);
        env.block.timestamp = RevmU256::from(timestamp);
        env.block.gas_price = RevmU256::from(gas_price);
        env.tx.gas_price = RevmU256::from(gas_price);
        env.tx.gas_limit = 1_000_000_000u64;
        env.tx.caller = Address::ZERO;
        env.tx.kind = TxKind::Call(Address::ZERO);
        env.tx.value = RevmU256::ZERO;
        env.tx.data = Bytes::default();
        self
    }

    pub fn set_contract(&mut self, address: Address, code: Bytecode) -> &mut Self {
        let mut account = AccountInfo::default();
        account.code = Some(code);
        account.balance = RevmU256::from(100_000_000_000_000_000_000u128);
        self.evm.db_mut().insert_account_info(address, account);
        self
    }

    pub fn set_token_balance(
        &mut self,
        token: Address,
        account: Address,
        balance: u128,
    ) -> &mut Self {
        let slot = self.compute_erc20_balance_slot(account);
        self.evm.db_mut().insert_account_storage(
            token,
            BTreeMap::from([(slot, RevmU256::from(balance))]),
        );
        self
    }

    fn compute_erc20_balance_slot(&self, account: Address) -> RevmU256 {
        let mut hasher = tiny_keccak::Keccak::v256();
        let mut key = [0u8; 64];

        let addr_bytes: [u8; 32] = {
            let mut b = [0u8; 32];
            b[12..32].copy_from_slice(account.as_ref());
            b
        };

        let slot_bytes: [u8; 32] = {
            let mut b = [0u8; 32];
            b[31] = 0; // balance slot = 0
            b
        };

        key[..32].copy_from_slice(&addr_bytes);
        key[32..].copy_from_slice(&slot_bytes);

        let mut output = [0u8; 32];
        hasher.update(&key);
        hasher.finalize(&mut output);

        RevmU256::from_be_bytes(output)
    }

    pub fn simulate_swap(
        &mut self,
        pool: Address,
        zero_for_one: bool,
        amount: u128,
    ) -> Result<(), revm::primitives::EVMError> {
        let swap_data = self.encode_v3_swap(
            pool,
            zero_for_one,
            amount,
        );

        self.evm.env_mut().tx.data = Bytes::from(swap_data);
        let _result = self.evm.transact()?;
        Ok(())
    }

    fn encode_v3_swap(
        &self,
        pool: Address,
        zero_for_one: bool,
        amount: u128,
    ) -> Vec<u8> {
        let mut data = Vec::with_capacity(4 + 32 * 5);
        data.extend_from_slice(&[0x12, 0x80, 0xac, 0x4b]); // swap selector placeholder

        let recipient = [0u8; 32];
        data.extend_from_slice(&recipient);

        let mut zfo = [0u8; 32];
        if zero_for_one { zfo[31] = 1; }
        data.extend_from_slice(&zfo);

        let mut amt = [0u8; 32];
        amt[16..32].copy_from_slice(&amount.to_be_bytes());
        data.extend_from_slice(&amt);

        let mut price_limit = [0u8; 32];
        if zero_for_one {
            price_limit[31] = 1; // MIN_SQRT_RATIO + 1
        } else {
            price_limit[0] = 0xff;
        }
        data.extend_from_slice(&price_limit);

        data.extend_from_slice(&[0u8; 32]); // empty callback data offset

        data
    }

    pub fn simulate_arbitrage(
        &mut self,
        opportunity: &ArbOpportunity,
        state: &SharedState,
    ) -> SimulationResult {
        let gas_price = 50_000_000_000u64; // 50 gwei default
        self.with_env(0, 0, gas_price);

        debug!("Simulating arbitrage: {}", opportunity.id);

        let mut total_gas: u64 = 0;

        for step in &opportunity.route {
            match self.simulate_swap(step.pool, step.zero_for_one, step.amount) {
                Ok(()) => total_gas += 90_000,
                Err(e) => {
                    debug!("Simulation step failed: {:?}", e);
                    return SimulationResult {
                        profitable: false,
                        profit: 0,
                        gas_used: total_gas,
                        tip_amount: 0,
                    };
                }
            }
        }

        let estimated_profit = opportunity.estimated_profit;
        let gas_cost = total_gas as u128 * gas_price as u128;

        if estimated_profit > gas_cost && estimated_profit > 1000 {
            let net = estimated_profit - gas_cost;
            let tip = (net * state.config.bribe_percent as u128) / 100;

            SimulationResult {
                profitable: true,
                profit: net - tip,
                gas_used: total_gas,
                tip_amount: tip,
            }
        } else {
            SimulationResult {
                profitable: false,
                profit: 0,
                gas_used: total_gas,
                tip_amount: 0,
            }
        }
    }
}

pub fn scan_for_opportunities(
    state: &SharedState,
) -> Vec<ArbOpportunity> {
    let mut opportunities = Vec::new();

    for pool_entry in state.pool_states.iter() {
        let pool = pool_entry.value();
        debug!("Scanning pool: {:?}", pool.address);

        if let Some(_cex_prices) = state.cex_prices.read().as_ref() {
            let mock_opportunity = ArbOpportunity {
                id: uuid::Uuid::new_v4().to_string(),
                arb_type: ArbType::CexDex,
                pools: vec![pool.address],
                token_in: pool.token0,
                token_out: pool.token1,
                amount_in: 100_000_000_000_000_000u128,
                estimated_profit: 50_000_000_000_000_000u128,
                success_probability: 0.75,
                timestamp: Instant::now(),
                route: vec![SwapStep {
                    pool: pool.address,
                    zero_for_one: true,
                    amount: 100_000_000_000_000_000u128,
                    sqrt_price_limit: math::min_sqrt_ratio().as_limbs()[0] + 1,
                }],
                simulation_result: None,
            };
            opportunities.push(mock_opportunity);
        }
    }

    opportunities
}
