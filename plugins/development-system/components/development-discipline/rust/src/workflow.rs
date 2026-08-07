//! Reusable, checked development-workflow lifecycle.
//!
//! This module is deliberately independent of Tiber: tickets and CI recovery
//! are optional project concerns, while test-first evidence is a development
//! discipline concern.

use std::{
    ffi::OsStr,
    fmt, fs,
    path::{Path, PathBuf},
    process::Command,
};

use eventcore::{mapping, ModelInput, ModelOutput};
use fs2::FileExt;
use serde::{Deserialize, Serialize};

const BLOCKER_DIRECTORY: &str = "development-discipline";
const BLOCKER_FILE: &str = "workflow-blocker.json";
const LEGACY_BLOCKER_DIRECTORY: &str = "tiber";
const STATE_FILE: &str = "workflow-state.json";
const STATE_LOCK_FILE: &str = "workflow-state.lock";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkflowBlocker {
    schema_version: u8,
    kind: String,
    error_code: String,
    required_action: String,
    created_at: u64,
}

/// Evaluates one harness hook invocation against the repository-wide workflow
/// hold. The hold lives under Git's common directory, so linked worktrees
/// cannot continue independently after an unresolved pushed-CI failure.
pub fn guard(hook_input: &str) -> Result<Option<String>, String> {
    let input: serde_json::Value = serde_json::from_str(hook_input)
        .map_err(|error| format!("development_workflow.hook_input_invalid source={error}"))?;
    let cwd = input
        .get("cwd")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(".");
    let common_directory = common_git_directory(Path::new(cwd))?;
    if crate::ci_recovery::active_hold(Path::new(cwd))? {
        if permits_recovery(&input) {
            return Ok(None);
        }
        return Ok(Some(
            "development_workflow.blocked workflow_blocked=true error_code=development_workflow.ci_recovery_active required_action=\"complete, transfer, or wait on the shared Development Workflow CI recovery\". Do not diagnose, edit, test, rerun, push, or perform unrelated work.".to_string(),
        ));
    }
    let blocker_path = common_directory.join(BLOCKER_DIRECTORY).join(BLOCKER_FILE);
    let legacy_blocker_path = common_directory
        .join(LEGACY_BLOCKER_DIRECTORY)
        .join(BLOCKER_FILE);
    let blocker_contents = match fs::read_to_string(&blocker_path) {
        Ok(contents) => Some(contents),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            match fs::read_to_string(&legacy_blocker_path) {
                Ok(contents) => Some(contents),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => {
                    return Err(format!(
                        "development_workflow.hold_read_failed source={error}"
                    ))
                }
            }
        }
        Err(error) => {
            return Err(format!(
                "development_workflow.hold_read_failed source={error}"
            ))
        }
    };
    if let Some(contents) = blocker_contents {
        let blocker: WorkflowBlocker = serde_json::from_str(&contents).map_err(|_| {
            "development_workflow.hold_invalid workflow_blocked=true required_action=\"repair the Development Discipline workflow hold before continuing\"".to_string()
        })?;
        if blocker.schema_version != 1 || blocker.kind.is_empty() || blocker.created_at == 0 {
            return Err("development_workflow.hold_invalid workflow_blocked=true".to_string());
        }
        if permits_recovery(&input) {
            return Ok(None);
        }
        return Ok(Some(format!(
            "development_workflow.blocked workflow_blocked=true error_code={} required_action=\"{}\". Do not diagnose, edit, test, rerun, push, or perform unrelated work.",
            blocker.error_code, blocker.required_action
        )));
    }
    lifecycle_guard(&common_directory, &input)
}

