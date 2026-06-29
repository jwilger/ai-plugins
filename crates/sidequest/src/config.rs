//! Loading project configuration from disk (imperative shell).

use std::path::Path;

use anyhow::Context as _;
use sidequest_core::config::Config;

/// Load `sidequest.toml` from `project_root`, returning the default config when
/// the file is absent.
///
/// # Errors
///
/// Returns an error if the file exists but cannot be read or parsed.
pub async fn load(project_root: &Path) -> anyhow::Result<Config> {
    let path = project_root.join("sidequest.toml");
    match tokio::fs::read_to_string(&path).await {
        Ok(text) => Config::from_toml(&text).context("parsing sidequest.toml"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(error) => Err(error).context("reading sidequest.toml"),
    }
}
