//! Typed task projection and its user-facing Markdown renderer.
//!
//! `Task` is reconstructed exclusively by folding [`crate::events::TiberEvent`]
//! values. Markdown is emitted only at CLI, MCP-resource, and dashboard-facing
//! document boundaries; it is never parsed back into authoritative state.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Claim {
    pub host: String,
    pub session: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChecklistItem {
    pub checked: bool,
    pub text: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Subtask {
    pub id: String,
    pub checked: bool,
    pub title: String,
    pub after: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Note {
    pub date: String,
    pub text: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct FinalReviewState {
    pub reviews: Vec<FinalReviewRecord>,
    pub clean_reviews: Vec<FinalReviewRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_snapshot: Option<FinalReviewCompletionSnapshot>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FinalReviewCompletionSnapshot {
    pub commit_oid: String,
    pub tree_oid: String,
    pub source_fingerprint: String,
    pub verification_fingerprint: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FinalReviewRecord {
    pub review_id: String,
    pub reviewer_identity: String,
    pub reviewer_type: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requested_scope: Vec<String>,
    pub scope: Vec<String>,
    pub commit_range: String,
    pub outcome: FinalReviewOutcome,
    pub evidence: String,
    pub timestamp: String,
    pub source_fingerprint: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requested_verification_scope: Vec<String>,
    pub verification_scope: Vec<String>,
    pub verification_fingerprint: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FinalReviewOutcome {
    Clean,
    Finding,
}

impl FinalReviewOutcome {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "clean" => Some(Self::Clean),
            "finding" => Some(Self::Finding),
            _ => None,
        }
    }
}

impl fmt::Display for FinalReviewOutcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Clean => "clean",
            Self::Finding => "finding",
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "repair", rename_all = "snake_case")]
pub enum ValidationRepair {
    ReciprocalLinkAdded {
        task: String,
        field: String,
        target: String,
    },
    BoardEntryAdded {
        task: String,
    },
    BoardEntryRemoved {
        task: String,
    },
}

/// Authoritative task state. Markdown is deliberately absent: it is an output
/// representation produced only at user-facing rendering boundaries.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Task {
    pub stem: String,
    pub status: String,
    pub title: String,
    pub blocked_by: Vec<String>,
    pub blocks: Vec<String>,
    pub tags: Vec<String>,
    pub pr_mr_url: Option<String>,
    pub pr_mr_status: Option<String>,
    pub claim: Option<Claim>,
    pub summary: String,
    pub context: String,
    pub acceptance: Vec<ChecklistItem>,
    pub subtasks: Vec<Subtask>,
    pub notes: Vec<Note>,
    #[serde(default)]
    pub final_review: FinalReviewState,
    pub committed_at: String,
}

impl Task {
    pub fn new(stem: String, title: String, committed_at: String) -> Self {
        Self {
            stem,
            status: "backlog".into(),
            title,
            blocked_by: Vec::new(),
            blocks: Vec::new(),
            tags: Vec::new(),
            pr_mr_url: None,
            pr_mr_status: None,
            claim: None,
            summary: String::new(),
            context: String::new(),
            acceptance: Vec::new(),
            subtasks: Vec::new(),
            notes: Vec::new(),
            final_review: FinalReviewState::default(),
            committed_at,
        }
    }

    #[must_use]
    pub fn render_markdown(&self) -> String {
        let mut output = String::new();
        output.push_str("---\n");
        output.push_str(&format!("title: {}\n", self.title));
        render_array(&mut output, "blocked_by", &self.blocked_by);
        render_array(&mut output, "blocks", &self.blocks);
        render_array(&mut output, "tags", &self.tags);
        output.push_str(&format!(
            "pr_mr_url: {}\n",
            self.pr_mr_url.as_deref().unwrap_or("")
        ));
        output.push_str(&format!(
            "pr_mr_status: {}\n",
            self.pr_mr_status.as_deref().unwrap_or("")
        ));
        if let Some(claim) = &self.claim {
            output.push_str("claim:\n");
            output.push_str(&format!("  host: {}\n", claim.host));
            output.push_str(&format!("  session: {}\n", claim.session));
        }
        output.push_str("---\n\n## Summary\n");
        render_body(&mut output, &self.summary);
        output.push_str("\n## Context / Why\n");
        render_body(&mut output, &self.context);
        output.push_str("\n## Acceptance criteria\n");
        for item in &self.acceptance {
            output.push_str(&format!(
                "\n- [{}] {}",
                if item.checked { 'x' } else { ' ' },
                item.text
            ));
        }
        output.push_str("\n## Subtasks\n");
        for item in &self.subtasks {
            let after = if item.after.is_empty() {
                String::new()
            } else {
                format!(" — after: {}", item.after.join(", "))
            };
            output.push_str(&format!(
                "\n- [{}] ({}) {}{}",
                if item.checked { 'x' } else { ' ' },
                item.id,
                item.title,
                after
            ));
        }
        output.push_str("\n## Notes / Log\n");
        for note in &self.notes {
            output.push_str(&format!("\n- {}: {}", note.date, note.text));
        }
        output.push_str("\n## Final reviews\n");
        for review in &self.final_review.reviews {
            output.push_str(&format!(
                "\n- {} [{}] {} reviewer={} type={} requested_scope={} scope={} range={} source={} requested_verification_scope={} verification_scope={} verification={} evidence={}",
                review.timestamp,
                review.outcome,
                review.review_id,
                review.reviewer_identity,
                review.reviewer_type,
                review.requested_scope.join(","),
                review.scope.join(","),
                review.commit_range,
                review.source_fingerprint,
                review.requested_verification_scope.join(","),
                review.verification_scope.join(","),
                review.verification_fingerprint,
                review.evidence
            ));
        }
        if let Some(snapshot) = &self.final_review.completion_snapshot {
            output.push_str(&format!(
                "\n- completion_snapshot commit={} tree={} source={} verification={}",
                snapshot.commit_oid,
                snapshot.tree_oid,
                snapshot.source_fingerprint,
                snapshot.verification_fingerprint
            ));
        }
        output.push('\n');
        output
    }
}

fn render_array(output: &mut String, name: &str, values: &[String]) {
    if values.is_empty() {
        output.push_str(&format!("{name}: []\n"));
    } else {
        output.push_str(&format!("{name}: [{}]\n", values.join(", ")));
    }
}

fn render_body(output: &mut String, value: &str) {
    if !value.is_empty() {
        output.push('\n');
        output.push_str(value.trim());
        output.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::FinalReviewOutcome;

    #[test]
    fn final_review_outcome_rejects_unknown_persisted_values() {
        assert_eq!(
            FinalReviewOutcome::parse("clean"),
            Some(FinalReviewOutcome::Clean)
        );
        assert_eq!(
            FinalReviewOutcome::parse("finding"),
            Some(FinalReviewOutcome::Finding)
        );
        assert_eq!(FinalReviewOutcome::parse("approved"), None);
        assert_eq!(FinalReviewOutcome::Clean.to_string(), "clean");
        assert_eq!(FinalReviewOutcome::Finding.to_string(), "finding");
        assert_eq!(
            serde_json::from_str::<FinalReviewOutcome>("\"clean\"").unwrap(),
            FinalReviewOutcome::Clean
        );
        assert_eq!(
            serde_json::from_str::<FinalReviewOutcome>("\"finding\"").unwrap(),
            FinalReviewOutcome::Finding
        );
        assert!(serde_json::from_str::<FinalReviewOutcome>("\"approved\"").is_err());
    }
}