fn lifecycle_guard(
    common_directory: &Path,
    input: &serde_json::Value,
) -> Result<Option<String>, String> {
    let state_path = common_directory.join(BLOCKER_DIRECTORY).join(STATE_FILE);
    let Some(workflow) = read_workflow_state(&state_path)? else {
        return if !is_repository_mutation(input) || permits_workflow_control(input) {
            Ok(None)
        } else {
            Ok(Some(
                "development_workflow.blocked workflow_blocked=true error_code=development_workflow.lifecycle_required required_action=\"start a production or explicit-exemption workflow before mutating the repository\"".to_string(),
            ))
        };
    };
    if workflow.is_terminal() || workflow.phase == Phase::Exempt || permits_workflow_control(input)
    {
        return Ok(None);
    }
    match workflow.phase {
        Phase::AwaitingRed if is_test_edit(input) || !is_repository_mutation(input) => Ok(None),
        Phase::AwaitingRed => Ok(Some(
            "development_workflow.blocked workflow_blocked=true error_code=development_workflow.red_evidence_required required_action=\"add or update a focused test, then record its observed failure\"".to_string(),
        )),
        Phase::AwaitingImplementation if !is_repository_mutation(input) => Ok(None),
        Phase::AwaitingImplementation => Ok(Some(
            "development_workflow.blocked workflow_blocked=true error_code=development_workflow.implementation_authorization_required required_action=\"authorize implementation after red evidence\"".to_string(),
        )),
        Phase::AwaitingReview | Phase::Reviewing if !is_repository_mutation(input) => Ok(None),
        Phase::AwaitingReview | Phase::Reviewing => Ok(Some(
            "development_workflow.blocked workflow_blocked=true error_code=development_workflow.review_required required_action=\"complete final review before further delivery work\"".to_string(),
        )),
        Phase::AwaitingGreen | Phase::Delivered | Phase::Abandoned | Phase::Exempt => Ok(None),
        Phase::AwaitingDelivery if !is_repository_mutation(input) => Ok(None),
        Phase::AwaitingDelivery => Ok(Some(
            "development_workflow.blocked workflow_blocked=true error_code=development_workflow.delivery_authorization_required required_action=\"authorize delivery before further repository mutations\"".to_string(),
        )),
    }
}

fn permits_workflow_control(input: &serde_json::Value) -> bool {
    let tool_name = input
        .get("tool_name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    tool_name.contains("workflow.")
        || tool_name.contains("final_review.")
        || tool_name.contains("__workflow_")
        || tool_name.contains("__final_review_")
        || permits_recovery(input)
}

/// Returns whether the hook invocation can modify repository state.
///
/// The lifecycle gate intentionally has no read/research allowlist: an
/// unrecognized tool is allowed unless it is known to mutate. This keeps
/// inspection, web research, and other non-mutating work available at every
/// phase instead of making progress depend on a brittle list of commands.
fn is_repository_mutation(input: &serde_json::Value) -> bool {
    is_file_mutation_tool(input) || is_known_terminal_mutation(input)
}

fn is_file_mutation_tool(input: &serde_json::Value) -> bool {
    let tool_name = input
        .get("tool_name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    [
        "write",
        "edit",
        "applypatch",
        "apply_patch",
        "delete",
        "rename",
        "move",
    ]
    .iter()
    .any(|operation| tool_name.contains(operation))
}

fn is_known_terminal_mutation(input: &serde_json::Value) -> bool {
    let command = input
        .pointer("/tool_input/command")
        .or_else(|| input.pointer("/tool_input/cmd"))
        .and_then(serde_json::Value::as_str);
    let Some(command) = command else {
        return false;
    };
    command.lines().any(terminal_line_mutates)
}

fn terminal_line_mutates(command: &str) -> bool {
    let normalized = command.to_ascii_lowercase();
    let write_markers = [
        "apply_patch",
        "sed -i",
        "sed --in-place",
        "perl -i",
        "perl --in-place",
        "tee ",
        "touch ",
        "mkdir ",
        "rm ",
        "rm -",
        "mv ",
        "cp ",
        "patch ",
        "git add ",
        "git commit",
        "git push",
        "git reset",
        "git checkout",
        "git restore",
        "git clean",
        "git merge",
        "git rebase",
        "npm install",
        "npm ci",
        "cargo fmt",
        "prettier --write",
    ];
    normalized.contains('>')
        || write_markers
            .iter()
            .any(|marker| normalized.starts_with(marker) || normalized.contains(marker))
}

fn is_test_edit(input: &serde_json::Value) -> bool {
    let tool_input = input.get("tool_input").unwrap_or(&serde_json::Value::Null);
    let explicit_paths = ["path", "file_path"]
        .iter()
        .filter_map(|field| tool_input.get(*field).and_then(serde_json::Value::as_str))
        .collect::<Vec<_>>();
    if !explicit_paths.is_empty() {
        return explicit_paths.into_iter().all(is_test_path);
    }
    let patch = tool_input
        .get("patch")
        .and_then(serde_json::Value::as_str)
        .or_else(|| tool_input.as_str());
    patch.map(patch_targets).is_some_and(|targets| {
        !targets.is_empty() && targets.iter().all(|target| is_test_path(target))
    })
}

fn patch_targets(patch: &str) -> Vec<&str> {
    patch
        .lines()
        .filter_map(|line| {
            line.strip_prefix("*** Update File: ")
                .or_else(|| line.strip_prefix("*** Add File: "))
                .or_else(|| line.strip_prefix("*** Delete File: "))
                .or_else(|| line.strip_prefix("+++ b/"))
        })
        .collect()
}

fn is_test_path(value: &str) -> bool {
    value.split(['/', '\\', '\n', '\r']).any(|segment| {
        let segment = segment.to_ascii_lowercase();
        segment == "test"
            || segment == "tests"
            || segment == "spec"
            || segment == "specs"
            || segment.contains("_test.")
            || segment.contains("_spec.")
    })
}

fn common_git_directory(cwd: &Path) -> Result<PathBuf, String> {
    let output = Command::new("git")
        .args(["rev-parse", "--git-common-dir"])
        .current_dir(cwd)
        .output()
        .map_err(|error| format!("development_workflow.git_unavailable source={error}"))?;
    if !output.status.success() {
        return Err("development_workflow.git_repository_required=true".to_string());
    }
    let path = String::from_utf8(output.stdout)
        .map_err(|_| "development_workflow.git_path_invalid=true".to_string())?;
    let path = PathBuf::from(path.trim());
    Ok(if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    })
}

