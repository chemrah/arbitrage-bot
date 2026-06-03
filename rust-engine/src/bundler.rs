use std::sync::Arc;
use tracing::{info, debug};

use crate::{SharedState, ArbOpportunity};

pub async fn build_and_submit_bundle(
    _opportunity: &ArbOpportunity,
    _state: &Arc<SharedState>,
) -> anyhow::Result<()> {
    info!("Submitting bundle...");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    Ok(())
}

pub async fn submit_to_all_relays(
    _opportunity: &ArbOpportunity,
    _state: &Arc<SharedState>,
) -> Vec<String> {
    vec!["flashbots".to_string(), "beaverbuild".to_string()]
}

pub async fn auto_execute_opportunities(
    _state: &SharedState,
) -> anyhow::Result<()> {
    debug!("Auto-execute cycle (no-op in dev mode)");
    Ok(())
}
