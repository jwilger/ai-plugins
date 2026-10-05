//! Pure reporting over persisted round facts. Iteration counters and filtered
//! buckets cannot reconstruct raw allegations or completed reviewer rounds.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RoundEvidence {
    pub schema_version: u32,
    pub completed_lens_round: bool,
    pub expected_lenses: Vec<String>,
    pub completed_lenses: Vec<String>,
    pub scope: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior_scope: Option<Value>,
    pub scope_changed: bool,
    pub source_changed: bool,
    pub raw_findings: Vec<Value>,
    pub confirmed_findings: Vec<Value>,
    pub duplicate_findings: Vec<Value>,
    pub rejected_findings: Vec<Value>,
    pub reopened_findings: Vec<Value>,
    pub repair_verified_findings: Vec<Value>,
    pub verifier_evidence: Option<Value>,
}

const BUCKETS: [(&str, &str); 6] = [
    ("raw_findings", "raw_allegations"),
    ("confirmed_findings", "confirmed"),
    ("duplicate_findings", "duplicates"),
    ("rejected_findings", "rejected"),
    ("reopened_findings", "reopened"),
    ("repair_verified_findings", "repair_verified"),
];

pub(super) fn report(state: &Value) -> Result<Value, String> {
    analyze(state).map(|(report, _)| report)
}

pub(super) fn evidence(state: &Value, reference: &str) -> Result<Value, String> {
    analyze(state)?
        .1
        .remove(reference)
        .ok_or_else(|| "review_yield_evidence_not_found=true".to_string())
}

type EvidenceIndex = BTreeMap<String, Value>;
fn analyze(state: &Value) -> Result<(Value, EvidenceIndex), String> {
    let history = state
        .get("finding_history")
        .and_then(Value::as_array)
        .ok_or("review_yield_history_required=true")?;
    let session = state
        .get("session_id")
        .and_then(Value::as_str)
        .ok_or("review_yield_session_required=true")?;
    let omitted_prior_history_rows = history.iter().try_fold(0u64, |count, row| {
        let omitted = match row
            .get("omitted_prior_history_rows")
            .filter(|value| !value.is_null())
        {
            Some(value) => value
                .as_u64()
                .ok_or("review_yield_retention_metadata_invalid=true")?,
            None => 0,
        };
        count
            .checked_add(omitted)
            .ok_or("review_yield_retention_metadata_overflow=true")
    })?;
    let mut details = EvidenceIndex::new();
    let mut rounds = Vec::new();
    let mut complete = 0u64;
    let mut unavailable = 0u64;
    let mut totals = BTreeMap::<&str, u64>::new();
    let mut confirmed_identities = BTreeSet::new();
    for (_, metric) in BUCKETS {
        totals.insert(metric, 0);
    }
    totals.insert("new_confirmed", 0);
    for (index, row) in history.iter().enumerate() {
        let Some(raw) = row.get("round_evidence").filter(|value| !value.is_null()) else {
            unavailable += 1;
            rounds.push(json!({"history_index":index,"completed_iteration":row.get("completed_iteration"),"evidence_available":false,
                "completed_lens_round":null,"counts":null,"reason":"legacy_round_evidence_not_recorded"}));
            continue;
        };
        let round: RoundEvidence = serde_json::from_value(raw.clone()).map_err(|error| {
            format!("review_yield_round_invalid=true history_index={index} source={error}")
        })?;
        validate(&round)?;
        complete += u64::from(round.completed_lens_round);
        let mut counts = BTreeMap::<&str, u64>::new();
        let mut references = BTreeMap::<&str, Vec<String>>::new();
        for (bucket, metric) in BUCKETS {
            let entries = raw[bucket].as_array().expect("typed round evidence array");
            counts.insert(metric, entries.len() as u64);
            *totals.get_mut(metric).expect("metric initialized") += entries.len() as u64;
            let mut refs = Vec::new();
            for (finding_index, finding) in entries.iter().enumerate() {
                let mut detail = json!({"session_id":session,"history_index":index,
                    "completed_iteration":row.get("completed_iteration"),"category":bucket,
                    "finding_index":finding_index,"finding":finding,"scope":round.scope,
                    "scope_changed":round.scope_changed,"source_changed":round.source_changed,
                    "verifier_evidence":round.verifier_evidence});
                if let Some(prior_scope) = &round.prior_scope {
                    detail["prior_scope"] = prior_scope.clone();
                }
                refs.push(insert_evidence(&mut details, detail)?);
            }
            references.insert(bucket, refs);
        }
        let mut new_confirmed = 0u64;
        for finding in &round.confirmed_findings {
            let identity = (
                finding["lens"].as_str().unwrap_or_default().to_owned(),
                finding["id"].as_str().unwrap_or_default().to_owned(),
            );
            if confirmed_identities.insert(identity) {
                new_confirmed += 1;
            }
        }
        counts.insert("new_confirmed", new_confirmed);
        *totals.get_mut("new_confirmed").expect("metric initialized") += new_confirmed;
        let round_reference = insert_evidence(
            &mut details,
            json!({"session_id":session,"history_index":index,
            "completed_iteration":row.get("completed_iteration"),"round_evidence":round}),
        )?;
        rounds.push(json!({"history_index":index,"completed_iteration":row.get("completed_iteration"),
            "evidence_available":true,"completed_lens_round":round.completed_lens_round,
            "clean":round.completed_lens_round && round.raw_findings.is_empty()
                && row.get("clean").and_then(Value::as_bool)==Some(true)
                && row.get("reset_reason").and_then(Value::as_str).is_none_or(|reason|reason=="none"),
            "finding_free":round.raw_findings.is_empty(),"reset_reason":row.get("reset_reason"),
            "scope":round.scope,"prior_scope":round.prior_scope,"scope_changed":round.scope_changed,"source_changed":round.source_changed,
            "counts":counts,"evidence_ref":round_reference,"evidence_refs":references}));
    }
    // Missing or pruned facts cannot establish full-history totals or novelty.
    let full_history_available = unavailable == 0 && omitted_prior_history_rows == 0;
    let mut observed_totals = json!(totals);
    if !full_history_available {
        observed_totals["first_confirmed_in_observed_history"] =
            observed_totals["new_confirmed"].clone();
        observed_totals["new_confirmed"] = Value::Null;
        for round in &mut rounds {
            if let Some(counts) = round.get_mut("counts").and_then(Value::as_object_mut) {
                if let Some(observed) = counts.insert("new_confirmed".into(), Value::Null) {
                    counts.insert("first_confirmed_in_observed_history".into(), observed);
                }
            }
        }
    }
    Ok((
        json!({"schema_version":1,"session_id":session,
        "counts_available_for_full_history":full_history_available,
        "coverage":{
            "status":if omitted_prior_history_rows>0 {"retained_window_only"} else if unavailable>0 {"incomplete_legacy_evidence"} else {"complete"},
            "retained_history_rows":history.len(),"omitted_prior_history_rows":omitted_prior_history_rows,
            "legacy_rows_without_round_evidence":unavailable,
            "guidance":if full_history_available {"All retained review history has recorded round evidence."} else {"Use observed_totals only for recorded retained rows and final_review.evidence to inspect them. Full-history totals and finding novelty are unavailable from this state."}
        },
        "completed_lens_rounds":if full_history_available {json!(complete)} else {Value::Null},
        "observed_completed_lens_rounds":complete,
        "legacy_rows_without_round_evidence":unavailable,
        "totals":if full_history_available {json!(totals)} else {Value::Null},
        "observed_totals":observed_totals,"rounds":rounds,
        "interpretation":"Counts describe recorded allegations and adjudications; a clean round is not evidence of useless work. Pruned and missing legacy facts are unavailable, not zero."}),
        details,
    ))
}

