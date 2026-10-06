//! Durable evidence-backed rejection reuse. Identity is an exact finding/lens
//! pair; no wording, semantic similarity, or caller preference closes a finding.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ResolutionScope {
    pub project_root: String,
    pub baseline_commit: String,
    pub diff_hash: String,
    pub changed_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct VerifierProvenance {
    pub assignment_id: String,
    pub subagent_key: String,
    pub model_role: String,
    pub fresh_context: bool,
    pub closed_after_result: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ResolutionEvidence {
    #[serde(default)]
    pub outcome: ResolutionOutcome,
    pub resolution_id: String,
    pub finding_id: String,
    pub lens: String,
    pub rationale: String,
    pub evidence_checked: String,
    pub assumptions: Vec<String>,
    pub verifier: VerifierProvenance,
    pub scope: ResolutionScope,
    /// Exact relative paths and Git blob identities, including all source on
    /// which the rejection depends. Empty evidence remains historical only.
    pub dependency_blobs: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ResolutionOutcome {
    #[default]
    Rejected,
    Reopened,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ReopenReason {
    ContradictoryEvidence,
    RelevantChange,
    IncompletePriorVerification,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReopenEvidence {
    pub resolution_id: String,
    pub reason: ReopenReason,
    pub explanation: String,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReuseDecision {
    NoResolution,
    Reuse { resolution_id: String },
    VerifyReopening { resolution_id: String },
    VerifyDependencyChange { resolution_id: String },
}

fn nonblank(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_oid(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_dependency(path: &str, identity: &str) -> bool {
    let Some((mode, oid)) = identity.split_once(':') else {
        return false;
    };
    safe_dependency_path(path) && matches!(mode, "100644" | "100755" | "120000") && valid_oid(oid)
}

fn safe_dependency_path(value: &str) -> bool {
    !value.is_empty()
        && std::path::Path::new(value).components().all(
            |component| matches!(component, std::path::Component::Normal(part) if part != ".git"),
        )
}

pub(crate) fn capture(mut evidence: ResolutionEvidence) -> Result<ResolutionEvidence, String> {
    if [
        &evidence.finding_id,
        &evidence.lens,
        &evidence.rationale,
        &evidence.evidence_checked,
        &evidence.verifier.assignment_id,
        &evidence.verifier.subagent_key,
        &evidence.verifier.model_role,
        &evidence.scope.project_root,
        &evidence.scope.diff_hash,
    ]
    .iter()
    .any(|value| !nonblank(value))
        || !valid_oid(&evidence.scope.baseline_commit)
        || !evidence.verifier.fresh_context
        || !evidence.verifier.closed_after_result
        || evidence.assumptions.iter().any(|value| !nonblank(value))
        || evidence.dependency_blobs.len() > 128
        || evidence
            .dependency_blobs
            .iter()
            .any(|(path, identity)| !valid_dependency(path, identity))
    {
        return Err("resolution_evidence_invalid=true".to_string());
    }
    // Length framing is deterministic across processes/releases and does not
    // turn natural-language wording into finding identity.
    let parts = [
        &evidence.verifier.subagent_key,
        &evidence.verifier.assignment_id,
        &evidence.lens,
        &evidence.finding_id,
    ];
    evidence.resolution_id = format!(
        "resolution-v1:{}",
        parts
            .iter()
            .map(|part| format!("{}:{part}", part.len()))
            .collect::<Vec<_>>()
            .join(":")
    );
    Ok(evidence)
}

pub(crate) fn decide(
    history: &[ResolutionEvidence],
    finding_id: &str,
    lens: &str,
    scope: &ResolutionScope,
    dependencies: &BTreeMap<String, String>,
    reopening: Option<&ReopenEvidence>,
) -> Result<ReuseDecision, String> {
    decide_inner(
        history,
        finding_id,
        lens,
        scope,
        dependencies,
        reopening,
        false,
    )
}

/// Carried history is not a new reviewer challenge. Actual changed source
/// dependencies require fresh adjudication without inventing caller evidence.
pub(crate) fn decide_carried(
    history: &[ResolutionEvidence],
    finding_id: &str,
    lens: &str,
    scope: &ResolutionScope,
    dependencies: &BTreeMap<String, String>,
    reopening: Option<&ReopenEvidence>,
) -> Result<ReuseDecision, String> {
    decide_inner(
        history,
        finding_id,
        lens,
        scope,
        dependencies,
        reopening,
        true,
    )
}

fn decide_inner(
    history: &[ResolutionEvidence],
    finding_id: &str,
    lens: &str,
    scope: &ResolutionScope,
    dependencies: &BTreeMap<String, String>,
    reopening: Option<&ReopenEvidence>,
    reverify_carried: bool,
) -> Result<ReuseDecision, String> {
    let Some(prior) = history
        .iter()
        .rev()
        .find(|entry| entry.finding_id == finding_id && entry.lens == lens)
    else {
        return if reopening.is_some() {
            Err("resolution_reopen_unknown=true".to_string())
        } else {
            Ok(ReuseDecision::NoResolution)
        };
    };
    // Pre-upgrade or incomplete source evidence stays readable, never reusable.
    if prior.outcome == ResolutionOutcome::Reopened || prior.dependency_blobs.is_empty() {
        return Ok(ReuseDecision::NoResolution);
    }
    capture(prior.clone())?;
    let same_scope = prior.scope.project_root == scope.project_root
        && prior.scope.baseline_commit == scope.baseline_commit;
    let unchanged = same_scope
        && prior
            .dependency_blobs
            .iter()
            .all(|(path, identity)| dependencies.get(path) == Some(identity));
    if let Some(reopen) = reopening {
        if reopen.resolution_id != prior.resolution_id
            || !nonblank(&reopen.explanation)
            || !nonblank(&reopen.evidence_ref)
        {
            return Err("resolution_reopen_evidence_required=true".to_string());
        }
        if matches!(reopen.reason, ReopenReason::RelevantChange) && unchanged {
            return Err("resolution_reopen_relevant_change_not_observed=true".to_string());
        }
        return Ok(ReuseDecision::VerifyReopening {
            resolution_id: prior.resolution_id.clone(),
        });
    }
    if reverify_carried
        && same_scope
        && !unchanged
        && prior
            .dependency_blobs
            .keys()
            .all(|path| dependencies.contains_key(path))
    {
        return Ok(ReuseDecision::VerifyDependencyChange {
            resolution_id: prior.resolution_id.clone(),
        });
    }
    if unchanged {
        Ok(ReuseDecision::Reuse {
            resolution_id: prior.resolution_id.clone(),
        })
    } else {
        Err("resolution_reopen_evidence_required=true dependency_or_scope_changed=true".to_string())
    }
}

/// Imperative boundary observation; the caller records this in its intent.
pub(crate) fn observe_dependencies(
    project_root: &str,
    paths: &[String],
) -> Result<BTreeMap<String, String>, String> {
    use std::io::Write;
    use std::os::unix::{ffi::OsStrExt, fs::PermissionsExt};
    use std::process::{Command, Stdio};
    let mut observed = BTreeMap::new();
    if paths.is_empty() {
        return Ok(observed);
    }
    if paths.len() > 4096 || paths.iter().any(|path| !safe_dependency_path(path)) {
        return Err("resolution_dependency_path_invalid=true".to_string());
    }
    let root = std::fs::canonicalize(project_root)
        .map_err(|error| format!("resolution_dependency_root_unavailable source={error}"))?;
    let mut read_bytes = 0_u64;
    for relative in paths {
        if observed.contains_key(relative) {
            continue;
        }
        let absolute = root.join(relative);
        let metadata = match std::fs::symlink_metadata(&absolute) {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("resolution_dependency_unavailable source={error}")),
        };
        let parent = absolute
            .parent()
            .and_then(|parent| std::fs::canonicalize(parent).ok())
            .ok_or_else(|| "resolution_dependency_parent_unavailable=true".to_string())?;
        let safe_parent = parent.strip_prefix(&root).ok().is_some_and(|path| {
            path.components().all(|component| matches!(component, std::path::Component::Normal(part) if part != ".git"))
        });
        if !safe_parent {
            return Err("resolution_dependency_path_escape=true".to_string());
        }
        if !(metadata.is_file() || metadata.file_type().is_symlink()) {
            return Err("resolution_dependency_file_type_unsupported=true".to_string());
        }
        read_bytes = read_bytes.saturating_add(metadata.len());
        if metadata.len() > 16 * 1024 * 1024 || read_bytes > 128 * 1024 * 1024 {
            return Err("resolution_dependency_evidence_too_large=true".to_string());
        }
        let (mode, bytes) = if metadata.file_type().is_symlink() {
            let target = std::fs::read_link(&absolute)
                .map_err(|error| format!("resolution_dependency_read_failed source={error}"))?;
            ("120000", target.as_os_str().as_bytes().to_vec())
        } else {
            let mode = if metadata.permissions().mode() & 0o100 == 0 {
                "100644"
            } else {
                "100755"
            };
            (
                mode,
                std::fs::read(&absolute)
                    .map_err(|error| format!("resolution_dependency_read_failed source={error}"))?,
            )
        };
        let mut child = Command::new("git")
            .current_dir(&root)
            .args(["hash-object", "--stdin"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("resolution_dependency_git_failed source={error}"))?;
        child
            .stdin
            .take()
            .ok_or_else(|| "resolution_dependency_git_stdin_missing=true".to_string())?
            .write_all(&bytes)
            .map_err(|error| format!("resolution_dependency_git_write_failed source={error}"))?;
        let result = child
            .wait_with_output()
            .map_err(|error| format!("resolution_dependency_git_wait_failed source={error}"))?;
        let oid = String::from_utf8(result.stdout)
            .map_err(|_| "resolution_dependency_git_output_invalid=true".to_string())?;
        if !result.status.success() || !valid_oid(oid.trim()) {
            return Err("resolution_dependency_git_output_invalid=true".to_string());
        }
        observed.insert(relative.clone(), format!("{mode}:{}", oid.trim()));
    }
    Ok(observed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rejection() -> ResolutionEvidence {
        ResolutionEvidence {
            outcome: ResolutionOutcome::Rejected,
            resolution_id: String::new(),
            finding_id: "guard-1".to_string(),
            lens: "production-risk-footguns".to_string(),
            rationale: "The alleged unbounded read is bounded before allocation.".to_string(),
            evidence_checked: "src/read.rs: input length guard and oversized-input regression"
                .to_string(),
            assumptions: vec!["The public reader is the only input boundary.".to_string()],
            verifier: VerifierProvenance {
                assignment_id: "assignment-7".to_string(),
                subagent_key: "review:2:verifier".to_string(),
                model_role: "verifier".to_string(),
                fresh_context: true,
                closed_after_result: true,
            },
            scope: ResolutionScope {
                project_root: "/repo".to_string(),
                baseline_commit: "a".repeat(40),
                diff_hash: "scope-1".to_string(),
                changed_files: vec!["src/read.rs".to_string()],
            },
            dependency_blobs: BTreeMap::from([(
                "src/read.rs".to_string(),
                format!("100644:{}", "b".repeat(40)),
            )]),
        }
    }

    #[test]
    fn capture_retains_actual_checked_evidence_and_deterministic_provenance_identity() {
        let first = capture(rejection()).expect("valid independently verified rejection");
        let second = capture(rejection()).expect("same verification");
        assert!(!first.resolution_id.is_empty());
        assert_eq!(first.resolution_id, second.resolution_id);
        assert_eq!(
            first.evidence_checked,
            "src/read.rs: input length guard and oversized-input regression"
        );
        assert_eq!(first.verifier.assignment_id, "assignment-7");
        let encoded = serde_json::to_string(&first).expect("serialize");
        assert_eq!(
            serde_json::from_str::<ResolutionEvidence>(&encoded).expect("replay"),
            first
        );
    }

    #[test]
    fn capture_rejects_missing_evidence_or_unclosed_nonfresh_verifiers() {
        for field in ["rationale", "evidence", "assignment", "fresh", "closed"] {
            let mut supplied = rejection();
            match field {
                "rationale" => supplied.rationale.clear(),
                "evidence" => supplied.evidence_checked.clear(),
                "assignment" => supplied.verifier.assignment_id.clear(),
                "fresh" => supplied.verifier.fresh_context = false,
                "closed" => supplied.verifier.closed_after_result = false,
                _ => unreachable!(),
            }
            assert!(capture(supplied).is_err(), "accepted missing {field}");
        }
    }

    #[test]
    fn reuse_requires_exact_identity_and_scope_and_dependency_blobs() {
        let retained = capture(rejection()).expect("resolution");
        let history = vec![retained.clone()];
        assert_eq!(
            decide(
                &history,
                "guard-1",
                "production-risk-footguns",
                &retained.scope,
                &retained.dependency_blobs,
                None
            )
            .expect("decision"),
            ReuseDecision::Reuse {
                resolution_id: retained.resolution_id.clone()
            }
        );
        for (id, lens) in [
            ("guard-2", "production-risk-footguns"),
            ("guard-1", "security-safety"),
        ] {
            assert_eq!(
                decide(
                    &history,
                    id,
                    lens,
                    &retained.scope,
                    &retained.dependency_blobs,
                    None
                )
                .expect("new allegation"),
                ReuseDecision::NoResolution
            );
        }
        let mut changed = retained.dependency_blobs.clone();
        changed.insert(
            "src/read.rs".to_string(),
            format!("100644:{}", "c".repeat(40)),
        );
        assert!(decide(
            &history,
            "guard-1",
            "production-risk-footguns",
            &retained.scope,
            &changed,
            None
        )
        .is_err());
        let mut wrong_scope = retained.scope.clone();
        wrong_scope.baseline_commit = "d".repeat(40);
        assert!(decide(
            &history,
            "guard-1",
            "production-risk-footguns",
            &wrong_scope,
            &retained.dependency_blobs,
            None
        )
        .is_err());
    }

    #[test]
    fn explicit_contradiction_requests_independent_verification_never_closes_itself() {
        let retained = capture(rejection()).expect("resolution");
        let reopen = ReopenEvidence {
            resolution_id: retained.resolution_id.clone(),
            reason: ReopenReason::ContradictoryEvidence,
            explanation: "A second public reader bypasses the guard.".to_string(),
            evidence_ref: "src/alternate.rs: unguarded read".to_string(),
        };
        assert_eq!(
            decide(
                std::slice::from_ref(&retained),
                "guard-1",
                "production-risk-footguns",
                &retained.scope,
                &retained.dependency_blobs,
                Some(&reopen)
            )
            .expect("independent verification"),
            ReuseDecision::VerifyReopening {
                resolution_id: retained.resolution_id.clone()
            }
        );
        let mut blank = reopen.clone();
        blank.explanation.clear();
        assert!(decide(
            std::slice::from_ref(&retained),
            "guard-1",
            "production-risk-footguns",
            &retained.scope,
            &retained.dependency_blobs,
            Some(&blank)
        )
        .is_err());
        let mut invented = reopen;
        invented.resolution_id = "invented".to_string();
        assert!(decide(
            std::slice::from_ref(&retained),
            "guard-1",
            "production-risk-footguns",
            &retained.scope,
            &retained.dependency_blobs,
            Some(&invented)
        )
        .is_err());
    }

    #[test]
    fn relevant_change_requires_changed_dependency_and_old_records_without_blobs_are_historical() {
        let retained = capture(rejection()).expect("resolution");
        let reopen = ReopenEvidence {
            resolution_id: retained.resolution_id.clone(),
            reason: ReopenReason::RelevantChange,
            explanation: "The guard was removed.".to_string(),
            evidence_ref: "changed src/read.rs".to_string(),
        };
        assert!(decide(
            std::slice::from_ref(&retained),
            "guard-1",
            "production-risk-footguns",
            &retained.scope,
            &retained.dependency_blobs,
            Some(&reopen)
        )
        .is_err());
        let mut changed = retained.dependency_blobs.clone();
        changed.insert(
            "src/read.rs".to_string(),
            format!("100644:{}", "c".repeat(40)),
        );
        assert!(matches!(
            decide(
                std::slice::from_ref(&retained),
                "guard-1",
                "production-risk-footguns",
                &retained.scope,
                &changed,
                Some(&reopen)
            )
            .expect("new independent verification"),
            ReuseDecision::VerifyReopening { .. }
        ));
        let mut historical = retained.clone();
        historical.dependency_blobs.clear();
        assert_eq!(
            decide(
                &[historical],
                "guard-1",
                "production-risk-footguns",
                &retained.scope,
                &retained.dependency_blobs,
                None
            )
            .expect("no reusable source evidence"),
            ReuseDecision::NoResolution
        );
    }
    #[test]
    fn unrelated_documentation_change_reuses_the_same_verified_source_dependencies() {
        let retained = capture(rejection()).expect("resolution");
        let mut docs_only = retained.scope.clone();
        docs_only.diff_hash = "scope-with-readme-change".to_string();
        docs_only.changed_files.push("README.md".to_string());
        assert!(matches!(
            decide(
                std::slice::from_ref(&retained),
                "guard-1",
                "production-risk-footguns",
                &docs_only,
                &retained.dependency_blobs,
                None
            )
            .expect("unchanged dependency evidence"),
            ReuseDecision::Reuse { .. }
        ));
        let mut mode_change = retained.dependency_blobs.clone();
        mode_change.insert(
            "src/read.rs".to_string(),
            format!("100755:{}", "b".repeat(40)),
        );
        assert!(decide(
            std::slice::from_ref(&retained),
            "guard-1",
            "production-risk-footguns",
            &docs_only,
            &mode_change,
            None
        )
        .is_err());
    }

    #[test]
    fn dependency_observer_reads_real_source_without_writing_git_and_rejects_escape_paths() {
        use std::{fs, process::Command};
        let repo = tempfile::tempdir().expect("fixture repository");
        assert!(Command::new("git")
            .args(["init", "-q"])
            .arg(repo.path())
            .status()
            .expect("git init")
            .success());
        fs::write(repo.path().join("source.rs"), b"guarded source\n").expect("source");
        let root = repo.path().to_str().expect("root");
        let paths = vec!["source.rs".to_string()];
        let first = observe_dependencies(root, &paths).expect("observe source");
        assert!(first["source.rs"].starts_with("100644:"));
        fs::write(repo.path().join("README.md"), b"new documentation\n").expect("docs");
        assert_eq!(
            observe_dependencies(root, &paths).expect("observe unchanged source"),
            first
        );
        fs::write(repo.path().join("source.rs"), b"unguarded source\n").expect("changed source");
        assert_ne!(
            observe_dependencies(root, &paths).expect("observe changed source"),
            first
        );
        for escape in ["../outside", ".git/config", "/etc/passwd"] {
            assert!(
                observe_dependencies(root, &[escape.to_string()]).is_err(),
                "accepted {escape}"
            );
        }
        assert!(Command::new("git")
            .current_dir(repo.path())
            .args(["rev-list", "--all"])
            .output()
            .expect("refs")
            .stdout
            .is_empty());
    }

    #[test]
    fn carried_reverification_requires_actual_complete_changed_dependencies() {
        let prior = capture(rejection()).expect("rejection");
        let mut changed = prior.dependency_blobs.clone();
        let path = changed.keys().next().expect("dependency").clone();
        changed.insert(path, format!("100644:{}", "c".repeat(40)));
        assert!(matches!(
            decide_carried(
                std::slice::from_ref(&prior),
                &prior.finding_id,
                &prior.lens,
                &prior.scope,
                &changed,
                None
            )
            .expect("reverify"),
            ReuseDecision::VerifyDependencyChange { .. }
        ));
        assert!(decide(
            std::slice::from_ref(&prior),
            &prior.finding_id,
            &prior.lens,
            &prior.scope,
            &changed,
            None
        )
        .is_err());
        assert!(decide_carried(
            std::slice::from_ref(&prior),
            &prior.finding_id,
            &prior.lens,
            &prior.scope,
            &BTreeMap::new(),
            None
        )
        .is_err());
        let mut foreign_scope = prior.scope.clone();
        foreign_scope.baseline_commit = "c".repeat(40);
        assert!(decide_carried(
            std::slice::from_ref(&prior),
            &prior.finding_id,
            &prior.lens,
            &foreign_scope,
            &changed,
            None
        )
        .is_err());
    }

    #[test]
    fn reused_allegation_stays_nonclean_but_does_not_request_verification_or_escalation() {
        let retained = capture(rejection()).expect("resolution");
        let finding = serde_json::json!({"id":"guard-1","finding_id":"guard-1","lens":"production-risk-footguns",
            "severity":"MAJOR","security_impact":"major","safety_impact":"none","likelihood":"likely","causality":"caused",
            "message":"Same allegation phrased differently"});
        let mut filtered: crate::FilteredReviewFindings = serde_json::from_value(serde_json::json!({
            "actionable":[finding.clone()],"routed":[],"already_tracked":[],"defended_or_accepted":[],"out_of_scope":[],
            "security_escalations_required":[finding],"follow_up_tickets_required":[],"malformed":[],"needs_human_decision":[],"clean":false,
            "transition":{"session_id":null,"iteration_index":null,"diff_hash":null,"expected_lenses":[],"expected_subagent_keys":[],"seen_subagent_keys":[],"complete_lens_set":true}
        })).expect("typed filter fixture");
        crate::apply_retained_resolution_evidence(
            std::slice::from_ref(&retained),
            &retained.scope,
            &retained.dependency_blobs,
            &std::collections::HashSet::from([crate::ReviewFindingKey {
                id: "guard-1".into(),
                lens: "production-risk-footguns".into(),
            }]),
            &mut filtered,
        )
        .expect("reuse");
        assert!(filtered.actionable.is_empty());
        assert!(filtered.security_escalations_required.is_empty());
        assert_eq!(filtered.already_tracked.len(), 1);
        assert!(!filtered.clean);
        assert!(crate::typed_verification_candidates(&[], None, &filtered).is_empty());
    }

    #[test]
    fn independently_confirmed_reopening_retires_the_older_rejection() {
        let rejected = capture(rejection()).expect("rejection");
        let mut reopened = rejected.clone();
        reopened.outcome = ResolutionOutcome::Reopened;
        reopened.verifier.assignment_id = "new-verifier-assignment".to_string();
        reopened.rationale = "The new evidence establishes a reachable unguarded path.".to_string();
        let reopened = capture(reopened).expect("adjudicated reopening");
        assert_eq!(
            decide(
                &[rejected.clone(), reopened],
                "guard-1",
                "production-risk-footguns",
                &rejected.scope,
                &rejected.dependency_blobs,
                None
            )
            .expect("prior closure no longer applicable"),
            ReuseDecision::NoResolution
        );
    }

    #[test]
    fn rejected_resolution_capture_is_durable_and_rejects_forged_dependency_evidence() {
        let retained = capture(rejection()).expect("fixture evidence");
        let finding: crate::ReviewFindingFacts = serde_json::from_value(serde_json::json!({
            "id":"guard-1","finding_id":"guard-1","lens":"production-risk-footguns","severity":"MAJOR",
            "security_impact":"major","safety_impact":"none","likelihood":"likely","causality":"caused","message":"allegation"
        })).expect("finding");
        let result: crate::AdvanceVerifierResultInput = serde_json::from_value(serde_json::json!({
            "assignment_id":"assignment-7","subagent_key":"review:2:verifier","model_role":"verifier","status":"verified",
            "caller_attestation":{"model_role":"verifier","fresh_context":true,"closed_after_result":true},
            "verdicts":[{"finding_id":"guard-1","lens":"production-risk-footguns","verdict":"rejected","severity":"MINOR",
                "causality":"incidental","causality_evidence":"Checked guard and regression","security_impact":"none","safety_impact":"none","rationale":"Guard disproves allegation",
                "dependency_blobs":retained.dependency_blobs,"assumptions":["single reader boundary"]}]
        })).expect("verifier");
        let scope = crate::ReviewScopeFacts {
            kind: crate::ReviewScopeKind::Base,
            review_lifecycle: crate::ReviewLifecycle::Unlanded,
            split_lineage: None,
            base: retained.scope.baseline_commit.clone(),
            baseline_commit: Some(retained.scope.baseline_commit.clone()),
            snapshot_commit: Some("c".repeat(40)),
            project_root: retained.scope.project_root.clone(),
            changed_files: retained.scope.changed_files.clone(),
            diff_hash: retained.scope.diff_hash.clone(),
        };
        let round: crate::ReviewFindingHistoryFacts = serde_json::from_value(serde_json::json!({
            "completed_iteration":2,"clean":false,"reset_reason":"findings_or_malformed_results","actionable_count":0,"routed_count":0,
            "already_tracked_count":0,"defended_or_accepted_count":0,"out_of_scope_count":0,"malformed_count":0,"needs_human_decision_count":0
        })).expect("legacy-compatible round");
        let mut history = vec![round.clone()];
        crate::capture_rejected_resolution_evidence(
            &mut history,
            &scope,
            Some(&result),
            std::slice::from_ref(&finding),
            &[],
            &retained.dependency_blobs,
        )
        .expect("actual rejection");
        let saved = history[0]
            .resolution_history
            .as_ref()
            .expect("durable resolution");
        assert_eq!(saved[0].evidence_checked, "Checked guard and regression");
        assert_eq!(saved[0].assumptions, ["single reader boundary"]);
        assert_eq!(saved[0].dependency_blobs, retained.dependency_blobs);
        let prior_resolution_id = saved[0].resolution_id.clone();
        let mut reopened_result = result.clone();
        reopened_result.verdicts[0].verdict = "confirmed".to_string();
        reopened_result.assignment_id = "new-independent-verifier".to_string();
        history.push(round.clone());
        crate::capture_rejected_resolution_evidence(&mut history, &scope, Some(&reopened_result), &[],
            &[serde_json::json!({"finding_id":"guard-1","lens":"production-risk-footguns","resolution_id":prior_resolution_id})],
            &retained.dependency_blobs).expect("adjudicated reopening");
        assert_eq!(
            history[1]
                .resolution_history
                .as_ref()
                .expect("durable reopened evidence")[0]
                .outcome,
            ResolutionOutcome::Reopened
        );
        let mut invalid = vec![round];
        assert!(crate::capture_rejected_resolution_evidence(
            &mut invalid,
            &scope,
            Some(&result),
            &[finding],
            &[],
            &BTreeMap::new()
        )
        .is_err());
        assert!(invalid[0].resolution_history.is_none());
    }
    #[test]
    fn bounded_round_pruning_retains_latest_resolution_and_reopening_tombstone() {
        let rejected = capture(rejection()).expect("rejection");
        let mut reopened = rejected.clone();
        reopened.outcome = ResolutionOutcome::Reopened;
        reopened.verifier.assignment_id = "independent-reopening".to_string();
        let reopened = capture(reopened).expect("reopened evidence");
        let mut active = rejected.clone();
        active.finding_id = "other-independent-finding".to_string();
        let active = capture(active).expect("separate rejection");
        let round: crate::ReviewFindingHistoryFacts = serde_json::from_value(serde_json::json!({
            "completed_iteration":1,"clean":false,"reset_reason":"findings_or_malformed_results","actionable_count":0,"routed_count":0,
            "already_tracked_count":0,"defended_or_accepted_count":0,"out_of_scope_count":0,"malformed_count":0,"needs_human_decision_count":0,
            "resolution_history":[rejected,active]
        })).expect("round");
        let mut history = vec![round];
        let filtered: crate::FilteredReviewFindings = serde_json::from_value(serde_json::json!({
            "actionable":[],"routed":[],"already_tracked":[],"defended_or_accepted":[],"out_of_scope":[],
            "security_escalations_required":[],"follow_up_tickets_required":[],"malformed":[],"needs_human_decision":[],"clean":true,
            "transition":{"session_id":null,"iteration_index":null,"diff_hash":null,"expected_lenses":[],"expected_subagent_keys":[],"seen_subagent_keys":[],"complete_lens_set":true}
        })).expect("filter");
        crate::append_typed_finding_history(&mut history, 2, &filtered, "none");
        history.last_mut().expect("new round").resolution_history = Some(vec![reopened.clone()]);
        for iteration in 3..=130 {
            crate::append_typed_finding_history(&mut history, iteration, &filtered, "none");
        }
        assert_eq!(history.len(), crate::MAX_RETAINED_HISTORY_ENTRIES);
        let replayed: Vec<crate::ReviewFindingHistoryFacts> =
            serde_json::from_value(serde_json::to_value(&history).expect("persist"))
                .expect("replay");
        let retained = crate::retained_resolutions(&replayed);
        assert_eq!(
            retained.len(),
            2,
            "latest distinct outcomes must survive round pruning"
        );
        assert!(
            retained.contains(&active),
            "active rejection evidence is unchanged"
        );
        assert!(
            retained.contains(&reopened),
            "reopening tombstone is unchanged"
        );
        assert_eq!(
            decide(
                &retained,
                &rejected.finding_id,
                &rejected.lens,
                &rejected.scope,
                &rejected.dependency_blobs,
                None
            )
            .expect("reopened"),
            ReuseDecision::NoResolution
        );
        let mut docs_scope = active.scope.clone();
        docs_scope.diff_hash = "unrelated-docs-after-130-rounds".to_string();
        docs_scope.changed_files.push("README.md".to_string());
        assert!(matches!(
            decide(
                &retained,
                &active.finding_id,
                &active.lens,
                &docs_scope,
                &active.dependency_blobs,
                None
            )
            .expect("unchanged source"),
            ReuseDecision::Reuse { .. }
        ));
    }
}
