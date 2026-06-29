//! The side-quest registry: a durable record of launched side-quests
//! (imperative shell).
//!
//! Stored under the project's `.git/` directory so it is shared across the main
//! checkout and its worktrees and never shows up as a tracked change.

use std::path::{Path, PathBuf};

use sidequest_core::side_quest::SideQuestRecord;
use thiserror::Error;

/// A failure reading or writing the registry.
#[derive(Debug, Error)]
pub enum RegistryError {
    /// The registry file could not be read or written.
    #[error("registry-io-failed: {0}")]
    Io(String),
    /// The registry file held invalid JSON.
    #[error("registry-parse-failed: {0}")]
    Parse(String),
}

fn registry_path(project_root: &Path) -> PathBuf {
    project_root
        .join(".git")
        .join("sidequest")
        .join("registry.json")
}

/// Read all recorded side-quests (empty when the registry does not yet exist).
///
/// # Errors
///
/// Returns a [`RegistryError`] if the registry exists but cannot be read or
/// parsed.
pub async fn list(project_root: &Path) -> Result<Vec<SideQuestRecord>, RegistryError> {
    let path = registry_path(project_root);
    match tokio::fs::read(&path).await {
        Ok(bytes) => {
            serde_json::from_slice(&bytes).map_err(|error| RegistryError::Parse(error.to_string()))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(RegistryError::Io(error.to_string())),
    }
}

/// Record `record`, replacing any existing record on the same branch.
///
/// # Errors
///
/// Returns a [`RegistryError`] if the registry cannot be read or written.
pub async fn record(project_root: &Path, record: SideQuestRecord) -> Result<(), RegistryError> {
    let mut records = list(project_root).await?;
    records.retain(|existing| existing.branch != record.branch);
    records.push(record);

    let path = registry_path(project_root);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| RegistryError::Io(error.to_string()))?;
    }
    let bytes = serde_json::to_vec_pretty(&records)
        .map_err(|error| RegistryError::Parse(error.to_string()))?;
    tokio::fs::write(&path, bytes)
        .await
        .map_err(|error| RegistryError::Io(error.to_string()))
}
