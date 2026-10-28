//! Project configuration parsed by the advisory reader and confirmed setup service.

use std::{collections::BTreeMap, fs, path::Path};

use serde::{Deserialize, Serialize};

pub const CONFIG_FILE: &str = ".development-system.toml";
pub const SCHEMA_VERSION: u32 = 3;
const MAX_CONFIG_BYTES: u64 = 256 * 1024;
const PROTECTED_PATHS: &[&str] = &[
    ".development-system.toml",
    ".development-system/",
    ".codex/agents/development-system-",
    ".git/",
];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityCategory {
    Tests,
    Source,
    Documentation,
    DeveloperEnvironment,
    BuildOutput,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandCapability {
    Tests,
    Implementation,
    Documentation,
    DeveloperEnvironment,
    Verification,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkPolicy {
    Denied,
    Allowed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterType {
    String,
    Integer,
    Boolean,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub category: CapabilityCategory,
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedService {
    pub readiness_command: String,
    pub shutdown_command: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectCommand {
    pub argv: Vec<String>,
    pub capability: CommandCapability,
    #[serde(default)]
    pub parameters: BTreeMap<String, ParameterType>,
    #[serde(default)]
    pub output_scopes: Vec<String>,
    #[serde(default)]
    pub network: Option<NetworkPolicy>,
    #[serde(default)]
    pub environment: Vec<String>,
    #[serde(default)]
    pub service: Option<ManagedService>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SigningRequirements {
    #[serde(default)]
    pub commit: bool,
    #[serde(default)]
    pub tag: bool,
}

/// Existing repository policy retained during the schema-v2 to schema-v3
/// migration. These settings remain outside the mutation-capability decision,
/// but are not disposable setup metadata.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryPolicy {
    pub mode: String,
    #[serde(default)]
    pub trunk_branch: Option<String>,
    #[serde(default)]
    pub merge_method: MergeMethod,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeMethod {
    #[default]
    Merge,
    Squash,
    Rebase,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ForgeProvider {
    GitHub,
    GitLab,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ForgePolicy {
    pub provider: ForgeProvider,
    /// Provider-native repository identity, for example `owner/name`.
    /// Remote URLs are never accepted by a semantic forge operation.
    pub repository: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FeaturePolicy {
    #[serde(default)]
    pub tiber: bool,
    #[serde(default)]
    pub agentic_systems: bool,
    #[serde(default)]
    pub eval_case_reporting: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorktreePolicy {
    pub root: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TiberPolicy {
    pub max_queued: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    pub schema_version: u32,
    pub scopes: BTreeMap<String, Scope>,
    #[serde(default)]
    pub commands: BTreeMap<String, ProjectCommand>,
    #[serde(default)]
    pub signing: Option<SigningRequirements>,
    #[serde(default)]
    pub diagnostic_mode: Option<bool>,
    #[serde(default)]
    pub model_routing: BTreeMap<String, String>,
    #[serde(default)]
    pub delivery: Option<DeliveryPolicy>,
    #[serde(default)]
    pub forge: Option<Box<ForgePolicy>>,
    #[serde(default)]
    pub features: Option<FeaturePolicy>,
    #[serde(default)]
    pub worktrees: Option<WorktreePolicy>,
    #[serde(default)]
    pub tiber: Option<TiberPolicy>,
    /// Final-review routing is consumed by the review service, which retains
    /// its own strict parser. Preserve this established policy verbatim while
    /// setup adds the capability schema around it.
    #[serde(default)]
    pub final_review: Option<toml::Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConfigState {
    Absent,
    Valid(ProjectConfig),
    Invalid(String),
}

impl ProjectConfig {
    pub fn parse(text: &str) -> Result<Self, String> {
        let config: Self = toml::from_str(text)
            .map_err(|error| format!("development_system.config_invalid source={error}"))?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION {
            return Err("development_system.config_schema_unsupported".to_string());
        }
        if self.scopes.is_empty() {
            return Err("development_system.config_scope_required".to_string());
        }
        for (id, scope) in &self.scopes {
            valid_identifier(id, "scope")?;
            if scope.include.is_empty() {
                return Err(format!("development_system.scope_include_required id={id}"));
            }
            for pattern in scope.include.iter().chain(&scope.exclude) {
                validate_glob(pattern)?;
            }
        }
        for (id, command) in &self.commands {
            valid_identifier(id, "command")?;
            command.validate(self)?;
        }
        if let Some(forge) = &self.forge {
            validate_forge_repository(&forge.repository)?;
        }
        Ok(())
    }

    pub fn digest(&self) -> String {
        // Stable, bounded configuration identity.  This detects stale
        // assignments without treating serialization order as authority.
        let serialized = toml::to_string(self).unwrap_or_default();
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in serialized.bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{hash:016x}")
    }
}

fn validate_forge_repository(repository: &str) -> Result<(), String> {
    let mut parts = repository.split('/');
    let valid_part = |part: &str| {
        !part.is_empty()
            && part.len() <= 100
            && part
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    };
    match (parts.next(), parts.next(), parts.next()) {
        (Some(owner), Some(name), None) if valid_part(owner) && valid_part(name) => Ok(()),
        _ => Err("development_system.forge_repository_invalid".to_string()),
    }
}

impl ProjectCommand {
    fn validate(&self, config: &ProjectConfig) -> Result<(), String> {
        if self.argv.is_empty() || self.argv.iter().any(|part| part.is_empty()) {
            return Err("development_system.command_argv_required".to_string());
        }
        let program = Path::new(&self.argv[0])
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if matches!(
            program,
            "sh" | "bash" | "zsh" | "fish" | "dash" | "pwsh" | "powershell" | "git" | "gh" | "glab"
        ) {
            return Err(format!(
                "development_system.command_program_forbidden program={program}"
            ));
        }
        if self.argv.iter().any(|part| {
            part.contains('\n') || part.contains('\r') || part.contains("$(") || part.contains('`')
        }) {
            return Err("development_system.command_argv_invalid".to_string());
        }
        for parameter in self.parameters.keys() {
            valid_identifier(parameter, "parameter")?;
        }
        for argument in &self.argv {
            if argument.contains('{') || argument.contains('}') {
                let parameter = argument
                    .strip_prefix('{')
                    .and_then(|value| value.strip_suffix('}'))
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        "development_system.command_parameter_reference_invalid".to_string()
                    })?;
                if !self.parameters.contains_key(parameter) {
                    return Err("development_system.command_parameter_unknown".to_string());
                }
            }
        }
        for scope_id in &self.output_scopes {
            if !config.scopes.contains_key(scope_id) {
                return Err(format!(
                    "development_system.command_output_scope_unknown id={scope_id}"
                ));
            }
        }
        if self.environment.iter().any(|name| {
            name.is_empty()
                || !name
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte == b'_' || byte.is_ascii_digit())
        }) {
            return Err("development_system.command_environment_invalid".to_string());
        }
        if let Some(service) = &self.service {
            valid_identifier(&service.readiness_command, "service_readiness_command")?;
            valid_identifier(&service.shutdown_command, "service_shutdown_command")?;
            if !config.commands.contains_key(&service.readiness_command)
                || !config.commands.contains_key(&service.shutdown_command)
            {
                return Err("development_system.service_command_unknown".to_string());
            }
        }
        Ok(())
    }
}

pub fn config_at(root: &Path) -> ConfigState {
    let path = root.join(CONFIG_FILE);
    let metadata = match fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return ConfigState::Absent,
        Err(error) => {
            return ConfigState::Invalid(format!(
                "development_system.config_read_failed source={error}"
            ));
        }
    };
    if metadata.len() > MAX_CONFIG_BYTES {
        return ConfigState::Invalid("development_system.config_too_large".to_string());
    }
    match fs::read_to_string(path) {
        Ok(text) => {
            ProjectConfig::parse(&text).map_or_else(ConfigState::Invalid, ConfigState::Valid)
        }
        Err(error) => ConfigState::Invalid(format!(
            "development_system.config_read_failed source={error}"
        )),
    }
}

fn valid_identifier(value: &str, kind: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 80
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
    {
        return Err(format!("development_system.{kind}_identifier_invalid"));
    }
    Ok(())
}

fn validate_glob(glob: &str) -> Result<(), String> {
    if glob.is_empty() || glob.starts_with('/') || glob.contains("..") || glob.contains('\\') {
        return Err("development_system.scope_glob_invalid".to_string());
    }
    if is_protected(glob) {
        return Err("development_system.scope_protected_path".to_string());
    }
    Ok(())
}

fn is_protected(path: &str) -> bool {
    PROTECTED_PATHS
        .iter()
        .any(|protected| path == protected.trim_end_matches('/') || path.starts_with(protected))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_configuration_roundtrip_preserves_identity() {
        let configuration = ProjectConfig::parse(
            r#"
            schema_version = 3
            diagnostic_mode = true
            [scopes.application]
            category = "source"
            include = ["apps/*/lib/**/*.ex"]
            exclude = ["apps/*/lib/**/*_test.exs"]
            [commands.verify]
            argv = ["mix", "test"]
            capability = "verification"
            output_scopes = []
            network = "denied"
            environment = []
            [commands.verify.parameters]
            name = "string"
            count = "integer"
            enabled = "boolean"
            [commands.verify.service]
            readiness_command = "verify"
            shutdown_command = "verify"
            [signing]
            commit = true
            tag = false
            [model_routing]
            review = "model"
            [delivery]
            mode = "direct-to-trunk"
            trunk_branch = "main"
            merge_method = "merge"
            [forge]
            provider = "git_hub"
            repository = "owner/repository"
            [features]
            tiber = true
            agentic_systems = true
            eval_case_reporting = true
            [worktrees]
            root = ".worktrees"
            [tiber]
            max_queued = 20
            [final_review.models]
            pre_filter = "strong-reviewer"
            lens_review = "substantive-worker"
            post_filter = "bounded-helper"
            verifier = "strong-reviewer"
            [final_review.models.codex]
            pre_filter = "model"
            lens_review = "model"
            post_filter = "model"
            verifier = "model"
            [final_review.dispositions.MAJOR]
            correctness-behavior = "block"
        "#,
        )
        .expect("complete configuration");
        let serialized = toml::to_string(&configuration).expect("serialized configuration");
        let reloaded = ProjectConfig::parse(&serialized).expect("reloaded configuration");
        assert_eq!(configuration.digest(), reloaded.digest());
    }

    fn config() -> ProjectConfig {
        ProjectConfig::parse(
            r#"schema_version = 3

[scopes.source]
category = "source"
include = ["src/**"]
exclude = ["src/generated/**"]

[scopes.tests]
category = "tests"
include = ["tests/**"]

[commands.unit]
argv = ["just", "test"]
capability = "tests"
output_scopes = ["tests"]
network = "denied"

[commands.implementation]
argv = ["just", "build"]
capability = "implementation"
"#,
        )
        .expect("valid config")
    }

    #[test]
    fn config_rejects_shell_git_protected_and_escape_surfaces() {
        for text in [
            "schema_version=3\n[scopes.source]\ncategory='source'\ninclude=['../src/**']",
            "schema_version=3\n[scopes.source]\ncategory='source'\ninclude=['.git/**']",
            "schema_version=3\n[scopes.source]\ncategory='source'\ninclude=['src/**']\n[commands.x]\nargv=['bash','-c','x']\ncapability='tests'",
            "schema_version=3\n[scopes.source]\ncategory='source'\ninclude=['src/**']\n[commands.x]\nargv=['git','status']\ncapability='tests'",
        ] {
            assert!(ProjectConfig::parse(text).is_err(), "{text}");
        }
    }

    #[test]
    fn forge_configuration_accepts_only_provider_native_repository_identity() {
        let mut valid = config();
        valid.forge = Some(Box::new(ForgePolicy {
            provider: ForgeProvider::GitHub,
            repository: "owner/project".to_string(),
        }));
        valid.validate().expect("provider identity");

        valid.forge = Some(Box::new(ForgePolicy {
            provider: ForgeProvider::GitHub,
            repository: "https://github.com/owner/project".to_string(),
        }));
        assert_eq!(
            valid.validate().expect_err("raw URL must fail closed"),
            "development_system.forge_repository_invalid"
        );
    }
}
