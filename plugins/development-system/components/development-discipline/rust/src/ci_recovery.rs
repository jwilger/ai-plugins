//! Remote, workflow-owned CI recovery coordination.
//!
//! The authoritative state is one JSON document committed on the dedicated
//! `development-workflow` ref.  Every mutation fetches, checks its parent, and
//! publishes with Git's lease compare-and-swap; this is deliberately separate
//! from Tiber's optional task-board ref.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[cfg(test)]
use std::cell::RefCell;

const REMOTE_REF: &str = "refs/remotes/origin/development-workflow";
const REMOTE_HEAD: &str = "refs/heads/development-workflow";
const DOCUMENT: &str = "ci-recovery.json";
const LEASE_SECONDS: u64 = 60 * 60;
const MAX_TEXT_BYTES: usize = 16 * 1024;
const MAX_ATTEMPTS: usize = 3;

#[cfg(test)]
thread_local! {
    static TEST_SESSION: RefCell<Option<String>> = const { RefCell::new(None) };
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct Participant {
    host: String,
    session: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct Trigger {
    run_id: String,
    run_url: String,
    failed_sha: String,
    workflow: String,
    git_ref: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Assignment {
    id: String,
    owner_epoch: u64,
    assignee: Participant,
    capabilities: Vec<String>,
    scope: String,
    report: Option<Report>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Report {
    summary: String,
    evidence: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Diagnosis {
    job: String,
    step: String,
    log_evidence: String,
    cause: String,
    classification: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Action {
    kind: String,
    description: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Replacement {
    run_id: String,
    run_url: String,
    sha: String,
    status: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
struct ReleaseProof {
    replacement_run_id: String,
    replacement_run_url: String,
    sha: String,
    terminal_status: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct State {
    schema_version: u8,
    incident_id: String,
    state: String,
    epoch: u64,
    trigger: Trigger,
    #[serde(default)]
    triggers: Vec<Trigger>,
    owner: Participant,
    lease_expires_at: u64,
    #[serde(default)]
    participants: Vec<Participant>,
    #[serde(default)]
    assignments: Vec<Assignment>,
    #[serde(default)]
    diagnosis: Option<Diagnosis>,
    #[serde(default)]
    next_action: Option<Action>,
    #[serde(default)]
    replacement: Option<Replacement>,
    #[serde(default)]
    release_proof: Option<ReleaseProof>,
}

/// Dispatches the workflow CI-recovery MCP surface.
pub fn call(root: &Path, action: &str, arguments: &Value) -> Result<Value, String> {
    match action {
        "claim" => claim(root, arguments),
        "status" => status(root),
        "assert_owner" => assertion(root, arguments, false),
        "heartbeat" => assertion(root, arguments, true),
        "transfer" => transfer(root, arguments),
        "takeover" => takeover(root, arguments),
        "assign" => assign(root, arguments),
        "report" => report(root, arguments),
        "wait" => wait(root, arguments),
        "diagnose" => diagnose(root, arguments),
        "choose_action" => choose_action(root, arguments),
        "record_replacement" => record_replacement(root, arguments),
        "resolve" => resolve(root, arguments),
        _ => Err("development_workflow.ci_recovery_unknown_action".to_string()),
    }
}

/// This fetches the independent authority.  An unreachable origin is an error,
/// never an implicit release of an active CI hold.
pub fn active_hold(root: &Path) -> Result<bool, String> {
    if !has_origin(root) {
        return Ok(false);
    }
    Ok(load(root)?.1.is_some_and(|state| state.state != "resolved"))
}

fn claim(root: &Path, arguments: &Value) -> Result<Value, String> {
    let trigger = Trigger {
        run_id: text(arguments, "run_id")?,
        run_url: text(arguments, "run_url")?,
        failed_sha: text(arguments, "failed_sha")?,
        workflow: text(arguments, "workflow")?,
        git_ref: text(arguments, "git_ref")?,
    };
    let participant = participant()?;
    let result = mutate(root, |current, now| match current {
        Some(mut state) if state.state != "resolved" => {
            if !state.triggers.contains(&trigger) {
                let allowed_retry = state.replacement.as_ref().is_some_and(|replacement| {
                    replacement.status == "failed"
                        && replacement.run_id == trigger.run_id
                        && replacement.run_url == trigger.run_url
                        && replacement.sha == trigger.failed_sha
                });
                if !allowed_retry {
                    return Err(format!(
                        "development_workflow.ci_recovery_distinct_trigger active_incident_id={}",
                        state.incident_id
                    ));
                }
                state.trigger = trigger.clone();
                state.triggers.push(trigger.clone());
            }
            if !state.participants.contains(&participant) {
                state.participants.push(participant.clone());
            }
            let role = if state.owner == participant {
                "owner"
            } else {
                "waiting"
            };
            Ok((
                state.clone(),
                json!({"incident_id":state.incident_id,"state":state.state,"role":role,"epoch":state.epoch,"lease_expires_at":state.lease_expires_at}),
            ))
        }
        _ => {
            let state = State {
                schema_version: 1,
                incident_id: incident_id(&trigger.run_id),
                state: "diagnosing".to_string(),
                epoch: 1,
                trigger: trigger.clone(),
                triggers: vec![trigger.clone()],
                owner: participant.clone(),
                lease_expires_at: now.saturating_add(LEASE_SECONDS),
                participants: vec![participant.clone()],
                assignments: vec![],
                diagnosis: None,
                next_action: None,
                replacement: None,
                release_proof: None,
            };
            Ok((
                state.clone(),
                json!({"incident_id":state.incident_id,"state":state.state,"role":"owner","epoch":state.epoch,"lease_expires_at":state.lease_expires_at}),
            ))
        }
    });
    if result.is_err() {
        let _ = write_blocker(
            root,
            "development_workflow.ci_recovery_claim_failed",
            "retry the shared Development Workflow CI-recovery claim",
        );
    }
    result
}

fn status(root: &Path) -> Result<Value, String> {
    let (_, state) = load(root)?;
    state
        .map(status_value)
        .ok_or_else(|| "development_workflow.ci_recovery_incident_missing active=false".to_string())
}

fn assertion(root: &Path, arguments: &Value, renew: bool) -> Result<Value, String> {
    let caller = participant()?;
    let incident = text(arguments, "incident_id")?;
    let epoch = number(arguments, "epoch")?;
    mutate(root, |current, now| {
        let mut state = required_state(current)?;
        ensure_owner(&state, &incident, epoch, &caller)?;
        ensure_lease(&state, now)?;
        if renew {
            state.lease_expires_at = now.saturating_add(LEASE_SECONDS);
        }
        Ok((
            state.clone(),
            json!({"allowed":true,"incident_id":state.incident_id,"epoch":state.epoch,"lease_expires_at":state.lease_expires_at}),
        ))
    })
}

fn transfer(root: &Path, arguments: &Value) -> Result<Value, String> {
    let caller = participant()?;
    let incident = text(arguments, "incident_id")?;
    let epoch = number(arguments, "epoch")?;
    let recipient = named_participant(arguments, "to_host", "to_session")?;
    mutate(root, |current, now| {
        let mut state = required_state(current)?;
        ensure_owner(&state, &incident, epoch, &caller)?;
        ensure_open(&state)?;
        ensure_lease(&state, now)?;
        state.owner = recipient.clone();
        if !state.participants.contains(&recipient) {
            state.participants.push(recipient.clone());
        }
        state.epoch = state.epoch.saturating_add(1);
        state.lease_expires_at = now.saturating_add(LEASE_SECONDS);
        Ok((state.clone(), transfer_value(&state)))
    })
}

fn takeover(root: &Path, arguments: &Value) -> Result<Value, String> {
    let successor = participant()?;
    let incident = text(arguments, "incident_id")?;
    let epoch = number(arguments, "epoch")?;
    mutate(root, |current, now| {
        let mut state = required_state(current)?;
        ensure_epoch(&state, &incident, epoch)?;
        ensure_open(&state)?;
        if state.owner == successor {
            return Err("development_workflow.ci_recovery_already_owner".to_string());
        }
        if state.lease_expires_at > now {
            return Err(format!(
                "development_workflow.ci_recovery_lease_active expires_at={}",
                state.lease_expires_at
            ));
        }
        state.owner = successor.clone();
        if !state.participants.contains(&successor) {
            state.participants.push(successor.clone());
        }
        state.epoch = state.epoch.saturating_add(1);
        state.lease_expires_at = now.saturating_add(LEASE_SECONDS);
        Ok((state.clone(), transfer_value(&state)))
    })
}

fn assign(root: &Path, arguments: &Value) -> Result<Value, String> {
    let caller = participant()?;
    let incident = text(arguments, "incident_id")?;
    let epoch = number(arguments, "epoch")?;
    let assignee = named_participant(arguments, "to_host", "to_session")?;
    let scope = text(arguments, "scope")?;
    let capabilities = text(arguments, "capabilities")?
        .split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if capabilities.is_empty()
        || capabilities
            .iter()
            .any(|value| !matches!(value.as_str(), "inspect" | "reproduce" | "edit" | "test"))
    {
        return Err("development_workflow.ci_recovery_capability_invalid allowed=inspect,reproduce,edit,test".to_string());
    }
    mutate(root, |current, now| {
        let mut state = required_state(current)?;
        ensure_owner(&state, &incident, epoch, &caller)?;
        ensure_lease(&state, now)?;
        if !state.participants.contains(&assignee) {
            return Err("development_workflow.ci_recovery_assignee_not_joined".to_string());
        }
        let id = format!("a{}", state.assignments.len() + 1);
        state.assignments.push(Assignment {
            id: id.clone(),
            owner_epoch: state.epoch,
            assignee: assignee.clone(),
            capabilities: capabilities.clone(),
            scope: scope.clone(),
            report: None,
        });
        Ok((
            state.clone(),
            json!({"incident_id":state.incident_id,"assignment_id":id,"epoch":state.epoch}),
        ))
    })
}

fn report(root: &Path, arguments: &Value) -> Result<Value, String> {
    let caller = participant()?;
    let incident = text(arguments, "incident_id")?;
    let assignment_id = text(arguments, "assignment_id")?;
    let summary = text(arguments, "summary")?;
    let evidence = text(arguments, "evidence")?;
    mutate(root, |current, _| {
        let mut state = required_state(current)?;
        if state.incident_id != incident {
            return Err("development_workflow.ci_recovery_incident_mismatch".to_string());
        };
        let epoch = state.epoch;
        let assignment = state
            .assignments
            .iter_mut()
            .find(|item| item.id == assignment_id)
            .ok_or_else(|| "development_workflow.ci_recovery_assignment_missing".to_string())?;
        if assignment.owner_epoch != epoch {
            return Err("development_workflow.ci_recovery_assignment_stale".to_string());
        };
        if assignment.assignee != caller {
            return Err("development_workflow.ci_recovery_assignment_not_assignee".to_string());
        };
        assignment.report = Some(Report {
            summary: summary.clone(),
            evidence: evidence.clone(),
        });
        Ok((
            state.clone(),
            json!({"incident_id":state.incident_id,"assignment_id":assignment_id,"epoch":state.epoch}),
        ))
    })
}

fn wait(root: &Path, arguments: &Value) -> Result<Value, String> {
    let incident = text(arguments, "incident_id")?;
    let epoch = number(arguments, "epoch")?;
    let seconds = number(arguments, "timeout_seconds")?;
    if seconds > 60 {
        return Err(
            "development_workflow.ci_recovery_wait_timeout_invalid maximum_seconds=60".to_string(),
        );
    };
    let caller = participant()?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
    loop {
        let (_, state) = load(root)?;
        let state = required_state(state)?;
        if state.incident_id != incident {
            return Err("development_workflow.ci_recovery_incident_mismatch".to_string());
        };
        if !state.participants.contains(&caller) {
            return Err("development_workflow.ci_recovery_participant_required".to_string());
        };
        let reason = if state.epoch != epoch {
            Some("epoch-changed")
        } else if state.state == "resolved" {
            Some("resolved")
        } else if let Some(item) = state
            .assignments
            .iter()
            .find(|item| item.owner_epoch == state.epoch && item.assignee == caller)
        {
            return Ok(
                json!({"incident_id":state.incident_id,"state":state.state,"epoch":state.epoch,"wake_reason":"assignment","assignment_id":item.id}),
            );
        } else {
            None
        };
        if let Some(reason) = reason {
            return Ok(
                json!({"incident_id":state.incident_id,"state":state.state,"epoch":state.epoch,"wake_reason":reason,"assignment_id":null}),
            );
        };
        if std::time::Instant::now() >= deadline {
            return Ok(
                json!({"incident_id":state.incident_id,"state":state.state,"epoch":state.epoch,"wake_reason":"timeout","assignment_id":null}),
            );
        };
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
}

fn diagnose(root: &Path, arguments: &Value) -> Result<Value, String> {
    let caller = participant()?;
    let incident = text(arguments, "incident_id")?;
    let epoch = number(arguments, "epoch")?;
    let classification = choice(
        arguments,
        "classification",
        &["caused", "unrelated", "transient"],
    )?;
    let diagnosis = Diagnosis {
        job: text(arguments, "job")?,
        step: text(arguments, "step")?,
        log_evidence: text(arguments, "log_evidence")?,
        cause: text(arguments, "cause")?,
        classification,
    };
    mutate(root, |current, now| {
        let mut state = required_state(current)?;
        ensure_owner(&state, &incident, epoch, &caller)?;
        ensure_lease(&state, now)?;
        state.diagnosis = Some(diagnosis.clone());
        state.next_action = None;
        state.replacement = None;
        state.release_proof = None;
        state.state = "diagnosing".to_string();
        Ok((state.clone(), status_value(state)))
    })
}

fn choose_action(root: &Path, arguments: &Value) -> Result<Value, String> {
    let caller = participant()?;
    let incident = text(arguments, "incident_id")?;
    let epoch = number(arguments, "epoch")?;
    let kind = choice(arguments, "kind", &["repair", "rerun"])?;
    let description = text(arguments, "description")?;
    mutate(root, |current, now| {
        let mut state = required_state(current)?;
        ensure_owner(&state, &incident, epoch, &caller)?;
        ensure_lease(&state, now)?;
        let diagnosis = state
            .diagnosis
            .as_ref()
            .ok_or_else(|| "development_workflow.ci_recovery_diagnosis_required".to_string())?;
        if !matches!(
            (diagnosis.classification.as_str(), kind.as_str()),
            ("caused", "repair") | ("unrelated", "rerun") | ("transient", "rerun")
        ) {
            return Err("development_workflow.ci_recovery_action_conflicts".to_string());
        };
        state.next_action = Some(Action {
            kind: kind.clone(),
            description: description.clone(),
        });
        state.state = "action-selected".to_string();
        Ok((state.clone(), status_value(state)))
    })
}

fn record_replacement(root: &Path, arguments: &Value) -> Result<Value, String> {
    let caller = participant()?;
    let incident = text(arguments, "incident_id")?;
    let epoch = number(arguments, "epoch")?;
    let replacement = Replacement {
        run_id: text(arguments, "run_id")?,
        run_url: text(arguments, "run_url")?,
        sha: text(arguments, "sha")?,
        status: choice(arguments, "status", &["queued", "running", "failed"])?,
    };
    mutate(root, |current, now| {
        let mut state = required_state(current)?;
        ensure_owner(&state, &incident, epoch, &caller)?;
        ensure_lease(&state, now)?;
        let action = state
            .next_action
            .as_ref()
            .ok_or_else(|| "development_workflow.ci_recovery_next_action_required".to_string())?;
        if action.kind == "rerun" && replacement.sha != state.trigger.failed_sha {
            return Err("development_workflow.ci_recovery_rerun_sha_mismatch".to_string());
        };
        state.replacement = Some(replacement.clone());
        if replacement.status == "failed" {
            state.state = "diagnosing".to_string();
            state.diagnosis = None;
            state.next_action = None
        } else {
            state.state = "waiting-ci".to_string()
        };
        Ok((state.clone(), status_value(state)))
    })
}

fn resolve(root: &Path, arguments: &Value) -> Result<Value, String> {
    let caller = participant()?;
    let incident = text(arguments, "incident_id")?;
    let terminal = choice(arguments, "terminal_status", &["success"])?;
    let proof = ReleaseProof {
        replacement_run_id: text(arguments, "replacement_run_id")?,
        replacement_run_url: text(arguments, "replacement_run_url")?,
        sha: text(arguments, "sha")?,
        terminal_status: terminal,
    };
    mutate(root, |current, _| {
        let mut state = required_state(current)?;
        if state.incident_id != incident {
            return Err("development_workflow.ci_recovery_incident_mismatch".to_string());
        };
        if !state.participants.contains(&caller) {
            return Err("development_workflow.ci_recovery_participant_required".to_string());
        };
        let replacement = state
            .replacement
            .as_ref()
            .ok_or_else(|| "development_workflow.ci_recovery_replacement_required".to_string())?;
        if replacement.status == "failed" {
            return Err("development_workflow.ci_recovery_replacement_failed".to_string());
        };
        if replacement.run_id != proof.replacement_run_id
            || replacement.run_url != proof.replacement_run_url
            || replacement.sha != proof.sha
        {
            return Err("development_workflow.ci_recovery_release_proof_mismatch".to_string());
        };
        state.release_proof = Some(proof.clone());
        state.state = "resolved".to_string();
        Ok((state.clone(), status_value(state)))
    })
}

fn mutate(
    root: &Path,
    operation: impl Fn(Option<State>, u64) -> Result<(State, Value), String>,
) -> Result<Value, String> {
    let _lock = lock(root)?;
    for _ in 0..MAX_ATTEMPTS {
        let (parent, current) = load(root)?;
        let (now_state, result) = operation(current, now()?)?;
        match publish(root, parent.as_deref(), &now_state) {
            Ok(()) => {
                if now_state.state == "resolved" {
                    clear_blocker(root)?
                } else {
                    write_blocker(
                        root,
                        "development_workflow.ci_recovery_active",
                        "complete or transfer the shared Development Workflow CI recovery",
                    )?
                };
                return Ok(result);
            }
            Err(error) if error == "development_workflow.ci_recovery_version_conflict" => continue,
            Err(error) => return Err(error),
        }
    }
    Err("development_workflow.ci_recovery_version_conflict".to_string())
}

fn load(root: &Path) -> Result<(Option<String>, Option<State>), String> {
    git(root, &["remote", "get-url", "origin"])?;
    let fetch = Command::new("git")
        .args([
            "fetch",
            "--quiet",
            "origin",
            &format!("+{REMOTE_HEAD}:{REMOTE_REF}"),
        ])
        .current_dir(root)
        .output()
        .map_err(|error| format!("development_workflow.git_unavailable source={error}"))?;
    if !fetch.status.success() {
        let stderr = String::from_utf8_lossy(&fetch.stderr);
        if stderr.contains("couldn't find remote ref") {
            return Ok((None, None));
        }
        return Err(format!(
            "development_workflow.git_failed command=fetch origin {REMOTE_HEAD} stderr={}",
            stderr.trim()
        ));
    }
    let parent = git_optional(root, &["rev-parse", "--verify", REMOTE_REF])?
        .map(|value| value.trim().to_string());
    let Some(parent) = parent else {
        return Ok((None, None));
    };
    let raw = git(root, &["show", &format!("{parent}:{DOCUMENT}")])?;
    let state = serde_json::from_str(&raw)
        .map_err(|_| "development_workflow.ci_recovery_state_invalid".to_string())?;
    Ok((Some(parent), Some(state)))
}

fn has_origin(root: &Path) -> bool {
    Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(root)
        .output()
        .is_ok_and(|output| output.status.success())
}

fn publish(root: &Path, parent: Option<&str>, state: &State) -> Result<(), String> {
    let raw = serde_json::to_vec(state).map_err(|error| {
        format!("development_workflow.ci_recovery_encode_failed source={error}")
    })?;
    let blob = git_stdin(root, &["hash-object", "-w", "--stdin"], &raw)?;
    let tree = git_stdin(
        root,
        &["mktree"],
        format!("100644 blob {}\t{DOCUMENT}\n", blob.trim()).as_bytes(),
    )?;
    let mut args = vec![
        "commit-tree".to_string(),
        tree.trim().to_string(),
        "-m".to_string(),
        "development workflow CI recovery transaction".to_string(),
    ];
    if let Some(parent) = parent {
        args.push("-p".to_string());
        args.push(parent.to_string())
    };
    let commit = git_with_env(
        root,
        &args.iter().map(String::as_str).collect::<Vec<_>>(),
        &[
            ("GIT_AUTHOR_NAME", "Development Workflow"),
            ("GIT_AUTHOR_EMAIL", "development-workflow@local"),
            ("GIT_COMMITTER_NAME", "Development Workflow"),
            ("GIT_COMMITTER_EMAIL", "development-workflow@local"),
        ],
    )?;
    let lease = format!("--force-with-lease={REMOTE_HEAD}:{}", parent.unwrap_or(""));
    let result = run_git(
        root,
        &[
            "push",
            "origin",
            &format!("{}:{REMOTE_HEAD}", commit.trim()),
            &lease,
        ],
    );
    if result.is_err() {
        return Err("development_workflow.ci_recovery_version_conflict".to_string());
    }
    Ok(())
}

fn status_value(state: State) -> Value {
    json!({"schema_version":state.schema_version,"incident_id":state.incident_id,"state":state.state,"epoch":state.epoch,"lease_expires_at":state.lease_expires_at,"hold_released":state.state=="resolved","trigger_count":state.triggers.len(),"trigger":state.trigger,"triggers":state.triggers,"owner":state.owner,"participants":state.participants,"assignments":state.assignments,"diagnosis":state.diagnosis,"next_action":state.next_action,"replacement":state.replacement,"release_proof":state.release_proof})
}
fn transfer_value(state: &State) -> Value {
    json!({"incident_id":state.incident_id,"epoch":state.epoch,"lease_expires_at":state.lease_expires_at})
}
fn required_state(value: Option<State>) -> Result<State, String> {
    value
        .ok_or_else(|| "development_workflow.ci_recovery_incident_missing active=false".to_string())
}
fn ensure_open(state: &State) -> Result<(), String> {
    if state.state == "resolved" {
        Err("development_workflow.ci_recovery_resolved".to_string())
    } else {
        Ok(())
    }
}
fn ensure_epoch(state: &State, incident: &str, epoch: u64) -> Result<(), String> {
    if state.incident_id != incident {
        return Err("development_workflow.ci_recovery_incident_mismatch".to_string());
    }
    if state.epoch != epoch {
        return Err("development_workflow.ci_recovery_stale_epoch".to_string());
    }
    Ok(())
}
fn ensure_owner(
    state: &State,
    incident: &str,
    epoch: u64,
    caller: &Participant,
) -> Result<(), String> {
    ensure_epoch(state, incident, epoch)?;
    if &state.owner != caller {
        return Err("development_workflow.ci_recovery_not_owner".to_string());
    }
    Ok(())
}
fn ensure_lease(state: &State, current: u64) -> Result<(), String> {
    if state.lease_expires_at <= current {
        Err("development_workflow.ci_recovery_lease_expired".to_string())
    } else {
        Ok(())
    }
}
fn incident_id(run_id: &str) -> String {
    format!(
        "ci-{}",
        run_id
            .chars()
            .filter(|value| value.is_ascii_alphanumeric() || *value == '-')
            .collect::<String>()
    )
}
fn text(arguments: &Value, key: &str) -> Result<String, String> {
    let value = arguments
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    let lower = value.to_ascii_lowercase();
    if value.is_empty()
        || value.len() > MAX_TEXT_BYTES
        || value.chars().any(char::is_control)
        || [
            "ghp_",
            "github_pat_",
            "authorization:",
            "bearer ",
            "password=",
            "token=",
            "secret=",
            "-----begin private key",
        ]
        .iter()
        .any(|needle| lower.contains(needle))
    {
        return Err(format!(
            "development_workflow.ci_recovery_field_invalid field={key}"
        ));
    }
    Ok(value.to_string())
}
fn number(arguments: &Value, key: &str) -> Result<u64, String> {
    arguments
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("development_workflow.ci_recovery_field_invalid field={key}"))
}
fn choice(arguments: &Value, key: &str, allowed: &[&str]) -> Result<String, String> {
    let value = text(arguments, key)?;
    if allowed.contains(&value.as_str()) {
        Ok(value)
    } else {
        Err(format!(
            "development_workflow.ci_recovery_choice_invalid field={key}"
        ))
    }
}
fn participant() -> Result<Participant, String> {
    #[cfg(test)]
    if let Some(session) = TEST_SESSION.with(|slot| slot.borrow().clone()) {
        return named_participant(
            &json!({"host":hostname(),"session":session}),
            "host",
            "session",
        );
    }
    let session = std::env::var("DEVELOPMENT_WORKFLOW_SESSION")
        .or_else(|_| std::env::var("CODEX_SESSION_ID"))
        .or_else(|_| std::env::var("CLAUDE_SESSION_ID"))
        .map_err(|_| "development_workflow.ci_recovery_session_required".to_string())?;
    named_participant(
        &json!({"host":hostname(),"session":session}),
        "host",
        "session",
    )
}

#[cfg(test)]
fn with_test_session<T>(session: &str, operation: impl FnOnce() -> T) -> T {
    TEST_SESSION.with(|slot| {
        let previous = slot.replace(Some(session.to_string()));
        let result = operation();
        slot.replace(previous);
        result
    })
}
fn named_participant(
    arguments: &Value,
    host_key: &str,
    session_key: &str,
) -> Result<Participant, String> {
    Ok(Participant {
        host: text(arguments, host_key)?,
        session: text(arguments, session_key)?,
    })
}
fn hostname() -> String {
    std::env::var("HOSTNAME").unwrap_or_else(|_| "local".to_string())
}
fn now() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| format!("development_workflow.clock_failed source={error}"))
}
fn common_dir(root: &Path) -> Result<PathBuf, String> {
    let path = git(root, &["rev-parse", "--git-common-dir"])?;
    let path = PathBuf::from(path.trim());
    Ok(if path.is_absolute() {
        path
    } else {
        root.join(path)
    })
}
fn lock(root: &Path) -> Result<fs::File, String> {
    let directory = common_dir(root)?.join("development-discipline");
    fs::create_dir_all(&directory).map_err(|error| {
        format!("development_workflow.ci_recovery_lock_directory_failed source={error}")
    })?;
    let file = fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(directory.join("ci-recovery.lock"))
        .map_err(|error| {
            format!("development_workflow.ci_recovery_lock_open_failed source={error}")
        })?;
    file.lock_exclusive()
        .map_err(|error| format!("development_workflow.ci_recovery_lock_failed source={error}"))?;
    Ok(file)
}
fn blocker(root: &Path) -> Result<PathBuf, String> {
    Ok(common_dir(root)?
        .join("development-discipline")
        .join("workflow-blocker.json"))
}
fn write_blocker(root: &Path, error_code: &str, required_action: &str) -> Result<(), String> {
    let path = blocker(root)?;
    let parent = path.parent().expect("blocker parent");
    fs::create_dir_all(parent)
        .map_err(|error| format!("development_workflow.hold_directory_failed source={error}"))?;
    fs::write(path,json!({"schema_version":1,"kind":"ci_recovery","error_code":error_code,"required_action":required_action,"created_at":now()?}).to_string()).map_err(|error|format!("development_workflow.hold_write_failed source={error}"))
}
fn clear_blocker(root: &Path) -> Result<(), String> {
    let path = blocker(root)?;
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "development_workflow.hold_clear_failed source={error}"
        )),
    }
}
fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    run_git(root, args)
}
fn git_optional(root: &Path, args: &[&str]) -> Result<Option<String>, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("development_workflow.git_unavailable source={error}"))?;
    if output.status.success() {
        String::from_utf8(output.stdout)
            .map(Some)
            .map_err(|_| "development_workflow.git_output_invalid".to_string())
    } else {
        Ok(None)
    }
}
fn run_git(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("development_workflow.git_unavailable source={error}"))?;
    if !output.status.success() {
        return Err(format!(
            "development_workflow.git_failed command={} stderr={}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout)
        .map_err(|_| "development_workflow.git_output_invalid".to_string())
}
fn git_with_env(
    root: &Path,
    args: &[&str],
    environment: &[(&str, &str)],
) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .envs(environment.iter().copied())
        .current_dir(root)
        .output()
        .map_err(|error| format!("development_workflow.git_unavailable source={error}"))?;
    if !output.status.success() {
        return Err(format!(
            "development_workflow.git_failed command={} stderr={}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout)
        .map_err(|_| "development_workflow.git_output_invalid".to_string())
}
fn git_stdin(root: &Path, args: &[&str], input: &[u8]) -> Result<String, String> {
    let mut child = Command::new("git")
        .args(args)
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("development_workflow.git_unavailable source={error}"))?;
    child
        .stdin
        .as_mut()
        .expect("piped stdin")
        .write_all(input)
        .map_err(|error| format!("development_workflow.git_input_failed source={error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("development_workflow.git_wait_failed source={error}"))?;
    if !output.status.success() {
        return Err(format!(
            "development_workflow.git_failed command={} stderr={}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout)
        .map_err(|_| "development_workflow.git_output_invalid".to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        active_hold, call, load, publish, status, with_test_session, Participant, State, Trigger,
    };
    use serde_json::json;
    use std::{
        path::Path,
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };
    use tempfile::TempDir;

    fn git(root: &Path, arguments: &[&str]) {
        assert!(Command::new("git")
            .args(arguments)
            .current_dir(root)
            .status()
            .expect("git should run")
            .success());
    }

    fn fixture() -> (TempDir, TempDir) {
        let remote = TempDir::new().expect("bare remote");
        git(remote.path(), &["init", "--bare", "--quiet"]);
        let clone = TempDir::new().expect("clone");
        git(clone.path(), &["init", "--quiet"]);
        git(clone.path(), &["config", "user.name", "Workflow test"]);
        git(
            clone.path(),
            &["config", "user.email", "workflow@example.test"],
        );
        git(
            clone.path(),
            &[
                "remote",
                "add",
                "origin",
                remote.path().to_str().expect("path"),
            ],
        );
        (remote, clone)
    }

    fn state() -> State {
        State {
            schema_version: 1,
            incident_id: "ci-123".to_string(),
            state: "diagnosing".to_string(),
            epoch: 1,
            trigger: Trigger {
                run_id: "123".to_string(),
                run_url: "https://example.test/123".to_string(),
                failed_sha: "abc123".to_string(),
                workflow: "CI".to_string(),
                git_ref: "refs/heads/main".to_string(),
            },
            triggers: vec![],
            owner: Participant {
                host: "test".to_string(),
                session: "one".to_string(),
            },
            lease_expires_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_secs()
                + 60,
            participants: vec![],
            assignments: vec![],
            diagnosis: None,
            next_action: None,
            replacement: None,
            release_proof: None,
        }
    }

    #[test]
    fn active_ci_recovery_is_published_only_on_the_dedicated_workflow_ref() {
        let (remote, clone) = fixture();
        let state = state();
        publish(clone.path(), None, &state).expect("publish workflow state");

        let (_, loaded) = load(clone.path()).expect("load workflow state");
        assert_eq!(loaded.expect("state").incident_id, "ci-123");
        assert!(active_hold(clone.path()).expect("read active hold"));
        assert!(Command::new("git")
            .args(["show-ref", "--verify", "refs/heads/development-workflow"])
            .current_dir(remote.path())
            .status()
            .expect("inspect remote ref")
            .success());
        assert!(!Command::new("git")
            .args(["show-ref", "--verify", "refs/heads/tiber"])
            .current_dir(remote.path())
            .status()
            .expect("inspect unrelated ref")
            .success());
    }

    #[test]
    fn simultaneous_clone_claims_elect_one_owner_and_make_the_other_wait() {
        let (remote, first) = fixture();
        let second = TempDir::new().expect("second clone");
        git(
            second.path(),
            &[
                "clone",
                "--quiet",
                remote.path().to_str().expect("remote path"),
                ".",
            ],
        );
        let arguments = json!({
            "run_id": "456", "run_url": "https://example.test/456", "failed_sha": "def456",
            "workflow": "CI", "git_ref": "refs/heads/main"
        });

        let first_claim = with_test_session("first", || call(first.path(), "claim", &arguments))
            .expect("first claim");
        let second_claim = with_test_session("second", || call(second.path(), "claim", &arguments))
            .expect("second claim");

        assert_eq!(first_claim["role"], "owner");
        assert_eq!(second_claim["role"], "waiting");
        assert_eq!(second_claim["incident_id"], first_claim["incident_id"]);
        assert_eq!(
            status(second.path()).expect("status")["participants"]
                .as_array()
                .expect("participants")
                .len(),
            2
        );
    }
}
