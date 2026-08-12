#![forbid(unsafe_code)]

#[cfg(test)]
#[expect(
    clippy::absolute_paths,
    clippy::arbitrary_source_item_ordering,
    clippy::expect_used,
    clippy::implicit_return,
    clippy::shadow_unrelated,
    clippy::too_many_lines,
    reason = "black-box EventCore contract fixtures use fail-fast test ergonomics without entering shipping library code"
)]
mod tests {
    use eventcore::{RetryPolicy, execute};
    use eventcore_memory::InMemoryEventStore;
    use tiber_review::{
        ReviewStream,
        assignment::issue_assignment,
        clean::accept_clean_review,
        delta::reassess_delta,
        resolution::verify_finding_resolution,
        result::accept_assignment_result,
        risk::assess_risk,
        supersession::supersede_assignment,
        types::{
            AgentId, AssignmentAttempt, AssignmentId, AssignmentKind, AssignmentResult,
            ContextReceiptId, EvidenceId, FindingOccurrence, FindingOccurrenceId, FindingSeverity,
            LensDeltaClassification, LensRoute, LifecycleReceiptId, ModelRole, ReviewAssignment,
            ReviewIteration, ReviewLens, ReviewSessionId, ReviewSnapshotId, RiskAssessment,
            VerifierRoute,
        },
    };

    fn parsed<T>(
        value: &str,
        parser: impl FnOnce(&str) -> Result<T, tiber_review::types::ReviewError>,
    ) -> T {
        parser(value).expect("fixture value must be valid")
    }

    struct Fixture {
        stream: ReviewStream,
        snapshot: ReviewSnapshotId,
        lens: ReviewLens,
        reviewer_role: ModelRole,
        session: ReviewSessionId,
    }

    fn fixture() -> Fixture {
        let session = parsed("review-contract", ReviewSessionId::parse);
        Fixture {
            stream: ReviewStream::for_session(&session).expect("fixture stream must be valid"),
            snapshot: parsed("source-snapshot-a", ReviewSnapshotId::parse),
            lens: parsed("correctness", ReviewLens::parse),
            reviewer_role: parsed("reviewer", ModelRole::parse),
            session,
        }
    }

    fn assessment(fixture: &Fixture) -> RiskAssessment {
        RiskAssessment::parse(
            parsed("risk-evidence", EvidenceId::parse),
            parsed("delta-reviewer", ModelRole::parse),
            vec![LensRoute::new(
                fixture.lens.clone(),
                fixture.reviewer_role.clone(),
                VerifierRoute::NotRequired,
                parsed("remediation-reviewer", ModelRole::parse),
            )],
            parsed("risk-agent", AgentId::parse),
            parsed("risk-reviewer", ModelRole::parse),
            parsed("risk-context", ContextReceiptId::parse),
            parsed("risk-life", LifecycleReceiptId::parse),
        )
        .expect("fixture assessment must be valid")
    }

    fn assignment(
        fixture: &Fixture,
        attempt: AssignmentAttempt,
        context: &str,
    ) -> ReviewAssignment {
        let lifecycle = format!("{context}-closed");
        ReviewAssignment::new(
            AssignmentId::new(
                fixture.session.clone(),
                fixture.lens.clone(),
                ReviewIteration::FIRST,
                attempt,
                AssignmentKind::Lens,
            ),
            fixture.snapshot.clone(),
            parsed("reviewer-agent", AgentId::parse),
            fixture.reviewer_role.clone(),
            parsed(context, ContextReceiptId::parse),
            parsed(&lifecycle, LifecycleReceiptId::parse),
        )
    }

