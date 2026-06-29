//! Project configuration (`sidequest.toml`). Pure parsing — the shell reads the
//! file; this module turns its text into typed config.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// How a finished side-quest's work is delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeliveryMode {
    /// Merge the side-quest branch into the local integration branch.
    LocalMerge,
    /// Push the side-quest branch to the origin integration branch.
    PushOrigin,
    /// Open a pull/merge request for the side-quest branch.
    Pr,
}

impl DeliveryMode {
    /// The kebab-case identifier used in `sidequest.toml` and on the wire.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LocalMerge => "local-merge",
            Self::PushOrigin => "push-origin",
            Self::Pr => "pr",
        }
    }
}

/// The `[delivery]` table.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DeliverySection {
    /// The default delivery mode, if configured.
    pub mode: Option<DeliveryMode>,
}

/// Parsed `sidequest.toml`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Delivery configuration.
    pub delivery: DeliverySection,
}

impl Config {
    /// Parse configuration from `sidequest.toml` text.
    ///
    /// # Errors
    ///
    /// Returns a [`ConfigError`] if the TOML is malformed or names an unknown
    /// delivery mode.
    pub fn from_toml(text: &str) -> Result<Self, ConfigError> {
        toml::from_str(text).map_err(|error| ConfigError::Parse(error.to_string()))
    }

    /// The configured default delivery mode, if any.
    #[must_use]
    pub fn delivery_mode(&self) -> Option<DeliveryMode> {
        self.delivery.mode
    }
}

/// A failure parsing configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The TOML was malformed or contained an unknown value.
    #[error("config-parse-failed: {0}")]
    Parse(String),
}

#[cfg(test)]
#[expect(clippy::expect_used, reason = "unit tests use expect() for clarity")]
mod tests {
    use super::*;

    #[test]
    fn reads_the_delivery_mode() {
        let config =
            Config::from_toml("[delivery]\nmode = \"local-merge\"\n").expect("valid config parses");
        assert_eq!(
            config.delivery_mode(),
            Some(DeliveryMode::LocalMerge),
            "the configured delivery mode is read"
        );
    }

    #[test]
    fn empty_config_has_no_delivery_mode() {
        let config = Config::from_toml("").expect("empty config is valid");
        assert_eq!(
            config.delivery_mode(),
            None,
            "an unconfigured project has no default delivery mode"
        );
    }

    #[test]
    fn unknown_delivery_mode_is_rejected() {
        assert!(
            Config::from_toml("[delivery]\nmode = \"teleport\"\n").is_err(),
            "an unknown delivery mode is a parse error"
        );
    }
}