fn permits_recovery(input: &serde_json::Value) -> bool {
    let tool_name = input
        .get("tool_name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if tool_name.contains("workflow.ci_recovery.") || tool_name.ends_with("workflow.status") {
        return true;
    }
    let command = input
        .pointer("/tool_input/command")
        .or_else(|| input.pointer("/tool_input/cmd"))
        .and_then(serde_json::Value::as_str);
    let Some(command) = command else {
        return false;
    };
    if command.contains("$(")
        || command
            .chars()
            .any(|character| matches!(character, ';' | '|' | '&' | '<' | '>' | '\n' | '\r' | '`'))
    {
        return false;
    }
    let Some(tokens) = shlex::split(command) else {
        return false;
    };
    let Some(executable) = tokens.first() else {
        return false;
    };
    Path::new(executable).file_name() == Some(OsStr::new("development-discipline-mcp"))
        && matches!(
            tokens.get(1).map(String::as_str),
            Some("ci-recovery") | Some("workflow-status")
        )
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ChangeKind {
    Production,
    Exempt,
}

impl ChangeKind {
    pub fn parse(value: &str) -> Result<Self, WorkflowError> {
        match value {
            "production" => Ok(Self::Production),
            "exempt" => Ok(Self::Exempt),
            _ => Err(WorkflowError::UnexpectedEvidence),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
enum Phase {
    AwaitingRed,
    AwaitingImplementation,
    AwaitingGreen,
    AwaitingReview,
    Reviewing,
    AwaitingDelivery,
    Delivered,
    Abandoned,
    Exempt,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Workflow {
    phase: Phase,
    change_kind: ChangeKind,
    red_observed: bool,
    green_observed: bool,
    clean_review_observed: bool,
}

pub fn start_at(project_root: &Path, change_kind: ChangeKind) -> Result<Workflow, String> {
    with_locked_state(project_root, |existing| {
        if existing.is_some_and(|workflow| !workflow.is_terminal()) {
            return Err("development_workflow.active_lifecycle_exists".to_string());
        }
        Ok(Workflow::start(change_kind))
    })
}

pub fn transition_at(project_root: &Path, action: &str) -> Result<Workflow, String> {
    with_locked_state(project_root, |existing| {
        let mut workflow =
            existing.ok_or_else(|| "development_workflow.not_started".to_string())?;
        let ci_hold_active = if action == "authorize_delivery" {
            ci_hold_active_at(project_root)?
        } else {
            false
        };
        workflow
            .transition(action, ci_hold_active)
            .map_err(|error| error.to_string())?;
        Ok(workflow)
    })
}

fn ci_hold_active_at(project_root: &Path) -> Result<bool, String> {
    // The remote workflow ref is authoritative across independent clones.  A
    // local blocker remains a fail-closed bridge for a failed claim/publish.
    if crate::ci_recovery::active_hold(project_root)? {
        return Ok(true);
    }
    let common_directory = common_git_directory(project_root)?;
    let current = common_directory.join(BLOCKER_DIRECTORY).join(BLOCKER_FILE);
    let legacy = common_directory
        .join(LEGACY_BLOCKER_DIRECTORY)
        .join(BLOCKER_FILE);
    for path in [current, legacy] {
        match fs::read_to_string(path) {
            Ok(contents) => {
                let blocker: WorkflowBlocker = serde_json::from_str(&contents).map_err(|_| {
                    "development_workflow.hold_invalid workflow_blocked=true".to_string()
                })?;
                if blocker.schema_version == 1 && !blocker.kind.is_empty() && blocker.created_at > 0
                {
                    return Ok(true);
                }
                return Err("development_workflow.hold_invalid workflow_blocked=true".to_string());
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "development_workflow.hold_read_failed source={error}"
                ));
            }
        }
    }
    Ok(false)
}

pub fn status_at(project_root: &Path) -> Result<Workflow, String> {
    let state_path = workflow_state_path(project_root)?;
    read_workflow_state(&state_path)?.ok_or_else(|| "development_workflow.not_started".to_string())
}

fn with_locked_state(
    project_root: &Path,
    transition: impl FnOnce(Option<Workflow>) -> Result<Workflow, String>,
) -> Result<Workflow, String> {
    let state_path = workflow_state_path(project_root)?;
    let directory = state_path
        .parent()
        .expect("workflow state always has a parent");
    fs::create_dir_all(directory)
        .map_err(|error| format!("development_workflow.state_directory_failed source={error}"))?;
    let lock = fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(directory.join(STATE_LOCK_FILE))
        .map_err(|error| format!("development_workflow.state_lock_open_failed source={error}"))?;
    lock.lock_exclusive()
        .map_err(|error| format!("development_workflow.state_lock_failed source={error}"))?;
    let result = (|| {
        let workflow = transition(read_workflow_state(&state_path)?)?;
        write_workflow_state(&state_path, &workflow)?;
        Ok(workflow)
    })();
    let _ = lock.unlock();
    result
}

fn workflow_state_path(project_root: &Path) -> Result<PathBuf, String> {
    Ok(common_git_directory(project_root)?
        .join(BLOCKER_DIRECTORY)
        .join(STATE_FILE))
}

fn read_workflow_state(path: &Path) -> Result<Option<Workflow>, String> {
    let contents = match fs::read(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "development_workflow.state_read_failed source={error}"
            ))
        }
    };
    serde_json::from_slice(&contents)
        .map(Some)
        .map_err(|_| "development_workflow.state_invalid=true".to_string())
}