fn validate(round: &RoundEvidence) -> Result<(), String> {
    if round.schema_version != 1 || !round.scope.is_object() {
        return Err("review_yield_round_schema_or_scope_invalid=true".into());
    }
    let expected = round.expected_lenses.iter().collect::<BTreeSet<_>>();
    let completed = round.completed_lenses.iter().collect::<BTreeSet<_>>();
    if expected.len() != round.expected_lenses.len()
        || completed.len() != round.completed_lenses.len()
        || !completed.is_subset(&expected)
        || (round.completed_lens_round && (expected.is_empty() || expected != completed))
    {
        return Err("review_yield_completed_lenses_invalid=true".into());
    }
    for finding in round
        .confirmed_findings
        .iter()
        .chain(&round.duplicate_findings)
        .chain(&round.rejected_findings)
        .chain(&round.reopened_findings)
        .chain(&round.repair_verified_findings)
    {
        if ["id", "lens"].iter().any(|key| {
            finding
                .get(key)
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
        }) {
            return Err("review_yield_outcome_finding_identity_required=true".into());
        }
    }
    if (!round.confirmed_findings.is_empty()
        || !round.rejected_findings.is_empty()
        || !round.repair_verified_findings.is_empty())
        && !round
            .verifier_evidence
            .as_ref()
            .is_some_and(Value::is_object)
    {
        return Err("review_yield_adjudication_evidence_required=true".into());
    }
    Ok(())
}

fn insert_evidence(index: &mut EvidenceIndex, detail: Value) -> Result<String, String> {
    // This is an opaque deterministic address, not an authorization signature.
    // Match the repository's existing stable storage fingerprint convention.
    let mut identity = detail.clone();
    // Position is a display hint, not part of the persisted round identity;
    // pruning an older retained row must not rename surviving evidence.
    if let Some(fields) = identity.as_object_mut() {
        fields.remove("history_index");
    }
    let bytes = serde_json::to_vec(&identity)
        .map_err(|error| format!("review_yield_encode_failed source={error}"))?;
    let mut hash = 0xcbf29ce484222325u64;
    for byte in &bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    let reference = format!("review-evidence-v1:{hash:016x}");
    if let Some(previous) = index.insert(reference.clone(), detail.clone()) {
        if previous != detail {
            return Err("review_yield_evidence_reference_collision=true".into());
        }
    }
    Ok(reference)
}
