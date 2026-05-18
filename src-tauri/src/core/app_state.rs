use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};

use super::{central_repo, scenario_service, skill_store::SkillStore, sync_metadata, tool_service};

pub fn initialize_store() -> Result<Arc<SkillStore>> {
    initialize_store_inner(true)
}

pub fn initialize_cli_store() -> Result<Arc<SkillStore>> {
    initialize_store_inner(false)
}

fn initialize_store_inner(apply_startup_default: bool) -> Result<Arc<SkillStore>> {
    let total_start = Instant::now();

    let step = Instant::now();
    central_repo::ensure_central_repo().context("Failed to create central repo")?;
    log::info!(
        "startup: ensure_central_repo done in {} ms",
        step.elapsed().as_millis()
    );

    let db_path = central_repo::db_path();
    let step = Instant::now();
    let store = Arc::new(SkillStore::new(&db_path).context("Failed to initialize database")?);
    log::info!(
        "startup: open SkillStore done in {} ms",
        step.elapsed().as_millis()
    );

    let step = Instant::now();
    tool_service::migrate_legacy_tool_keys(&store)
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("Failed to migrate legacy tool keys")?;
    log::info!(
        "startup: migrate_legacy_tool_keys done in {} ms",
        step.elapsed().as_millis()
    );

    let skill_count = store.get_all_skills().map(|s| s.len()).unwrap_or(0);
    log::info!("startup: skill_count={skill_count}");

    if sync_metadata::metadata_exists() {
        let step = Instant::now();
        sync_metadata::reindex_from_metadata(&store)
            .context("Failed to reindex from sync metadata")?;
        log::info!(
            "startup: reindex_from_metadata done in {} ms (skills={skill_count})",
            step.elapsed().as_millis()
        );
    }

    let step = Instant::now();
    if scenario_service::restore_all_skills_sync_included(&store)
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("Failed to restore skill sync inclusion")?
    {
        log::info!(
            "startup: restore_all_skills_sync_included changed rows in {} ms",
            step.elapsed().as_millis()
        );
        let step = Instant::now();
        sync_metadata::write_all_from_db(&store)
            .context("Failed to persist restored skill sync inclusion")?;
        log::info!(
            "startup: write_all_from_db done in {} ms",
            step.elapsed().as_millis()
        );
    } else {
        log::info!(
            "startup: restore_all_skills_sync_included no-op in {} ms",
            step.elapsed().as_millis()
        );
    }

    let step = Instant::now();
    if apply_startup_default {
        scenario_service::ensure_default_startup_scenario(&store)
            .map_err(|e| anyhow::anyhow!(e.to_string()))
            .context("Failed to initialize startup scenario")?;
        log::info!(
            "startup: ensure_default_startup_scenario done in {} ms (skills={skill_count})",
            step.elapsed().as_millis()
        );
    } else {
        scenario_service::ensure_cli_scenario_state(&store)
            .map_err(|e| anyhow::anyhow!(e.to_string()))
            .context("Failed to initialize CLI scenario state")?;
        log::info!(
            "startup: ensure_cli_scenario_state done in {} ms",
            step.elapsed().as_millis()
        );
    }

    log::info!(
        "startup: initialize_store total {} ms (skills={skill_count})",
        total_start.elapsed().as_millis()
    );
    Ok(store)
}