fn write_workflow_state(path: &Path, workflow: &Workflow) -> Result<(), String> {
    let bytes = serde_json::to_vec(workflow)
        .map_err(|error| format!("development_workflow.state_encode_failed source={error}"))?;
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(&temporary, bytes)
        .map_err(|error| format!("development_workflow.state_write_failed source={error}"))?;
    fs::rename(&temporary, path)
        .map_err(|error| format!("development_workflow.state_publish_failed source={error}"))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkflowError {
    RedEvidenceRequired,
    GreenEvidenceRequired,
    ReviewEvidenceRequired,
    DeliveryEvidenceRequired,
    UnexpectedEvidence,
}

impl Workflow {
    #[must_use]
    pub fn start(change_kind: ChangeKind) -> Self {
        let gate = model_gate(change_kind, false, false, false, false);
        Self {
            phase: if gate.red_required {
                Phase::AwaitingRed
            } else {
                // Exemption removes only the RED prerequisite. Successful
                // verification, review, and delivery remain mandatory.
                Phase::AwaitingGreen
            },
            change_kind,
            red_observed: false,
            green_observed: false,
            clean_review_observed: false,
        }
    }

    pub fn record_red(&mut self) -> Result<(), WorkflowError> {
        if self.phase != Phase::AwaitingRed {
            return Err(WorkflowError::UnexpectedEvidence);
        }
        self.red_observed = true;
        self.phase = Phase::AwaitingImplementation;
        Ok(())
    }

    pub fn authorize_implementation(&mut self) -> Result<(), WorkflowError> {
        let gate = self.gate(false);
        match self.phase {
            Phase::AwaitingRed if !gate.implementation_allowed => {
                Err(WorkflowError::RedEvidenceRequired)
            }
            Phase::AwaitingImplementation if gate.implementation_allowed => {
                self.phase = Phase::AwaitingGreen;
                Ok(())
            }
            Phase::Exempt => {
                self.phase = Phase::AwaitingGreen;
                Ok(())
            }
            _ => Err(WorkflowError::GreenEvidenceRequired),
        }
    }

    pub fn record_green(&mut self) -> Result<(), WorkflowError> {
        if !matches!(self.phase, Phase::AwaitingGreen | Phase::Exempt) {
            return Err(WorkflowError::UnexpectedEvidence);
        }
        self.green_observed = true;
        self.phase = Phase::AwaitingReview;
        Ok(())
    }

    pub fn authorize_review(&mut self) -> Result<(), WorkflowError> {
        if self.phase != Phase::AwaitingReview || !self.gate(false).review_allowed {
            return Err(WorkflowError::GreenEvidenceRequired);
        }
        self.phase = Phase::Reviewing;
        Ok(())
    }

    pub fn record_clean_review(&mut self) -> Result<(), WorkflowError> {
        if self.phase != Phase::Reviewing {
            return Err(WorkflowError::UnexpectedEvidence);
        }
        self.clean_review_observed = true;
        self.phase = Phase::AwaitingDelivery;
        Ok(())
    }

    pub fn authorize_delivery(&mut self, ci_hold_active: bool) -> Result<(), WorkflowError> {
        let gate = self.gate(ci_hold_active);
        match self.phase {
            Phase::AwaitingDelivery if gate.delivery_allowed => {
                self.phase = Phase::Delivered;
                Ok(())
            }
            Phase::Exempt => Err(WorkflowError::GreenEvidenceRequired),
            Phase::Reviewing => Err(WorkflowError::ReviewEvidenceRequired),
            _ => Err(WorkflowError::DeliveryEvidenceRequired),
        }
    }

    pub fn abandon(&mut self) -> Result<(), WorkflowError> {
        if self.is_terminal() {
            return Err(WorkflowError::UnexpectedEvidence);
        }
        self.phase = Phase::Abandoned;
        Ok(())
    }

    fn gate(&self, ci_hold_active: bool) -> ModelGate {
        model_gate(
            self.change_kind,
            self.red_observed,
            self.green_observed,
            self.clean_review_observed,
            ci_hold_active,
        )
    }

    pub fn phase_name(&self) -> &'static str {
        match self.phase {
            Phase::AwaitingRed => "awaiting_red",
            Phase::AwaitingImplementation => "awaiting_implementation",
            Phase::AwaitingGreen => "awaiting_green",
            Phase::AwaitingReview => "awaiting_review",
            Phase::Reviewing => "reviewing",
            Phase::AwaitingDelivery => "awaiting_delivery",
            Phase::Delivered => "delivered",
            Phase::Abandoned => "abandoned",
            Phase::Exempt => "exempt",
        }
    }

    fn is_terminal(&self) -> bool {
        matches!(self.phase, Phase::Delivered | Phase::Abandoned)
    }

    pub fn transition(&mut self, action: &str, ci_hold_active: bool) -> Result<(), WorkflowError> {
        match action {
            "record_red" => self.record_red(),
            "authorize_implementation" => self.authorize_implementation(),
            "record_green" => self.record_green(),
            "authorize_review" => self.authorize_review(),
            "record_clean_review" => self.record_clean_review(),
            "authorize_delivery" => self.authorize_delivery(ci_hold_active),
            "abandon" => self.abandon(),
            _ => Err(WorkflowError::UnexpectedEvidence),
        }
    }
}

