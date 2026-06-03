use std::sync::Arc;
use alloy::primitives::{Address, U256, Bytes};
use serde::{Deserialize, Serialize};
use tokio::time::{timeout, Duration};
use tracing::{info, warn, debug, error};

use crate::{
    SharedState, ArbOpportunity, ArbType,
    simulator::Simulator,
};

const FLASHBOTS_RELAY_RPC: &str = "https://relay.flashbots.net";
const BEAVERBUILD_RELAY: &str = "https://rpc.beaverbuild.org";
const TITAN_RELAY: &str = "https://rpc.titanbuilder.xyz";
const BUILDER069_RELAY: &str = "https://rpc.builder069.xyz";
const DEFAULT_GAS_LIMIT: u64 = 1_500_000;
const BUNDLE_TIMEOUT_SECS: u64 = 5;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FlashbotsBundle {
    pub txs: Vec<String>,
    pub block_number: String,
    pub min_timestamp: Option<u64>,
    pub max_timestamp: Option<u64>,
    pub reverting_hashes: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BundleResponse {
    pub bundle_hash: Option<String>,
    pub error: Option<String>,
}

pub struct BundleSubmissionResult {
    pub accepted: bool,
    pub relay: String,
    pub bundle_hash: Option<String>,
    pub latency_ms: u64,
}

pub async fn build_and_submit_bundle(
    opportunity: &ArbOpportunity,
    state: &Arc<SharedState>,
) -> anyhow::Result<BundleSubmissionResult> {
    let start = std::time::Instant::now();

    let flashbots_endpoint = state.config.flashbots_endpoint
        .clone()
        .unwrap_or_else(|| FLASHBOTS_RELAY_RPC.to_string());

    let key = &state.config.executor_private_key;

    if key == "0x0000000000000000000000000000000000000000" || key.is_empty() {
        return Err(anyhow::anyhow!("Executor private key not configured"));
    }

    let tx_data = build_transaction_call(opportunity, state)?;

    let bundle = FlashbotsBundle {
        txs: vec![tx_data],
        block_number: "latest".to_string(),
        min_timestamp: None,
        max_timestamp: None,
        reverting_hashes: vec![],
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(BUNDLE_TIMEOUT_SECS))
        .build()?;

    let response = match client
        .post(&flashbots_endpoint)
        .json(&bundle)
        .header("X-Flashbots-Simulation-Only", "false")
        .send()
        .await
    {
        Ok(resp) => resp.json::<BundleResponse>().await.unwrap_or(BundleResponse {
            bundle_hash: None,
            error: Some("Failed to parse response".to_string()),
        }),
        Err(e) => BundleResponse {
            bundle_hash: None,
            error: Some(format!("HTTP error: {}", e)),
        },
    };

    let latency = start.elapsed().as_millis() as u64;

    if response.error.is_some() {
        warn!("Bundle submission error: {:?}", response.error);
    }

    state.metrics.bundles_submitted.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    Ok(BundleSubmissionResult {
        accepted: response.error.is_none(),
        relay: flashbots_endpoint,
        bundle_hash: response.bundle_hash,
        latency_ms: latency,
    })
}

pub async fn submit_to_all_relays(
    opportunity: &ArbOpportunity,
    state: &Arc<SharedState>,
) -> Vec<BundleSubmissionResult> {
    let mut results = Vec::new();

    let relays = vec![
        state.config.flashbots_endpoint.clone().unwrap_or_else(|| FLASHBOTS_RELAY_RPC.to_string()),
        state.config.beaver_build_endpoint.clone().unwrap_or_else(|| BEAVERBUILD_RELAY.to_string()),
        state.config.titan_endpoint.clone().unwrap_or_else(|| TITAN_RELAY.to_string()),
        state.config.builder069_endpoint.clone().unwrap_or_else(|| BUILDER069_RELAY.to_string()),
    ];

    for relay in relays {
        match build_and_submit_bundle(opportunity, state).await {
            Ok(result) => {
                if result.accepted {
                    info!("Bundle accepted by {}", relay);
                }
                results.push(result);
            }
            Err(e) => {
                debug!("Relay {} failed: {}", relay, e);
            }
        }
    }

    results
}

pub fn build_transaction_call(
    opportunity: &ArbOpportunity,
    state: &SharedState,
) -> anyhow::Result<String> {
    let executor = state.config.executor_address;

    let selector: [u8; 4] = match opportunity.arb_type {
        ArbType::TriangularV3 => [0x00, 0x00, 0x00, 0x01], // placeholder
        ArbType::CrossPoolV3V4 => [0x00, 0x00, 0x00, 0x02], // placeholder
        ArbType::CexDex => [0x00, 0x00, 0x00, 0x03], // placeholder
        ArbType::JitLiquidity => [0x00, 0x00, 0x00, 0x04], // placeholder
    };

    let tx_hex = format!(
        "0x{}",
        hex::encode(selector)
    );

    Ok(tx_hex)
}

pub async fn auto_execute_opportunities(
    state: &SharedState,
) -> anyhow::Result<()> {
    let mut sim = Simulator::new();
    let mut opportunities = state.pending_opportunities.write().await;

    let mut i = 0;
    while i < opportunities.len() {
        let opportunity = &opportunities[i];
        if opportunity.timestamp.elapsed().as_secs() > 30 {
            opportunities.swap_remove(i);
            continue;
        }

        let result = sim.simulate_arbitrage(opportunity, state);

        if result.profitable && result.profit > 1000 {
            state.metrics.profitable_ops.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            info!(
                "Profitable opportunity found: {} | Profit: {} wei | Tip: {}",
                opportunity.id, result.profit, result.tip_amount
            );

            let _results = submit_to_all_relays(opportunity, state).await;
        }

        i += 1;
    }

    Ok(())
}