    #[test]
    fn issued_assignment_accepts_only_exact_scheduler_provenance() {
        let store = InMemoryEventStore::new();
        let fixture = fixture();
        let issued = assignment(&fixture, AssignmentAttempt::FIRST, "fresh-context-1");
        futures::executor::block_on(execute(
            &store,
            assess_risk(
                fixture.stream.clone(),
                fixture.snapshot.clone(),
                assessment(&fixture),
            ),
            RetryPolicy::new(),
        ))
        .expect("risk assessment must succeed");
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), issued.clone()),
            RetryPolicy::new(),
        ))
        .expect("assignment must succeed");

        let mismatched = AssignmentResult::new(
            issued.id().clone(),
            issued.snapshot().clone(),
            issued.agent_id().clone(),
            parsed("wrong-role", ModelRole::parse),
            issued.context_receipt().clone(),
            issued.lifecycle_receipt().clone(),
            parsed("result-evidence", EvidenceId::parse),
            vec![],
        );
        let mismatch = futures::executor::block_on(execute(
            &store,
            accept_assignment_result(fixture.stream.clone(), mismatched),
            RetryPolicy::new(),
        ));
        let _error = mismatch.expect_err("role mismatch must be rejected");

        let accepted = AssignmentResult::new(
            issued.id().clone(),
            issued.snapshot().clone(),
            issued.agent_id().clone(),
            issued.model_role().clone(),
            issued.context_receipt().clone(),
            issued.lifecycle_receipt().clone(),
            parsed("result-evidence", EvidenceId::parse),
            vec![],
        );
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(fixture.stream, accepted),
            RetryPolicy::new(),
        ))
        .expect("exact provenance must be accepted");
    }

    #[test]
    fn supersession_requires_a_fresh_bounded_attempt_and_context() {
        let store = InMemoryEventStore::new();
        let fixture = fixture();
        let first = assignment(&fixture, AssignmentAttempt::FIRST, "fresh-context-1");
        futures::executor::block_on(execute(
            &store,
            assess_risk(
                fixture.stream.clone(),
                fixture.snapshot.clone(),
                assessment(&fixture),
            ),
            RetryPolicy::new(),
        ))
        .expect("risk assessment must succeed");
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), first.clone()),
            RetryPolicy::new(),
        ))
        .expect("first assignment must succeed");
        futures::executor::block_on(execute(
            &store,
            supersede_assignment(
                fixture.stream.clone(),
                first.id().clone(),
                parsed("cancelled", EvidenceId::parse),
            ),
            RetryPolicy::new(),
        ))
        .expect("incomplete assignment may be superseded");

        let second_attempt = AssignmentAttempt::parse(2).expect("second attempt is bounded");
        let reused_context = assignment(&fixture, second_attempt, "fresh-context-1");
        let reused = futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), reused_context),
            RetryPolicy::new(),
        ));
        let _error = reused.expect_err("a context receipt cannot be reused");

        let replacement = assignment(&fixture, second_attempt, "fresh-context-2");
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream, replacement),
            RetryPolicy::new(),
        ))
        .expect("replacement uses the required attempt and a fresh context");
    }

    #[test]
    fn clean_review_requires_current_results_and_exact_finding_resolution() {
        let store = InMemoryEventStore::new();
        let fixture = fixture();
        let issued = assignment(&fixture, AssignmentAttempt::FIRST, "fresh-context-1");
        let finding_id = FindingOccurrenceId::new(
            issued.id().clone(),
            parsed("blocking-finding", EvidenceId::parse),
        );
        futures::executor::block_on(execute(
            &store,
            assess_risk(
                fixture.stream.clone(),
                fixture.snapshot.clone(),
                assessment(&fixture),
            ),
            RetryPolicy::new(),
        ))
        .expect("risk assessment must succeed");
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), issued.clone()),
            RetryPolicy::new(),
        ))
        .expect("assignment must succeed");
        let result = AssignmentResult::new(
            issued.id().clone(),
            issued.snapshot().clone(),
            issued.agent_id().clone(),
            issued.model_role().clone(),
            issued.context_receipt().clone(),
            issued.lifecycle_receipt().clone(),
            parsed("result-evidence", EvidenceId::parse),
            vec![FindingOccurrence::new(
                finding_id.clone(),
                FindingSeverity::Blocking,
            )],
        );
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(fixture.stream.clone(), result),
            RetryPolicy::new(),
        ))
        .expect("result must be accepted");
        let blocked = futures::executor::block_on(execute(
            &store,
            accept_clean_review(
                fixture.stream.clone(),
                fixture.snapshot.clone(),
                parsed("clean-evidence", EvidenceId::parse),
            ),
            RetryPolicy::new(),
        ));
        let _error = blocked.expect_err("unresolved blocker must prevent clean review");
        let remediation = ReviewAssignment::new(
            AssignmentId::new(
                fixture.session.clone(),
                fixture.lens.clone(),
                ReviewIteration::FIRST,
                AssignmentAttempt::FIRST,
                AssignmentKind::RemediationVerifier,
            )
            .with_occurrence_key(finding_id.evidence_id().clone()),
            fixture.snapshot.clone(),
            parsed("remediation-agent", AgentId::parse),
            parsed("remediation-reviewer", ModelRole::parse),
            parsed("remediation-context", ContextReceiptId::parse),
            parsed("remediation-closed", LifecycleReceiptId::parse),
        )
        .with_finding_target(finding_id.clone());
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), remediation.clone()),
            RetryPolicy::new(),
        ))
        .expect("remediation verifier must be assigned");
        let remediation_result = AssignmentResult::new(
            remediation.id().clone(),
            remediation.snapshot().clone(),
            remediation.agent_id().clone(),
            remediation.model_role().clone(),
            remediation.context_receipt().clone(),
            remediation.lifecycle_receipt().clone(),
            parsed("remediation-verification", EvidenceId::parse),
            vec![],
        );
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(fixture.stream.clone(), remediation_result),
            RetryPolicy::new(),
        ))
        .expect("remediation verifier result must be accepted");
        futures::executor::block_on(execute(
            &store,
            verify_finding_resolution(fixture.stream.clone(), finding_id, remediation.id().clone()),
            RetryPolicy::new(),
        ))
        .expect("exact blocking occurrence may be resolved");
        futures::executor::block_on(execute(
            &store,
            accept_clean_review(
                fixture.stream,
                fixture.snapshot,
                parsed("clean-evidence", EvidenceId::parse),
            ),
            RetryPolicy::new(),
        ))
        .expect("all current evidence permits the clean transition");
    }

    #[test]
    fn assignment_authority_and_supersession_fence_stale_work() {
        let store = InMemoryEventStore::new();
        let fixture = fixture();
        futures::executor::block_on(execute(
            &store,
            assess_risk(
                fixture.stream.clone(),
                fixture.snapshot.clone(),
                assessment(&fixture),
            ),
            RetryPolicy::new(),
        ))
        .expect("risk assessment must succeed");
        let foreign = ReviewAssignment::new(
            AssignmentId::new(
                parsed("other", ReviewSessionId::parse),
                fixture.lens.clone(),
                ReviewIteration::FIRST,
                AssignmentAttempt::FIRST,
                AssignmentKind::Lens,
            ),
            fixture.snapshot.clone(),
            parsed("agent", AgentId::parse),
            fixture.reviewer_role.clone(),
            parsed("foreign-context", ContextReceiptId::parse),
            parsed("foreign-life", LifecycleReceiptId::parse),
        );
        let rejected = futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), foreign),
            RetryPolicy::new(),
        ));
        let _error = rejected.expect_err("foreign session must be rejected");

        let issued = assignment(&fixture, AssignmentAttempt::FIRST, "late-context");
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), issued.clone()),
            RetryPolicy::new(),
        ))
        .expect("assignment succeeds");
        futures::executor::block_on(execute(
            &store,
            supersede_assignment(
                fixture.stream.clone(),
                issued.id().clone(),
                parsed("cancelled", EvidenceId::parse),
            ),
            RetryPolicy::new(),
        ))
        .expect("supersession succeeds");
        let late = AssignmentResult::new(
            issued.id().clone(),
            issued.snapshot().clone(),
            issued.agent_id().clone(),
            issued.model_role().clone(),
            issued.context_receipt().clone(),
            issued.lifecycle_receipt().clone(),
            parsed("late-result", EvidenceId::parse),
            vec![],
        );
        let rejected = futures::executor::block_on(execute(
            &store,
            accept_assignment_result(fixture.stream, late),
            RetryPolicy::new(),
        ));
        let _error = rejected.expect_err("superseded assignment result must be rejected");
    }

    #[test]
    fn all_unaffected_delta_preserves_content_review_at_commit_boundary() {
        let store = InMemoryEventStore::new();
        let fixture = fixture();
        let lens = assignment(&fixture, AssignmentAttempt::FIRST, "lens-context");
        futures::executor::block_on(execute(
            &store,
            assess_risk(
                fixture.stream.clone(),
                fixture.snapshot.clone(),
                assessment(&fixture),
            ),
            RetryPolicy::new(),
        ))
        .expect("risk succeeds");
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), lens.clone()),
            RetryPolicy::new(),
        ))
        .expect("lens assignment succeeds");
        let lens_result = AssignmentResult::new(
            lens.id().clone(),
            lens.snapshot().clone(),
            lens.agent_id().clone(),
            lens.model_role().clone(),
            lens.context_receipt().clone(),
            lens.lifecycle_receipt().clone(),
            parsed("lens-result", EvidenceId::parse),
            vec![],
        );
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(fixture.stream.clone(), lens_result),
            RetryPolicy::new(),
        ))
        .expect("lens result succeeds");
        let committed = parsed("content-identical-commit", ReviewSnapshotId::parse);
        let delta = ReviewAssignment::new(
            AssignmentId::new(
                fixture.session.clone(),
                fixture.lens.clone(),
                ReviewIteration::FIRST,
                AssignmentAttempt::FIRST,
                AssignmentKind::DeltaRisk,
            ),
            fixture.snapshot.clone(),
            parsed("delta-agent", AgentId::parse),
            parsed("delta-reviewer", ModelRole::parse),
            parsed("delta-context", ContextReceiptId::parse),
            parsed("delta-life", LifecycleReceiptId::parse),
        )
        .with_target_snapshot(committed.clone());
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), delta.clone()),
            RetryPolicy::new(),
        ))
        .expect("delta assignment succeeds");
        let delta_result = AssignmentResult::new(
            delta.id().clone(),
            delta.snapshot().clone(),
            delta.agent_id().clone(),
            delta.model_role().clone(),
            delta.context_receipt().clone(),
            delta.lifecycle_receipt().clone(),
            parsed("delta-result", EvidenceId::parse),
            vec![],
        )
        .with_delta_classifications(vec![LensDeltaClassification::new(
            fixture.lens.clone(),
            false,
        )]);
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(fixture.stream.clone(), delta_result),
            RetryPolicy::new(),
        ))
        .expect("delta result succeeds");
        futures::executor::block_on(execute(
            &store,
            reassess_delta(
                fixture.stream.clone(),
                fixture.snapshot,
                committed.clone(),
                delta.id().clone(),
            ),
            RetryPolicy::new(),
        ))
        .expect("complete delta succeeds");
        let delivered = parsed("content-identical-push", ReviewSnapshotId::parse);
        let second_delta = ReviewAssignment::new(
            AssignmentId::new(
                fixture.session.clone(),
                fixture.lens.clone(),
                ReviewIteration::parse(2).expect("second iteration is bounded"),
                AssignmentAttempt::FIRST,
                AssignmentKind::DeltaRisk,
            ),
            committed.clone(),
            parsed("second-delta-agent", AgentId::parse),
            parsed("delta-reviewer", ModelRole::parse),
            parsed("second-delta-context", ContextReceiptId::parse),
            parsed("second-delta-life", LifecycleReceiptId::parse),
        )
        .with_target_snapshot(delivered.clone());
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), second_delta.clone()),
            RetryPolicy::new(),
        ))
        .expect("a later content-identical boundary gets a new delta assignment");
        let second_result = AssignmentResult::new(
            second_delta.id().clone(),
            second_delta.snapshot().clone(),
            second_delta.agent_id().clone(),
            second_delta.model_role().clone(),
            second_delta.context_receipt().clone(),
            second_delta.lifecycle_receipt().clone(),
            parsed("second-delta-result", EvidenceId::parse),
            vec![],
        )
        .with_delta_classifications(vec![LensDeltaClassification::new(fixture.lens, false)]);
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(fixture.stream.clone(), second_result),
            RetryPolicy::new(),
        ))
        .expect("second delta result succeeds");
        futures::executor::block_on(execute(
            &store,
            reassess_delta(
                fixture.stream.clone(),
                committed,
                delivered.clone(),
                second_delta.id().clone(),
            ),
            RetryPolicy::new(),
        ))
        .expect("second complete delta succeeds");
        futures::executor::block_on(execute(
            &store,
            accept_clean_review(
                fixture.stream,
                delivered,
                parsed("clean-commit", EvidenceId::parse),
            ),
            RetryPolicy::new(),
        ))
        .expect("content-identical commit preserves review");
    }

    #[test]
    fn material_delta_rejects_stale_results_and_requires_next_iteration() {
        let store = InMemoryEventStore::new();
        let fixture = fixture();
        let stale = assignment(&fixture, AssignmentAttempt::FIRST, "stale-context");
        futures::executor::block_on(execute(
            &store,
            assess_risk(
                fixture.stream.clone(),
                fixture.snapshot.clone(),
                assessment(&fixture),
            ),
            RetryPolicy::new(),
        ))
        .expect("risk succeeds");
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), stale.clone()),
            RetryPolicy::new(),
        ))
        .expect("initial lens assignment succeeds");

        let changed = parsed("source-snapshot-b", ReviewSnapshotId::parse);
        let delta = ReviewAssignment::new(
            AssignmentId::new(
                fixture.session.clone(),
                fixture.lens.clone(),
                ReviewIteration::FIRST,
                AssignmentAttempt::FIRST,
                AssignmentKind::DeltaRisk,
            ),
            fixture.snapshot.clone(),
            parsed("delta-agent", AgentId::parse),
            parsed("delta-reviewer", ModelRole::parse),
            parsed("delta-context-material", ContextReceiptId::parse),
            parsed("delta-life-material", LifecycleReceiptId::parse),
        )
        .with_target_snapshot(changed.clone());
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), delta.clone()),
            RetryPolicy::new(),
        ))
        .expect("delta assignment succeeds");
        let delta_result = AssignmentResult::new(
            delta.id().clone(),
            delta.snapshot().clone(),
            delta.agent_id().clone(),
            delta.model_role().clone(),
            delta.context_receipt().clone(),
            delta.lifecycle_receipt().clone(),
            parsed("material-delta-result", EvidenceId::parse),
            vec![],
        )
        .with_delta_classifications(vec![LensDeltaClassification::new(
            fixture.lens.clone(),
            true,
        )]);
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(fixture.stream.clone(), delta_result),
            RetryPolicy::new(),
        ))
        .expect("delta result succeeds");
        futures::executor::block_on(execute(
            &store,
            reassess_delta(
                fixture.stream.clone(),
                fixture.snapshot.clone(),
                changed.clone(),
                delta.id().clone(),
            ),
            RetryPolicy::new(),
        ))
        .expect("material delta succeeds");

        let stale_result = AssignmentResult::new(
            stale.id().clone(),
            stale.snapshot().clone(),
            stale.agent_id().clone(),
            stale.model_role().clone(),
            stale.context_receipt().clone(),
            stale.lifecycle_receipt().clone(),
            parsed("stale-result", EvidenceId::parse),
            vec![],
        );
        let rejected = futures::executor::block_on(execute(
            &store,
            accept_assignment_result(fixture.stream.clone(), stale_result),
            RetryPolicy::new(),
        ));
        let _error = rejected.expect_err("pre-delta result must be rejected");

        let replacement = ReviewAssignment::new(
            AssignmentId::new(
                fixture.session,
                fixture.lens,
                ReviewIteration::parse(2).expect("second iteration is bounded"),
                AssignmentAttempt::FIRST,
                AssignmentKind::Lens,
            ),
            changed,
            parsed("replacement-agent", AgentId::parse),
            fixture.reviewer_role,
            parsed("replacement-context", ContextReceiptId::parse),
            parsed("replacement-life", LifecycleReceiptId::parse),
        );
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), replacement.clone()),
            RetryPolicy::new(),
        ))
        .expect("affected lens must be reassigned in the next iteration");
        let replacement_result = AssignmentResult::new(
            replacement.id().clone(),
            replacement.snapshot().clone(),
            replacement.agent_id().clone(),
            replacement.model_role().clone(),
            replacement.context_receipt().clone(),
            replacement.lifecycle_receipt().clone(),
            parsed("replacement-result", EvidenceId::parse),
            vec![],
        );
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(fixture.stream.clone(), replacement_result),
            RetryPolicy::new(),
        ))
        .expect("post-delta replacement result must be accepted");
        futures::executor::block_on(execute(
            &store,
            accept_clean_review(
                fixture.stream,
                replacement.snapshot().clone(),
                parsed("replacement-clean", EvidenceId::parse),
            ),
            RetryPolicy::new(),
        ))
        .expect("replacement evidence may complete the changed snapshot review");
    }

    #[test]
    fn concurrent_lenses_require_distinct_agents_at_issuance() {
        let store = InMemoryEventStore::new();
        let fixture = fixture();
        let second_lens = parsed("architecture", ReviewLens::parse);
        let assessment = RiskAssessment::parse(
            parsed("risk-evidence-two-lenses", EvidenceId::parse),
            parsed("delta-reviewer", ModelRole::parse),
            vec![
                LensRoute::new(
                    fixture.lens.clone(),
                    fixture.reviewer_role.clone(),
                    VerifierRoute::NotRequired,
                    parsed("remediation-reviewer", ModelRole::parse),
                ),
                LensRoute::new(
                    second_lens.clone(),
                    fixture.reviewer_role.clone(),
                    VerifierRoute::NotRequired,
                    parsed("remediation-reviewer", ModelRole::parse),
                ),
            ],
            parsed("risk-agent", AgentId::parse),
            parsed("risk-reviewer", ModelRole::parse),
            parsed("risk-context", ContextReceiptId::parse),
            parsed("risk-life", LifecycleReceiptId::parse),
        )
        .expect("two-lens assessment must be valid");
        futures::executor::block_on(execute(
            &store,
            assess_risk(fixture.stream.clone(), fixture.snapshot.clone(), assessment),
            RetryPolicy::new(),
        ))
        .expect("risk succeeds");
        let first = assignment(&fixture, AssignmentAttempt::FIRST, "first-lens-context");
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), first.clone()),
            RetryPolicy::new(),
        ))
        .expect("first lens assignment succeeds");
        let reused_agent = ReviewAssignment::new(
            AssignmentId::new(
                fixture.session,
                second_lens,
                ReviewIteration::FIRST,
                AssignmentAttempt::FIRST,
                AssignmentKind::Lens,
            ),
            fixture.snapshot,
            first.agent_id().clone(),
            fixture.reviewer_role,
            parsed("second-lens-context", ContextReceiptId::parse),
            parsed("second-lens-life", LifecycleReceiptId::parse),
        );
        let rejected = futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream, reused_agent),
            RetryPolicy::new(),
        ));
        let _error = rejected.expect_err("concurrent lenses must use distinct agents");
    }
}