impl fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::RedEvidenceRequired => "development_workflow.red_evidence_required",
            Self::GreenEvidenceRequired => "development_workflow.green_evidence_required",
            Self::ReviewEvidenceRequired => "development_workflow.review_evidence_required",
            Self::DeliveryEvidenceRequired => "development_workflow.delivery_evidence_required",
            Self::UnexpectedEvidence => "development_workflow.unexpected_evidence",
        })
    }
}

impl std::error::Error for WorkflowError {}

#[derive(ModelInput)]
struct WorkflowStart {
    #[model(origin)]
    kind: ChangeKind,
}

#[derive(ModelInput)]
struct RedEvidence {
    #[model(origin)]
    observed: bool,
}

#[derive(ModelInput)]
struct GreenEvidence {
    #[model(origin)]
    observed: bool,
}

#[derive(ModelInput)]
struct ReviewEvidence {
    #[model(origin)]
    clean: bool,
}

#[derive(ModelInput)]
struct CiRecoveryHold {
    #[model(origin)]
    active: bool,
}

#[derive(ModelOutput)]
struct RedRequired {
    value: bool,
}

#[derive(ModelOutput)]
struct ImplementationAllowed {
    value: bool,
}

#[derive(ModelOutput)]
struct ReviewAllowed {
    value: bool,
}

#[derive(ModelOutput)]
struct DeliveryAllowed {
    value: bool,
}

fn requires_red(kind: &ChangeKind) -> bool {
    matches!(kind, ChangeKind::Production)
}
fn implementation_allowed(required: &bool, red: &bool) -> bool {
    !*required || *red
}
fn review_allowed(implementation: &bool, green: &bool) -> bool {
    *implementation && *green
}
fn delivery_allowed(review: &bool, clean: &bool, ci_hold: &bool) -> bool {
    *review && *clean && !*ci_hold
}

mapping! { StartToRedRequired: WorkflowStart.kind => RedRequired.value using requires_red; }
mapping! { RedToImplementation: (RedRequired.value, RedEvidence.observed) => ImplementationAllowed.value using implementation_allowed; }
mapping! { GreenToReview: (ImplementationAllowed.value, GreenEvidence.observed) => ReviewAllowed.value using review_allowed; }
mapping! { ReviewToDelivery: (ReviewAllowed.value, ReviewEvidence.clean, CiRecoveryHold.active) => DeliveryAllowed.value using delivery_allowed; }

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ModelGate {
    red_required: bool,
    implementation_allowed: bool,
    review_allowed: bool,
    delivery_allowed: bool,
}

fn model_gate(
    kind: ChangeKind,
    red_observed: bool,
    green_observed: bool,
    clean_review: bool,
    ci_hold_active: bool,
) -> ModelGate {
    let start = WorkflowStart::model_builder().kind(kind).build();
    let red = RedEvidence::model_builder().observed(red_observed).build();
    let green = GreenEvidence::model_builder()
        .observed(green_observed)
        .build();
    let review = ReviewEvidence::model_builder().clean(clean_review).build();
    let ci_hold = CiRecoveryHold::model_builder()
        .active(ci_hold_active)
        .build();
    let required = RedRequired::model_builder()
        .value(StartToRedRequired::apply(start.as_ref()))
        .build();
    let implementation = ImplementationAllowed::model_builder()
        .value(RedToImplementation::apply((
            required.as_ref(),
            red.as_ref(),
        )))
        .build();
    let review_allowed = ReviewAllowed::model_builder()
        .value(GreenToReview::apply((
            implementation.as_ref(),
            green.as_ref(),
        )))
        .build();
    let delivery = DeliveryAllowed::model_builder()
        .value(ReviewToDelivery::apply((
            review_allowed.as_ref(),
            review.as_ref(),
            ci_hold.as_ref(),
        )))
        .build();
    let required = required.into_inner();
    let implementation = implementation.into_inner();
    let review_allowed = review_allowed.into_inner();
    let delivery = delivery.into_inner();
    ModelGate {
        red_required: required.value,
        implementation_allowed: implementation.value,
        review_allowed: review_allowed.value,
        delivery_allowed: delivery.value,
    }
}

#[cfg(test)]
pub fn check_model() -> Result<eventcore::model::CheckReport, eventcore::model::CheckError> {
    eventcore::model::check()
}

#[cfg(test)]
mod tests {
    use super::{
        check_model, start_at, status_at, transition_at, ChangeKind, Phase, Workflow, WorkflowError,
    };
    use std::process::Command;
    use tempfile::TempDir;

    #[test]
    fn production_change_requires_red_green_review_and_delivery_evidence() {
        let mut workflow = Workflow::start(ChangeKind::Production);

        assert_eq!(
            workflow.authorize_implementation(),
            Err(WorkflowError::RedEvidenceRequired)
        );
        workflow.record_red().expect("record failing focused test");
        workflow
            .authorize_implementation()
            .expect("authorize implementation after red evidence");
        assert_eq!(
            workflow.authorize_review(),
            Err(WorkflowError::GreenEvidenceRequired)
        );
        workflow
            .record_green()
            .expect("record passing focused test");
        workflow
            .authorize_review()
            .expect("authorize review after green evidence");
        assert_eq!(
            workflow.authorize_delivery(false),
            Err(WorkflowError::ReviewEvidenceRequired)
        );
        workflow
            .record_clean_review()
            .expect("record clean final review");
        workflow
            .authorize_delivery(false)
            .expect("authorize delivery after clean review");
    }

    #[test]
    fn exempt_change_skips_red_but_requires_green_and_review_evidence() {
        let mut workflow = Workflow::start(ChangeKind::Exempt);

        assert_eq!(
            workflow.authorize_review(),
            Err(WorkflowError::GreenEvidenceRequired)
        );
        workflow
            .record_green()
            .expect("record focused exempt verification");
        workflow
            .authorize_review()
            .expect("authorize review after exempt verification");
        assert_eq!(
            workflow.authorize_delivery(false),
            Err(WorkflowError::ReviewEvidenceRequired)
        );
        workflow
            .record_clean_review()
            .expect("record exempt final review");
        workflow
            .authorize_delivery(false)
            .expect("exempt change delivery after review");
    }

    #[test]
    fn legacy_exempt_lifecycle_can_resume_with_green_evidence_and_reach_delivery() {
        let mut workflow = Workflow {
            phase: Phase::Exempt,
            change_kind: ChangeKind::Exempt,
            red_observed: false,
            green_observed: false,
            clean_review_observed: false,
        };

        workflow
            .record_green()
            .expect("legacy exempt lifecycle accepts successful verification");
        workflow.authorize_review().expect("authorize review");
        workflow.record_clean_review().expect("record review");
        workflow.authorize_delivery(false).expect("deliver");
    }

    #[test]
    fn abandoned_lifecycle_is_terminal_and_allows_the_next_lifecycle_to_start() {
        let repository = TempDir::new().expect("temporary repository");
        assert!(Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(repository.path())
            .status()
            .expect("initialize repository")
            .success());

        start_at(repository.path(), ChangeKind::Production).expect("start lifecycle");
        transition_at(repository.path(), "abandon").expect("abandon lifecycle");
        let restarted = start_at(repository.path(), ChangeKind::Production)
            .expect("start replacement lifecycle");
        assert_eq!(restarted.phase_name(), "awaiting_red");
    }

    #[test]
    fn eventcore_model_verifies_workflow_gates() {
        let report = check_model().expect("model check completes");

        assert_eq!(report.status, eventcore::model::CheckStatus::Verified);
    }

    #[test]
    fn checked_delivery_gate_keeps_a_ci_recovery_hold_closed() {
        let mut workflow = Workflow::start(ChangeKind::Production);
        workflow.record_red().expect("red evidence");
        workflow
            .authorize_implementation()
            .expect("implementation authorization");
        workflow.record_green().expect("green evidence");
        workflow.authorize_review().expect("review authorization");
        workflow.record_clean_review().expect("clean review");

        assert_eq!(
            workflow.authorize_delivery(true),
            Err(WorkflowError::DeliveryEvidenceRequired)
        );
        workflow
            .authorize_delivery(false)
            .expect("delivery after CI recovery is released");
    }

    #[test]
    fn lifecycle_state_survives_an_mcp_restart_and_is_shared_by_worktrees() {
        let repository = TempDir::new().expect("temporary repository");
        assert!(Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(repository.path())
            .status()
            .expect("initialize repository")
            .success());
        let linked = TempDir::new().expect("linked worktree path");
        assert!(Command::new("git")
            .args(["worktree", "add", "-b", "workflow-linked"])
            .arg(linked.path())
            .current_dir(repository.path())
            .status()
            .expect("create linked worktree")
            .success());

        start_at(repository.path(), ChangeKind::Production).expect("start lifecycle");
        transition_at(repository.path(), "record_red").expect("record red evidence");

        assert_eq!(
            status_at(linked.path())
                .expect("read shared lifecycle")
                .phase_name(),
            "awaiting_implementation"
        );
    }
}
