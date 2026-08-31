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
#[expect(
    clippy::inline_modules,
    reason = "this integration-test target groups its cases in one local module"
)]
mod tests {
    use eventcore::{RetryPolicy, execute};
    use eventcore_memory::InMemoryEventStore;
    use tiber_review::{
        ReviewFact, ReviewStream,
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
            LensDeltaClassification, LensRoute, LifecycleReceiptId, MAX_ASSIGNMENT_RESULT_FINDINGS,
            MAX_REVIEW_LENSES, ModelRole, ReviewAssignment, ReviewIteration, ReviewLens,
            ReviewSessionId, ReviewSnapshotId, RiskAssessment, VerifierRoute,
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

    #[test]
    fn baseline_risk_assessment_event_defaults_to_three_clean_iterations() {
        let fixture = fixture();
        let mut persisted = serde_json::to_value(ReviewFact::RiskAssessed {
            snapshot: fixture.snapshot.clone(),
            assessment: assessment(&fixture),
        })
        .expect("current risk-assessment fact must serialize");
        persisted
            .as_object_mut()
            .and_then(|fact| fact.get_mut("RiskAssessed"))
            .and_then(serde_json::Value::as_object_mut)
            .and_then(|risk_assessed| risk_assessed.get_mut("assessment"))
            .and_then(serde_json::Value::as_object_mut)
            .expect("risk assessment payload must be an object")
            .remove("required_clean_iterations");

        let restored: ReviewFact =
            serde_json::from_value(persisted).expect("baseline fact must remain replayable");
        let restored_requirement = if let ReviewFact::RiskAssessed { assessment, .. } = restored {
            Some(assessment.required_clean_iterations())
        } else {
            None
        };
        assert_eq!(
            restored_requirement,
            Some(tiber_review::types::RequiredCleanIterations::MINIMUM)
        );
    }

    fn assignment(
        fixture: &Fixture,
        attempt: AssignmentAttempt,
        context: &str,
    ) -> ReviewAssignment {
        assignment_for_iteration(fixture, ReviewIteration::FIRST, attempt, context)
    }

    fn assignment_for_iteration(
        fixture: &Fixture,
        iteration: ReviewIteration,
        attempt: AssignmentAttempt,
        context: &str,
    ) -> ReviewAssignment {
        assignment_for_lens_iteration(
            fixture,
            &fixture.lens,
            "reviewer-agent",
            iteration,
            attempt,
            context,
        )
    }

    fn assignment_for_lens_iteration(
        fixture: &Fixture,
        lens: &ReviewLens,
        agent: &str,
        iteration: ReviewIteration,
        attempt: AssignmentAttempt,
        context: &str,
    ) -> ReviewAssignment {
        assignment_for_snapshot_lens_kind(
            fixture,
            &fixture.snapshot,
            lens,
            agent,
            &fixture.reviewer_role,
            iteration,
            attempt,
            AssignmentKind::Lens,
            context,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "the black-box fixture keeps every assignment authority field explicit"
    )]
    fn assignment_for_snapshot_lens_kind(
        fixture: &Fixture,
        snapshot: &ReviewSnapshotId,
        lens: &ReviewLens,
        agent: &str,
        model_role: &ModelRole,
        iteration: ReviewIteration,
        attempt: AssignmentAttempt,
        kind: AssignmentKind,
        context: &str,
    ) -> ReviewAssignment {
        let lifecycle = format!("{context}-closed");
        ReviewAssignment::new(
            AssignmentId::new(
                fixture.session.clone(),
                lens.clone(),
                iteration,
                attempt,
                kind,
            ),
            snapshot.clone(),
            parsed(agent, AgentId::parse),
            model_role.clone(),
            parsed(context, ContextReceiptId::parse),
            parsed(&lifecycle, LifecycleReceiptId::parse),
        )
    }

    fn clean_result(assignment: &ReviewAssignment, evidence: &str) -> AssignmentResult {
        AssignmentResult::new(
            assignment.id().clone(),
            assignment.snapshot().clone(),
            assignment.agent_id().clone(),
            assignment.model_role().clone(),
            assignment.context_receipt().clone(),
            assignment.lifecycle_receipt().clone(),
            parsed(evidence, EvidenceId::parse),
            vec![],
        )
    }

    #[test]
    fn clean_review_requires_three_consecutive_finding_free_iterations() {
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

        for iteration_number in 1..=3 {
            let iteration =
                ReviewIteration::parse(iteration_number).expect("clean pass is bounded");
            let context = format!("clean-context-{}", iteration.get());
            let issued =
                assignment_for_iteration(&fixture, iteration, AssignmentAttempt::FIRST, &context);
            futures::executor::block_on(execute(
                &store,
                issue_assignment(fixture.stream.clone(), issued.clone()),
                RetryPolicy::new(),
            ))
            .expect("each clean iteration must receive fresh assigned work");
            let result = AssignmentResult::new(
                issued.id().clone(),
                issued.snapshot().clone(),
                issued.agent_id().clone(),
                issued.model_role().clone(),
                issued.context_receipt().clone(),
                issued.lifecycle_receipt().clone(),
                parsed(
                    &format!("clean-result-{}", iteration.get()),
                    EvidenceId::parse,
                ),
                vec![],
            );
            futures::executor::block_on(execute(
                &store,
                accept_assignment_result(fixture.stream.clone(), result),
                RetryPolicy::new(),
            ))
            .expect("finding-free assigned work must be accepted");
            futures::executor::block_on(execute(
                &store,
                accept_clean_review(
                    fixture.stream.clone(),
                    fixture.snapshot.clone(),
                    parsed(
                        &format!("clean-iteration-{}", iteration.get()),
                        EvidenceId::parse,
                    ),
                ),
                RetryPolicy::new(),
            ))
            .expect("a completed finding-free iteration must be recorded");
        }

        let fourth = assignment_for_iteration(
            &fixture,
            ReviewIteration::parse(4).expect("fourth iteration is bounded"),
            AssignmentAttempt::FIRST,
            "clean-context-4",
        );
        let rejected = futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream, fourth),
            RetryPolicy::new(),
        ));
        let _error = rejected.expect_err("the third consecutive clean iteration is terminal");
    }

    #[test]
    fn malformed_result_after_two_clean_passes_requires_three_fresh_passes() {
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

        for iteration_number in 1..=2 {
            let iteration = ReviewIteration::parse(iteration_number).expect("pass is bounded");
            let context = format!("pre-malformed-context-{}", iteration.get());
            let issued =
                assignment_for_iteration(&fixture, iteration, AssignmentAttempt::FIRST, &context);
            futures::executor::block_on(execute(
                &store,
                issue_assignment(fixture.stream.clone(), issued.clone()),
                RetryPolicy::new(),
            ))
            .expect("pre-malformed assignment must succeed");
            futures::executor::block_on(execute(
                &store,
                accept_assignment_result(
                    fixture.stream.clone(),
                    AssignmentResult::new(
                        issued.id().clone(),
                        issued.snapshot().clone(),
                        issued.agent_id().clone(),
                        issued.model_role().clone(),
                        issued.context_receipt().clone(),
                        issued.lifecycle_receipt().clone(),
                        parsed(
                            &format!("pre-malformed-result-{}", iteration.get()),
                            EvidenceId::parse,
                        ),
                        vec![],
                    ),
                ),
                RetryPolicy::new(),
            ))
            .expect("pre-malformed clean result must succeed");
            futures::executor::block_on(execute(
                &store,
                accept_clean_review(
                    fixture.stream.clone(),
                    fixture.snapshot.clone(),
                    parsed(
                        &format!("pre-malformed-iteration-{}", iteration.get()),
                        EvidenceId::parse,
                    ),
                ),
                RetryPolicy::new(),
            ))
            .expect("pre-malformed clean pass must be recorded");
        }

        let malformed_assignment = assignment_for_iteration(
            &fixture,
            ReviewIteration::parse(3).expect("third iteration is bounded"),
            AssignmentAttempt::FIRST,
            "malformed-context-3",
        );
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), malformed_assignment.clone()),
            RetryPolicy::new(),
        ))
        .expect("third assignment must succeed");
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(
                fixture.stream.clone(),
                AssignmentResult::new(
                    malformed_assignment.id().clone(),
                    malformed_assignment.snapshot().clone(),
                    malformed_assignment.agent_id().clone(),
                    parsed("wrong-role", ModelRole::parse),
                    malformed_assignment.context_receipt().clone(),
                    malformed_assignment.lifecycle_receipt().clone(),
                    parsed("malformed-result-3", EvidenceId::parse),
                    vec![],
                ),
            ),
            RetryPolicy::new(),
        ))
        .expect("malformed current evidence must emit the durable reset fact");

        let premature = futures::executor::block_on(execute(
            &store,
            accept_clean_review(
                fixture.stream.clone(),
                fixture.snapshot.clone(),
                parsed("premature-after-malformed", EvidenceId::parse),
            ),
            RetryPolicy::new(),
        ));
        let _error = premature.expect_err("the malformed attempt must clear the two-pass streak");

        for iteration_number in 4..=6 {
            let iteration = ReviewIteration::parse(iteration_number).expect("pass is bounded");
            let context = format!("post-malformed-context-{}", iteration.get());
            let issued =
                assignment_for_iteration(&fixture, iteration, AssignmentAttempt::FIRST, &context);
            futures::executor::block_on(execute(
                &store,
                issue_assignment(fixture.stream.clone(), issued.clone()),
                RetryPolicy::new(),
            ))
            .expect("fresh post-malformed assignment must succeed");
            futures::executor::block_on(execute(
                &store,
                accept_assignment_result(
                    fixture.stream.clone(),
                    AssignmentResult::new(
                        issued.id().clone(),
                        issued.snapshot().clone(),
                        issued.agent_id().clone(),
                        issued.model_role().clone(),
                        issued.context_receipt().clone(),
                        issued.lifecycle_receipt().clone(),
                        parsed(
                            &format!("post-malformed-result-{}", iteration.get()),
                            EvidenceId::parse,
                        ),
                        vec![],
                    ),
                ),
                RetryPolicy::new(),
            ))
            .expect("fresh post-malformed result must succeed");
            futures::executor::block_on(execute(
                &store,
                accept_clean_review(
                    fixture.stream.clone(),
                    fixture.snapshot.clone(),
                    parsed(
                        &format!("post-malformed-iteration-{}", iteration.get()),
                        EvidenceId::parse,
                    ),
                ),
                RetryPolicy::new(),
            ))
            .expect("each fresh post-malformed pass must be recorded");
        }

        let seventh = assignment_for_iteration(
            &fixture,
            ReviewIteration::parse(7).expect("seventh iteration is bounded"),
            AssignmentAttempt::FIRST,
            "post-malformed-context-7",
        );
        let rejected = futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream, seventh),
            RetryPolicy::new(),
        ));
        let _error = rejected.expect_err("only three fresh passes after malformed are terminal");
    }

    #[test]
    fn any_finding_resets_the_consecutive_clean_streak() {
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

        for iteration_number in 1..=5 {
            let iteration =
                ReviewIteration::parse(iteration_number).expect("review pass is bounded");
            let context = format!("streak-context-{}", iteration.get());
            let issued =
                assignment_for_iteration(&fixture, iteration, AssignmentAttempt::FIRST, &context);
            futures::executor::block_on(execute(
                &store,
                issue_assignment(fixture.stream.clone(), issued.clone()),
                RetryPolicy::new(),
            ))
            .expect("each iteration must receive fresh assigned work");
            let findings = if iteration.get() == 2 {
                vec![FindingOccurrence::new(
                    FindingOccurrenceId::new(
                        issued.id().clone(),
                        parsed("iteration-two-finding", EvidenceId::parse),
                    ),
                    FindingSeverity::Observation,
                )]
            } else {
                vec![]
            };
            let result = AssignmentResult::new(
                issued.id().clone(),
                issued.snapshot().clone(),
                issued.agent_id().clone(),
                issued.model_role().clone(),
                issued.context_receipt().clone(),
                issued.lifecycle_receipt().clone(),
                parsed(
                    &format!("streak-result-{}", iteration.get()),
                    EvidenceId::parse,
                ),
                findings,
            );
            futures::executor::block_on(execute(
                &store,
                accept_assignment_result(fixture.stream.clone(), result),
                RetryPolicy::new(),
            ))
            .expect("assigned work must be accepted");
            if iteration.get() == 2 {
                continue;
            }
            futures::executor::block_on(execute(
                &store,
                accept_clean_review(
                    fixture.stream.clone(),
                    fixture.snapshot.clone(),
                    parsed(
                        &format!("streak-iteration-{}", iteration.get()),
                        EvidenceId::parse,
                    ),
                ),
                RetryPolicy::new(),
            ))
            .expect("each complete iteration must be durably recorded");
        }

        let sixth = assignment_for_iteration(
            &fixture,
            ReviewIteration::parse(6).expect("sixth iteration is bounded"),
            AssignmentAttempt::FIRST,
            "streak-context-6",
        );
        let rejected = futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream, sixth),
            RetryPolicy::new(),
        ));
        let _error = rejected.expect_err(
            "iterations three through five are the first three consecutive clean passes",
        );
    }

    #[test]
    fn every_clean_iteration_requires_every_risk_selected_lens() {
        let store = InMemoryEventStore::new();
        let fixture = fixture();
        let architecture = parsed("architecture", ReviewLens::parse);
        let risk = RiskAssessment::parse(
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
                    architecture.clone(),
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
            assess_risk(fixture.stream.clone(), fixture.snapshot.clone(), risk),
            RetryPolicy::new(),
        ))
        .expect("risk assessment must succeed");

        let correctness = assignment_for_iteration(
            &fixture,
            ReviewIteration::FIRST,
            AssignmentAttempt::FIRST,
            "multi-correctness-1",
        );
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), correctness.clone()),
            RetryPolicy::new(),
        ))
        .expect("correctness assignment must succeed");
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(
                fixture.stream.clone(),
                AssignmentResult::new(
                    correctness.id().clone(),
                    correctness.snapshot().clone(),
                    correctness.agent_id().clone(),
                    correctness.model_role().clone(),
                    correctness.context_receipt().clone(),
                    correctness.lifecycle_receipt().clone(),
                    parsed("multi-correctness-result-1", EvidenceId::parse),
                    vec![],
                ),
            ),
            RetryPolicy::new(),
        ))
        .expect("correctness result must succeed");
        let incomplete = futures::executor::block_on(execute(
            &store,
            accept_clean_review(
                fixture.stream.clone(),
                fixture.snapshot.clone(),
                parsed("incomplete-multi-pass", EvidenceId::parse),
            ),
            RetryPolicy::new(),
        ));
        let _error = incomplete.expect_err("one clean lens cannot complete a multi-lens pass");

        let architecture_assignment = ReviewAssignment::new(
            AssignmentId::new(
                fixture.session.clone(),
                architecture,
                ReviewIteration::FIRST,
                AssignmentAttempt::FIRST,
                AssignmentKind::Lens,
            ),
            fixture.snapshot.clone(),
            parsed("architecture-agent", AgentId::parse),
            fixture.reviewer_role.clone(),
            parsed("multi-architecture-1", ContextReceiptId::parse),
            parsed("multi-architecture-1-closed", LifecycleReceiptId::parse),
        );
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), architecture_assignment.clone()),
            RetryPolicy::new(),
        ))
        .expect("architecture assignment must succeed");
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(
                fixture.stream.clone(),
                AssignmentResult::new(
                    architecture_assignment.id().clone(),
                    architecture_assignment.snapshot().clone(),
                    architecture_assignment.agent_id().clone(),
                    architecture_assignment.model_role().clone(),
                    architecture_assignment.context_receipt().clone(),
                    architecture_assignment.lifecycle_receipt().clone(),
                    parsed("multi-architecture-result-1", EvidenceId::parse),
                    vec![],
                ),
            ),
            RetryPolicy::new(),
        ))
        .expect("architecture result must succeed");
        futures::executor::block_on(execute(
            &store,
            accept_clean_review(
                fixture.stream.clone(),
                fixture.snapshot.clone(),
                parsed("complete-multi-pass", EvidenceId::parse),
            ),
            RetryPolicy::new(),
        ))
        .expect("both clean lenses complete the first iteration");

        let second_correctness = assignment_for_iteration(
            &fixture,
            ReviewIteration::parse(2).expect("second iteration is bounded"),
            AssignmentAttempt::FIRST,
            "multi-correctness-2",
        );
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), second_correctness.clone()),
            RetryPolicy::new(),
        ))
        .expect("second-iteration correctness assignment must succeed");
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(
                fixture.stream.clone(),
                AssignmentResult::new(
                    second_correctness.id().clone(),
                    second_correctness.snapshot().clone(),
                    second_correctness.agent_id().clone(),
                    second_correctness.model_role().clone(),
                    second_correctness.context_receipt().clone(),
                    second_correctness.lifecycle_receipt().clone(),
                    parsed("multi-correctness-result-2", EvidenceId::parse),
                    vec![],
                ),
            ),
            RetryPolicy::new(),
        ))
        .expect("second-iteration correctness result must succeed");
        let incomplete = futures::executor::block_on(execute(
            &store,
            accept_clean_review(
                fixture.stream,
                fixture.snapshot.clone(),
                parsed("incomplete-second-multi-pass", EvidenceId::parse),
            ),
            RetryPolicy::new(),
        ));
        let _error = incomplete.expect_err("every iteration must rerun every selected lens");
    }

    #[test]
    fn malformed_current_result_invalidates_the_iteration() {
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
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(fixture.stream.clone(), mismatched),
            RetryPolicy::new(),
        ))
        .expect("role mismatch must be durably recorded as a rejected result");

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
        let stale_retry = futures::executor::block_on(execute(
            &store,
            accept_assignment_result(fixture.stream.clone(), accepted),
            RetryPolicy::new(),
        ));
        let _error = stale_retry.expect_err("the invalidated iteration cannot be retried");

        let next = assignment_for_iteration(
            &fixture,
            ReviewIteration::parse(2).expect("second iteration is bounded"),
            AssignmentAttempt::FIRST,
            "fresh-context-2",
        );
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream, next),
            RetryPolicy::new(),
        ))
        .expect("malformed evidence requires a fresh next-iteration assignment");
    }

    #[test]
    fn malformed_lens_result_invalidates_open_peers_in_same_iteration() {
        let store = InMemoryEventStore::new();
        let fixture = fixture();
        let architecture = parsed("architecture", ReviewLens::parse);
        let risk = RiskAssessment::parse(
            parsed("risk-evidence-peer-invalidation", EvidenceId::parse),
            parsed("delta-reviewer", ModelRole::parse),
            vec![
                LensRoute::new(
                    fixture.lens.clone(),
                    fixture.reviewer_role.clone(),
                    VerifierRoute::NotRequired,
                    parsed("remediation-reviewer", ModelRole::parse),
                ),
                LensRoute::new(
                    architecture.clone(),
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
            assess_risk(fixture.stream.clone(), fixture.snapshot.clone(), risk),
            RetryPolicy::new(),
        ))
        .expect("risk assessment must succeed");

        let first_correctness = assignment_for_iteration(
            &fixture,
            ReviewIteration::FIRST,
            AssignmentAttempt::FIRST,
            "peer-correctness-1",
        );
        let first_architecture = assignment_for_lens_iteration(
            &fixture,
            &architecture,
            "architecture-agent",
            ReviewIteration::FIRST,
            AssignmentAttempt::FIRST,
            "peer-architecture-1",
        );
        for issued in [&first_correctness, &first_architecture] {
            futures::executor::block_on(execute(
                &store,
                issue_assignment(fixture.stream.clone(), issued.clone()),
                RetryPolicy::new(),
            ))
            .expect("first-iteration assignment must succeed");
        }

        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(
                fixture.stream.clone(),
                AssignmentResult::new(
                    first_correctness.id().clone(),
                    first_correctness.snapshot().clone(),
                    first_correctness.agent_id().clone(),
                    parsed("wrong-role", ModelRole::parse),
                    first_correctness.context_receipt().clone(),
                    first_correctness.lifecycle_receipt().clone(),
                    parsed("peer-malformed-result-1", EvidenceId::parse),
                    vec![],
                ),
            ),
            RetryPolicy::new(),
        ))
        .expect("malformed result must durably invalidate the iteration");

        let stale_peer = futures::executor::block_on(execute(
            &store,
            accept_assignment_result(
                fixture.stream.clone(),
                AssignmentResult::new(
                    first_architecture.id().clone(),
                    first_architecture.snapshot().clone(),
                    first_architecture.agent_id().clone(),
                    first_architecture.model_role().clone(),
                    first_architecture.context_receipt().clone(),
                    first_architecture.lifecycle_receipt().clone(),
                    parsed("peer-stale-result-1", EvidenceId::parse),
                    vec![],
                ),
            ),
            RetryPolicy::new(),
        ));
        let _error = stale_peer.expect_err("an invalidated iteration must close every open peer");
        let stale_supersession = futures::executor::block_on(execute(
            &store,
            supersede_assignment(
                fixture.stream.clone(),
                first_architecture.id().clone(),
                parsed("peer-stale-supersession-1", EvidenceId::parse),
            ),
            RetryPolicy::new(),
        ));
        let _error = stale_supersession
            .expect_err("a malformed peer result must close supersession for the whole iteration");

        let second_iteration = ReviewIteration::parse(2).expect("second iteration is bounded");
        let second_correctness = assignment_for_iteration(
            &fixture,
            second_iteration,
            AssignmentAttempt::FIRST,
            "peer-correctness-2",
        );
        let second_architecture = assignment_for_lens_iteration(
            &fixture,
            &architecture,
            "architecture-agent",
            second_iteration,
            AssignmentAttempt::FIRST,
            "peer-architecture-2",
        );
        for issued in [&second_correctness, &second_architecture] {
            futures::executor::block_on(execute(
                &store,
                issue_assignment(fixture.stream.clone(), issued.clone()),
                RetryPolicy::new(),
            ))
            .expect("second-iteration assignment must succeed");
        }

        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(
                fixture.stream.clone(),
                AssignmentResult::new(
                    second_correctness.id().clone(),
                    second_correctness.snapshot().clone(),
                    second_correctness.agent_id().clone(),
                    second_correctness.model_role().clone(),
                    second_correctness.context_receipt().clone(),
                    second_correctness.lifecycle_receipt().clone(),
                    parsed("peer-correctness-result-2", EvidenceId::parse),
                    vec![],
                ),
            ),
            RetryPolicy::new(),
        ))
        .expect("second-iteration correctness result must succeed");
        let incomplete = futures::executor::block_on(execute(
            &store,
            accept_clean_review(
                fixture.stream.clone(),
                fixture.snapshot.clone(),
                parsed("peer-incomplete-clean-2", EvidenceId::parse),
            ),
            RetryPolicy::new(),
        ));
        let _error = incomplete.expect_err("stale peer evidence cannot carry into a new iteration");

        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(
                fixture.stream.clone(),
                AssignmentResult::new(
                    second_architecture.id().clone(),
                    second_architecture.snapshot().clone(),
                    second_architecture.agent_id().clone(),
                    second_architecture.model_role().clone(),
                    second_architecture.context_receipt().clone(),
                    second_architecture.lifecycle_receipt().clone(),
                    parsed("peer-architecture-result-2", EvidenceId::parse),
                    vec![],
                ),
            ),
            RetryPolicy::new(),
        ))
        .expect("fresh second-iteration peer result must succeed");
        futures::executor::block_on(execute(
            &store,
            accept_clean_review(
                fixture.stream,
                fixture.snapshot.clone(),
                parsed("peer-complete-clean-2", EvidenceId::parse),
            ),
            RetryPolicy::new(),
        ))
        .expect("only a complete fresh two-lens iteration may count clean");
    }

    #[test]
    fn finding_bearing_result_invalidates_open_peers_and_requires_a_fresh_complete_iteration() {
        let store = InMemoryEventStore::new();
        let fixture = fixture();
        let architecture = parsed("architecture", ReviewLens::parse);
        let risk = RiskAssessment::parse(
            parsed("risk-evidence-finding-peer-invalidation", EvidenceId::parse),
            parsed("delta-reviewer", ModelRole::parse),
            vec![
                LensRoute::new(
                    fixture.lens.clone(),
                    fixture.reviewer_role.clone(),
                    VerifierRoute::NotRequired,
                    parsed("remediation-reviewer", ModelRole::parse),
                ),
                LensRoute::new(
                    architecture.clone(),
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
            assess_risk(fixture.stream.clone(), fixture.snapshot.clone(), risk),
            RetryPolicy::new(),
        ))
        .expect("risk assessment must succeed");

        let first_correctness = assignment_for_iteration(
            &fixture,
            ReviewIteration::FIRST,
            AssignmentAttempt::FIRST,
            "finding-peer-correctness-1",
        );
        let first_architecture = assignment_for_lens_iteration(
            &fixture,
            &architecture,
            "architecture-agent",
            ReviewIteration::FIRST,
            AssignmentAttempt::FIRST,
            "finding-peer-architecture-1",
        );
        for issued in [&first_correctness, &first_architecture] {
            futures::executor::block_on(execute(
                &store,
                issue_assignment(fixture.stream.clone(), issued.clone()),
                RetryPolicy::new(),
            ))
            .expect("first-iteration assignment must succeed");
        }

        let finding_id = FindingOccurrenceId::new(
            first_correctness.id().clone(),
            parsed("finding-peer-observation-1", EvidenceId::parse),
        );
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(
                fixture.stream.clone(),
                AssignmentResult::new(
                    first_correctness.id().clone(),
                    first_correctness.snapshot().clone(),
                    first_correctness.agent_id().clone(),
                    first_correctness.model_role().clone(),
                    first_correctness.context_receipt().clone(),
                    first_correctness.lifecycle_receipt().clone(),
                    parsed("finding-peer-result-1", EvidenceId::parse),
                    vec![FindingOccurrence::new(
                        finding_id,
                        FindingSeverity::Observation,
                    )],
                ),
            ),
            RetryPolicy::new(),
        ))
        .expect("a valid finding-bearing result must be accepted");

        let stale_peer = futures::executor::block_on(execute(
            &store,
            accept_assignment_result(
                fixture.stream.clone(),
                clean_result(&first_architecture, "finding-peer-stale-result-1"),
            ),
            RetryPolicy::new(),
        ));
        let _error = stale_peer.expect_err("a finding must close every open peer assignment");
        let stale_supersession = futures::executor::block_on(execute(
            &store,
            supersede_assignment(
                fixture.stream.clone(),
                first_architecture.id().clone(),
                parsed("finding-peer-stale-supersession-1", EvidenceId::parse),
            ),
            RetryPolicy::new(),
        ));
        let _error = stale_supersession
            .expect_err("a finding must close peer supersession with the iteration");

        let second_iteration = ReviewIteration::parse(2).expect("second iteration is bounded");
        let second_correctness = assignment_for_iteration(
            &fixture,
            second_iteration,
            AssignmentAttempt::FIRST,
            "finding-peer-correctness-2",
        );
        let second_architecture = assignment_for_lens_iteration(
            &fixture,
            &architecture,
            "architecture-agent",
            second_iteration,
            AssignmentAttempt::FIRST,
            "finding-peer-architecture-2",
        );
        for issued in [&second_correctness, &second_architecture] {
            futures::executor::block_on(execute(
                &store,
                issue_assignment(fixture.stream.clone(), issued.clone()),
                RetryPolicy::new(),
            ))
            .expect("fresh second-iteration assignment must succeed");
        }

        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(
                fixture.stream.clone(),
                clean_result(&second_correctness, "finding-peer-correctness-result-2"),
            ),
            RetryPolicy::new(),
        ))
        .expect("fresh correctness result must succeed");
        let incomplete = futures::executor::block_on(execute(
            &store,
            accept_clean_review(
                fixture.stream.clone(),
                fixture.snapshot.clone(),
                parsed("finding-peer-incomplete-clean-2", EvidenceId::parse),
            ),
            RetryPolicy::new(),
        ));
        let _error =
            incomplete.expect_err("one stale peer result cannot complete a fresh iteration");

        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(
                fixture.stream.clone(),
                clean_result(&second_architecture, "finding-peer-architecture-result-2"),
            ),
            RetryPolicy::new(),
        ))
        .expect("fresh architecture result must succeed");
        futures::executor::block_on(execute(
            &store,
            accept_clean_review(
                fixture.stream,
                fixture.snapshot.clone(),
                parsed("finding-peer-complete-clean-2", EvidenceId::parse),
            ),
            RetryPolicy::new(),
        ))
        .expect("only a complete fresh two-lens iteration may count clean");
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
        let second_iteration = ReviewIteration::parse(2).expect("second iteration is bounded");
        let fresh_review = assignment_for_iteration(
            &fixture,
            second_iteration,
            AssignmentAttempt::FIRST,
            "post-finding-review",
        );
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), fresh_review.clone()),
            RetryPolicy::new(),
        ))
        .expect("a finding must restart the review with fresh assigned work");
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(
                fixture.stream.clone(),
                clean_result(&fresh_review, "post-finding-review-result"),
            ),
            RetryPolicy::new(),
        ))
        .expect("fresh post-finding review evidence must be accepted");

        let remediation = ReviewAssignment::new(
            AssignmentId::new(
                fixture.session.clone(),
                fixture.lens.clone(),
                second_iteration,
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
                fixture.stream.clone(),
                fixture.snapshot.clone(),
                parsed("clean-evidence", EvidenceId::parse),
            ),
            RetryPolicy::new(),
        ))
        .expect("resolved findings permit the fresh complete iteration to count clean");
        let next_review = assignment_for_iteration(
            &fixture,
            ReviewIteration::parse(3).expect("third iteration is bounded"),
            AssignmentAttempt::FIRST,
            "post-remediation-review-3",
        );
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream, next_review),
            RetryPolicy::new(),
        ))
        .expect("the invalidated finding iteration never counts toward the clean streak");
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
    fn oversized_delta_classification_is_rejected_before_result_persistence() {
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
        let changed = parsed("oversized-delta-snapshot", ReviewSnapshotId::parse);
        let delta = ReviewAssignment::new(
            AssignmentId::new(
                fixture.session.clone(),
                fixture.lens.clone(),
                ReviewIteration::FIRST,
                AssignmentAttempt::FIRST,
                AssignmentKind::DeltaRisk,
            ),
            fixture.snapshot.clone(),
            parsed("oversized-delta-agent", AgentId::parse),
            parsed("delta-reviewer", ModelRole::parse),
            parsed("oversized-delta-context", ContextReceiptId::parse),
            parsed("oversized-delta-life", LifecycleReceiptId::parse),
        )
        .with_target_snapshot(changed.clone());
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), delta.clone()),
            RetryPolicy::new(),
        ))
        .expect("delta assignment must succeed");
        let oversized = AssignmentResult::new(
            delta.id().clone(),
            delta.snapshot().clone(),
            delta.agent_id().clone(),
            delta.model_role().clone(),
            delta.context_receipt().clone(),
            delta.lifecycle_receipt().clone(),
            parsed("oversized-delta-result", EvidenceId::parse),
            vec![],
        )
        .with_delta_classifications(vec![
            LensDeltaClassification::new(
                fixture.lens.clone(),
                true
            );
            MAX_REVIEW_LENSES + 1
        ]);
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(fixture.stream.clone(), oversized),
            RetryPolicy::new(),
        ))
        .expect("oversized delta evidence must emit only a bounded rejection fact");

        let retry = AssignmentResult::new(
            delta.id().clone(),
            delta.snapshot().clone(),
            delta.agent_id().clone(),
            delta.model_role().clone(),
            delta.context_receipt().clone(),
            delta.lifecycle_receipt().clone(),
            parsed("bounded-delta-retry", EvidenceId::parse),
            vec![],
        )
        .with_delta_classifications(vec![LensDeltaClassification::new(
            fixture.lens.clone(),
            true,
        )]);
        let rejected_retry = futures::executor::block_on(execute(
            &store,
            accept_assignment_result(fixture.stream.clone(), retry),
            RetryPolicy::new(),
        ));
        let _error = rejected_retry.expect_err("rejected assignment cannot be reused");
        let unapplied = futures::executor::block_on(execute(
            &store,
            reassess_delta(
                fixture.stream,
                fixture.snapshot,
                changed,
                delta.id().clone(),
            ),
            RetryPolicy::new(),
        ));
        let _error = unapplied.expect_err("the oversized result was never accepted or persisted");
    }

    #[test]
    fn oversized_finding_result_is_rejected_before_identity_set_or_persistence() {
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
        let changed = parsed("oversized-finding-snapshot", ReviewSnapshotId::parse);
        let delta = ReviewAssignment::new(
            AssignmentId::new(
                fixture.session.clone(),
                fixture.lens.clone(),
                ReviewIteration::FIRST,
                AssignmentAttempt::FIRST,
                AssignmentKind::DeltaRisk,
            ),
            fixture.snapshot.clone(),
            parsed("oversized-finding-agent", AgentId::parse),
            parsed("delta-reviewer", ModelRole::parse),
            parsed("oversized-finding-context", ContextReceiptId::parse),
            parsed("oversized-finding-life", LifecycleReceiptId::parse),
        )
        .with_target_snapshot(changed.clone());
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), delta.clone()),
            RetryPolicy::new(),
        ))
        .expect("delta assignment must succeed");
        let findings = (0..=MAX_ASSIGNMENT_RESULT_FINDINGS)
            .map(|index| {
                FindingOccurrence::new(
                    FindingOccurrenceId::new(
                        delta.id().clone(),
                        parsed(&format!("oversized-finding-{index}"), EvidenceId::parse),
                    ),
                    FindingSeverity::Observation,
                )
            })
            .collect();
        let oversized = AssignmentResult::new(
            delta.id().clone(),
            delta.snapshot().clone(),
            delta.agent_id().clone(),
            delta.model_role().clone(),
            delta.context_receipt().clone(),
            delta.lifecycle_receipt().clone(),
            parsed("oversized-finding-result", EvidenceId::parse),
            findings,
        )
        .with_delta_classifications(vec![LensDeltaClassification::new(
            fixture.lens.clone(),
            true,
        )]);
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(fixture.stream.clone(), oversized),
            RetryPolicy::new(),
        ))
        .expect("oversized findings must emit only a bounded rejection fact");

        let unapplied = futures::executor::block_on(execute(
            &store,
            reassess_delta(
                fixture.stream,
                fixture.snapshot,
                changed,
                delta.id().clone(),
            ),
            RetryPolicy::new(),
        ));
        let _error =
            unapplied.expect_err("the oversized finding result was never accepted or persisted");
    }

    #[test]
    fn all_unaffected_delta_requires_fresh_lens_authorization_for_verifier() {
        let store = InMemoryEventStore::new();
        let fixture = fixture();
        let verifier_role = parsed("verifier", ModelRole::parse);
        let risk = RiskAssessment::parse(
            parsed("risk-evidence-delta-verifier", EvidenceId::parse),
            parsed("delta-reviewer", ModelRole::parse),
            vec![LensRoute::new(
                fixture.lens.clone(),
                fixture.reviewer_role.clone(),
                VerifierRoute::Required {
                    model_role: verifier_role.clone(),
                },
                parsed("remediation-reviewer", ModelRole::parse),
            )],
            parsed("risk-agent", AgentId::parse),
            parsed("risk-reviewer", ModelRole::parse),
            parsed("risk-context", ContextReceiptId::parse),
            parsed("risk-life", LifecycleReceiptId::parse),
        )
        .expect("verifier-routed assessment must be valid");
        futures::executor::block_on(execute(
            &store,
            assess_risk(fixture.stream.clone(), fixture.snapshot.clone(), risk),
            RetryPolicy::new(),
        ))
        .expect("risk assessment must succeed");

        let prior_lens = assignment(
            &fixture,
            AssignmentAttempt::FIRST,
            "delta-verifier-prior-lens-context",
        );
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), prior_lens.clone()),
            RetryPolicy::new(),
        ))
        .expect("prior lens assignment must succeed");
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(
                fixture.stream.clone(),
                clean_result(&prior_lens, "delta-verifier-prior-lens-result"),
            ),
            RetryPolicy::new(),
        ))
        .expect("prior lens result must succeed");

        let changed = parsed("delta-verifier-snapshot-b", ReviewSnapshotId::parse);
        let delta = assignment_for_snapshot_lens_kind(
            &fixture,
            &fixture.snapshot,
            &fixture.lens,
            "delta-verifier-agent",
            &parsed("delta-reviewer", ModelRole::parse),
            ReviewIteration::FIRST,
            AssignmentAttempt::FIRST,
            AssignmentKind::DeltaRisk,
            "delta-verifier-context",
        )
        .with_target_snapshot(changed.clone());
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), delta.clone()),
            RetryPolicy::new(),
        ))
        .expect("delta assignment must succeed");
        let delta_result =
            clean_result(&delta, "delta-verifier-result").with_delta_classifications(vec![
                LensDeltaClassification::new(fixture.lens.clone(), false),
            ]);
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(fixture.stream.clone(), delta_result),
            RetryPolicy::new(),
        ))
        .expect("all-unaffected delta result must succeed");
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
        .expect("all-unaffected delta reassessment must succeed");

        let second_iteration = ReviewIteration::parse(2).expect("second iteration must be bounded");
        let verifier = assignment_for_snapshot_lens_kind(
            &fixture,
            &changed,
            &fixture.lens,
            "delta-verifier-fresh-agent",
            &verifier_role,
            second_iteration,
            AssignmentAttempt::FIRST,
            AssignmentKind::Verifier,
            "delta-verifier-fresh-context",
        );
        let unauthorized = futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), verifier.clone()),
            RetryPolicy::new(),
        ));
        let _error = unauthorized
            .expect_err("a prior-iteration lens result cannot authorize a verifier after delta");

        let fresh_lens = assignment_for_snapshot_lens_kind(
            &fixture,
            &changed,
            &fixture.lens,
            "delta-verifier-fresh-lens-agent",
            &fixture.reviewer_role,
            second_iteration,
            AssignmentAttempt::FIRST,
            AssignmentKind::Lens,
            "delta-verifier-fresh-lens-context",
        );
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), fresh_lens.clone()),
            RetryPolicy::new(),
        ))
        .expect("fresh lens assignment must succeed in the new iteration");
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(
                fixture.stream.clone(),
                clean_result(&fresh_lens, "delta-verifier-fresh-lens-result"),
            ),
            RetryPolicy::new(),
        ))
        .expect("fresh lens result must succeed in the new iteration");
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream, verifier),
            RetryPolicy::new(),
        ))
        .expect("a fresh same-iteration lens result authorizes the verifier");
    }

    #[test]
    fn all_unaffected_delta_rejects_every_prior_iteration_assignment_result() {
        for stale_kind in [
            AssignmentKind::Lens,
            AssignmentKind::Verifier,
            AssignmentKind::RemediationVerifier,
        ] {
            let store = InMemoryEventStore::new();
            let fixture = fixture();
            let verifier_role = parsed("verifier", ModelRole::parse);
            let risk = RiskAssessment::parse(
                parsed("risk-evidence-stale-result", EvidenceId::parse),
                parsed("delta-reviewer", ModelRole::parse),
                vec![LensRoute::new(
                    fixture.lens.clone(),
                    fixture.reviewer_role.clone(),
                    VerifierRoute::Required {
                        model_role: verifier_role.clone(),
                    },
                    parsed("remediation-reviewer", ModelRole::parse),
                )],
                parsed("risk-agent", AgentId::parse),
                parsed("risk-reviewer", ModelRole::parse),
                parsed("risk-context", ContextReceiptId::parse),
                parsed("risk-life", LifecycleReceiptId::parse),
            )
            .expect("verifier-routed assessment must be valid");
            futures::executor::block_on(execute(
                &store,
                assess_risk(fixture.stream.clone(), fixture.snapshot.clone(), risk),
                RetryPolicy::new(),
            ))
            .expect("risk assessment must succeed");

            let (stale, current_iteration) = match stale_kind {
                AssignmentKind::Lens => (
                    assignment(
                        &fixture,
                        AssignmentAttempt::FIRST,
                        "all-unaffected-stale-lens",
                    ),
                    ReviewIteration::FIRST,
                ),
                AssignmentKind::Verifier => {
                    let lens = assignment(
                        &fixture,
                        AssignmentAttempt::FIRST,
                        "all-unaffected-verifier-prerequisite",
                    );
                    futures::executor::block_on(execute(
                        &store,
                        issue_assignment(fixture.stream.clone(), lens.clone()),
                        RetryPolicy::new(),
                    ))
                    .expect("verifier prerequisite assignment must succeed");
                    futures::executor::block_on(execute(
                        &store,
                        accept_assignment_result(
                            fixture.stream.clone(),
                            clean_result(&lens, "all-unaffected-verifier-prerequisite-result"),
                        ),
                        RetryPolicy::new(),
                    ))
                    .expect("verifier prerequisite result must succeed");
                    (
                        assignment_for_snapshot_lens_kind(
                            &fixture,
                            &fixture.snapshot,
                            &fixture.lens,
                            "all-unaffected-stale-verifier-agent",
                            &verifier_role,
                            ReviewIteration::FIRST,
                            AssignmentAttempt::FIRST,
                            AssignmentKind::Verifier,
                            "all-unaffected-stale-verifier",
                        ),
                        ReviewIteration::FIRST,
                    )
                }
                AssignmentKind::RemediationVerifier => {
                    let finding_assignment = assignment(
                        &fixture,
                        AssignmentAttempt::FIRST,
                        "all-unaffected-finding-assignment",
                    );
                    futures::executor::block_on(execute(
                        &store,
                        issue_assignment(fixture.stream.clone(), finding_assignment.clone()),
                        RetryPolicy::new(),
                    ))
                    .expect("finding assignment must succeed");
                    let finding_id = FindingOccurrenceId::new(
                        finding_assignment.id().clone(),
                        parsed("all-unaffected-blocker", EvidenceId::parse),
                    );
                    futures::executor::block_on(execute(
                        &store,
                        accept_assignment_result(
                            fixture.stream.clone(),
                            AssignmentResult::new(
                                finding_assignment.id().clone(),
                                finding_assignment.snapshot().clone(),
                                finding_assignment.agent_id().clone(),
                                finding_assignment.model_role().clone(),
                                finding_assignment.context_receipt().clone(),
                                finding_assignment.lifecycle_receipt().clone(),
                                parsed("all-unaffected-finding-result", EvidenceId::parse),
                                vec![FindingOccurrence::new(
                                    finding_id.clone(),
                                    FindingSeverity::Blocking,
                                )],
                            ),
                        ),
                        RetryPolicy::new(),
                    ))
                    .expect("blocking finding result must succeed");
                    let second_iteration =
                        ReviewIteration::parse(2).expect("second iteration must be bounded");
                    let fresh_lens = assignment_for_iteration(
                        &fixture,
                        second_iteration,
                        AssignmentAttempt::FIRST,
                        "all-unaffected-remediation-prerequisite",
                    );
                    futures::executor::block_on(execute(
                        &store,
                        issue_assignment(fixture.stream.clone(), fresh_lens.clone()),
                        RetryPolicy::new(),
                    ))
                    .expect("remediation prerequisite assignment must succeed");
                    futures::executor::block_on(execute(
                        &store,
                        accept_assignment_result(
                            fixture.stream.clone(),
                            clean_result(
                                &fresh_lens,
                                "all-unaffected-remediation-prerequisite-result",
                            ),
                        ),
                        RetryPolicy::new(),
                    ))
                    .expect("remediation prerequisite result must succeed");
                    let remediation_base = assignment_for_snapshot_lens_kind(
                        &fixture,
                        &fixture.snapshot,
                        &fixture.lens,
                        "all-unaffected-stale-remediation-agent",
                        &parsed("remediation-reviewer", ModelRole::parse),
                        second_iteration,
                        AssignmentAttempt::FIRST,
                        AssignmentKind::RemediationVerifier,
                        "all-unaffected-stale-remediation",
                    )
                    .with_finding_target(finding_id.clone());
                    (
                        ReviewAssignment::new(
                            remediation_base
                                .id()
                                .clone()
                                .with_occurrence_key(finding_id.evidence_id().clone()),
                            remediation_base.snapshot().clone(),
                            remediation_base.agent_id().clone(),
                            remediation_base.model_role().clone(),
                            remediation_base.context_receipt().clone(),
                            remediation_base.lifecycle_receipt().clone(),
                        )
                        .with_finding_target(finding_id),
                        second_iteration,
                    )
                }
                AssignmentKind::DeltaRisk => continue,
            };
            futures::executor::block_on(execute(
                &store,
                issue_assignment(fixture.stream.clone(), stale.clone()),
                RetryPolicy::new(),
            ))
            .expect("prior-snapshot assignment must succeed");

            let changed = parsed("all-unaffected-result-snapshot-b", ReviewSnapshotId::parse);
            let delta = assignment_for_snapshot_lens_kind(
                &fixture,
                &fixture.snapshot,
                &fixture.lens,
                "all-unaffected-result-delta-agent",
                &parsed("delta-reviewer", ModelRole::parse),
                current_iteration,
                AssignmentAttempt::FIRST,
                AssignmentKind::DeltaRisk,
                "all-unaffected-result-delta",
            )
            .with_target_snapshot(changed.clone());
            futures::executor::block_on(execute(
                &store,
                issue_assignment(fixture.stream.clone(), delta.clone()),
                RetryPolicy::new(),
            ))
            .expect("delta assignment must succeed");
            futures::executor::block_on(execute(
                &store,
                accept_assignment_result(
                    fixture.stream.clone(),
                    clean_result(&delta, "all-unaffected-result-delta-result")
                        .with_delta_classifications(vec![LensDeltaClassification::new(
                            fixture.lens.clone(),
                            false,
                        )]),
                ),
                RetryPolicy::new(),
            ))
            .expect("all-unaffected delta result must succeed");
            futures::executor::block_on(execute(
                &store,
                reassess_delta(
                    fixture.stream.clone(),
                    fixture.snapshot.clone(),
                    changed,
                    delta.id().clone(),
                ),
                RetryPolicy::new(),
            ))
            .expect("all-unaffected delta reassessment must succeed");

            let rejected = futures::executor::block_on(execute(
                &store,
                accept_assignment_result(
                    fixture.stream.clone(),
                    clean_result(&stale, "all-unaffected-stale-result"),
                ),
                RetryPolicy::new(),
            ));
            let _error = rejected.expect_err(
                "every assignment result from the prior snapshot and iteration must be rejected",
            );
        }
    }

    #[test]
    fn all_unaffected_delta_rejects_prior_iteration_finding_resolution() {
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

        let finding_assignment = assignment(
            &fixture,
            AssignmentAttempt::FIRST,
            "all-unaffected-resolution-finding",
        );
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), finding_assignment.clone()),
            RetryPolicy::new(),
        ))
        .expect("finding assignment must succeed");
        let finding_id = FindingOccurrenceId::new(
            finding_assignment.id().clone(),
            parsed("all-unaffected-resolution-blocker", EvidenceId::parse),
        );
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(
                fixture.stream.clone(),
                AssignmentResult::new(
                    finding_assignment.id().clone(),
                    finding_assignment.snapshot().clone(),
                    finding_assignment.agent_id().clone(),
                    finding_assignment.model_role().clone(),
                    finding_assignment.context_receipt().clone(),
                    finding_assignment.lifecycle_receipt().clone(),
                    parsed(
                        "all-unaffected-resolution-finding-result",
                        EvidenceId::parse,
                    ),
                    vec![FindingOccurrence::new(
                        finding_id.clone(),
                        FindingSeverity::Blocking,
                    )],
                ),
            ),
            RetryPolicy::new(),
        ))
        .expect("blocking finding result must succeed");

        let second_iteration = ReviewIteration::parse(2).expect("second iteration must be bounded");
        let fresh_lens = assignment_for_iteration(
            &fixture,
            second_iteration,
            AssignmentAttempt::FIRST,
            "all-unaffected-resolution-prerequisite",
        );
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), fresh_lens.clone()),
            RetryPolicy::new(),
        ))
        .expect("remediation prerequisite assignment must succeed");
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(
                fixture.stream.clone(),
                clean_result(&fresh_lens, "all-unaffected-resolution-prerequisite-result"),
            ),
            RetryPolicy::new(),
        ))
        .expect("remediation prerequisite result must succeed");
        let remediation_base = assignment_for_snapshot_lens_kind(
            &fixture,
            &fixture.snapshot,
            &fixture.lens,
            "all-unaffected-resolution-remediation-agent",
            &parsed("remediation-reviewer", ModelRole::parse),
            second_iteration,
            AssignmentAttempt::FIRST,
            AssignmentKind::RemediationVerifier,
            "all-unaffected-resolution-remediation",
        )
        .with_finding_target(finding_id.clone());
        let remediation = ReviewAssignment::new(
            remediation_base
                .id()
                .clone()
                .with_occurrence_key(finding_id.evidence_id().clone()),
            remediation_base.snapshot().clone(),
            remediation_base.agent_id().clone(),
            remediation_base.model_role().clone(),
            remediation_base.context_receipt().clone(),
            remediation_base.lifecycle_receipt().clone(),
        )
        .with_finding_target(finding_id.clone());
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), remediation.clone()),
            RetryPolicy::new(),
        ))
        .expect("remediation assignment must succeed");
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(
                fixture.stream.clone(),
                clean_result(&remediation, "all-unaffected-resolution-remediation-result"),
            ),
            RetryPolicy::new(),
        ))
        .expect("remediation result must succeed");

        let changed = parsed(
            "all-unaffected-resolution-snapshot-b",
            ReviewSnapshotId::parse,
        );
        let delta = assignment_for_snapshot_lens_kind(
            &fixture,
            &fixture.snapshot,
            &fixture.lens,
            "all-unaffected-resolution-delta-agent",
            &parsed("delta-reviewer", ModelRole::parse),
            second_iteration,
            AssignmentAttempt::FIRST,
            AssignmentKind::DeltaRisk,
            "all-unaffected-resolution-delta",
        )
        .with_target_snapshot(changed.clone());
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), delta.clone()),
            RetryPolicy::new(),
        ))
        .expect("delta assignment must succeed");
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(
                fixture.stream.clone(),
                clean_result(&delta, "all-unaffected-resolution-delta-result")
                    .with_delta_classifications(vec![LensDeltaClassification::new(
                        fixture.lens.clone(),
                        false,
                    )]),
            ),
            RetryPolicy::new(),
        ))
        .expect("all-unaffected delta result must succeed");
        futures::executor::block_on(execute(
            &store,
            reassess_delta(
                fixture.stream.clone(),
                fixture.snapshot,
                changed,
                delta.id().clone(),
            ),
            RetryPolicy::new(),
        ))
        .expect("all-unaffected delta reassessment must succeed");

        let rejected = futures::executor::block_on(execute(
            &store,
            verify_finding_resolution(fixture.stream, finding_id, remediation.id().clone()),
            RetryPolicy::new(),
        ));
        let _error = rejected.expect_err(
            "prior-snapshot remediation evidence cannot resolve a finding after any delta",
        );
    }

    #[test]
    fn all_unaffected_delta_rejects_prior_iteration_supersession() {
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
        let stale = assignment(
            &fixture,
            AssignmentAttempt::FIRST,
            "all-unaffected-supersession-stale",
        );
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), stale.clone()),
            RetryPolicy::new(),
        ))
        .expect("prior-snapshot assignment must succeed");

        let changed = parsed(
            "all-unaffected-supersession-snapshot-b",
            ReviewSnapshotId::parse,
        );
        let delta = assignment_for_snapshot_lens_kind(
            &fixture,
            &fixture.snapshot,
            &fixture.lens,
            "all-unaffected-supersession-delta-agent",
            &parsed("delta-reviewer", ModelRole::parse),
            ReviewIteration::FIRST,
            AssignmentAttempt::FIRST,
            AssignmentKind::DeltaRisk,
            "all-unaffected-supersession-delta",
        )
        .with_target_snapshot(changed.clone());
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), delta.clone()),
            RetryPolicy::new(),
        ))
        .expect("delta assignment must succeed");
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(
                fixture.stream.clone(),
                clean_result(&delta, "all-unaffected-supersession-delta-result")
                    .with_delta_classifications(vec![LensDeltaClassification::new(
                        fixture.lens.clone(),
                        false,
                    )]),
            ),
            RetryPolicy::new(),
        ))
        .expect("all-unaffected delta result must succeed");
        futures::executor::block_on(execute(
            &store,
            reassess_delta(
                fixture.stream.clone(),
                fixture.snapshot,
                changed,
                delta.id().clone(),
            ),
            RetryPolicy::new(),
        ))
        .expect("all-unaffected delta reassessment must succeed");

        let rejected = futures::executor::block_on(execute(
            &store,
            supersede_assignment(
                fixture.stream,
                stale.id().clone(),
                parsed("all-unaffected-stale-supersession", EvidenceId::parse),
            ),
            RetryPolicy::new(),
        ));
        let _error = rejected
            .expect_err("an assignment from the prior snapshot and iteration cannot be superseded");
    }

    #[test]
    fn all_unaffected_deltas_do_not_reuse_content_review_at_commit_boundary() {
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
        let rejected = futures::executor::block_on(execute(
            &store,
            accept_clean_review(
                fixture.stream,
                delivered,
                parsed("clean-commit", EvidenceId::parse),
            ),
            RetryPolicy::new(),
        ));
        let _error = rejected.expect_err(
            "content-identical snapshot boundaries still require current-iteration review work",
        );
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
    fn required_verifier_must_complete_every_clean_iteration() {
        let store = InMemoryEventStore::new();
        let fixture = fixture();
        let verifier_role = parsed("verifier", ModelRole::parse);
        let risk = RiskAssessment::parse(
            parsed("risk-evidence-required-verifier", EvidenceId::parse),
            parsed("delta-reviewer", ModelRole::parse),
            vec![LensRoute::new(
                fixture.lens.clone(),
                fixture.reviewer_role.clone(),
                VerifierRoute::Required {
                    model_role: verifier_role.clone(),
                },
                parsed("remediation-reviewer", ModelRole::parse),
            )],
            parsed("risk-agent", AgentId::parse),
            parsed("risk-reviewer", ModelRole::parse),
            parsed("risk-context", ContextReceiptId::parse),
            parsed("risk-life", LifecycleReceiptId::parse),
        )
        .expect("verifier-routed assessment must be valid");
        futures::executor::block_on(execute(
            &store,
            assess_risk(fixture.stream.clone(), fixture.snapshot.clone(), risk),
            RetryPolicy::new(),
        ))
        .expect("risk assessment must succeed");

        for iteration_number in 1..=3 {
            let iteration = ReviewIteration::parse(iteration_number).expect("pass is bounded");
            let lens_context = format!("verifier-lens-context-{iteration_number}");
            let lens = assignment_for_iteration(
                &fixture,
                iteration,
                AssignmentAttempt::FIRST,
                &lens_context,
            );
            futures::executor::block_on(execute(
                &store,
                issue_assignment(fixture.stream.clone(), lens.clone()),
                RetryPolicy::new(),
            ))
            .expect("lens assignment must succeed");
            futures::executor::block_on(execute(
                &store,
                accept_assignment_result(
                    fixture.stream.clone(),
                    clean_result(&lens, &format!("verifier-lens-result-{iteration_number}")),
                ),
                RetryPolicy::new(),
            ))
            .expect("lens result must succeed");

            let lens_only = futures::executor::block_on(execute(
                &store,
                accept_clean_review(
                    fixture.stream.clone(),
                    fixture.snapshot.clone(),
                    parsed(
                        &format!("verifier-lens-only-{iteration_number}"),
                        EvidenceId::parse,
                    ),
                ),
                RetryPolicy::new(),
            ));
            let _error = lens_only.expect_err("lens-only evidence cannot complete any iteration");

            let verifier = assignment_for_snapshot_lens_kind(
                &fixture,
                &fixture.snapshot,
                &fixture.lens,
                &format!("verifier-agent-{iteration_number}"),
                &verifier_role,
                iteration,
                AssignmentAttempt::FIRST,
                AssignmentKind::Verifier,
                &format!("verifier-context-{iteration_number}"),
            );
            futures::executor::block_on(execute(
                &store,
                issue_assignment(fixture.stream.clone(), verifier.clone()),
                RetryPolicy::new(),
            ))
            .expect("required verifier assignment must succeed after its lens result");
            futures::executor::block_on(execute(
                &store,
                accept_assignment_result(
                    fixture.stream.clone(),
                    clean_result(
                        &verifier,
                        &format!("required-verifier-result-{iteration_number}"),
                    ),
                ),
                RetryPolicy::new(),
            ))
            .expect("required verifier result must succeed");
            futures::executor::block_on(execute(
                &store,
                accept_clean_review(
                    fixture.stream.clone(),
                    fixture.snapshot.clone(),
                    parsed(
                        &format!("verifier-complete-{iteration_number}"),
                        EvidenceId::parse,
                    ),
                ),
                RetryPolicy::new(),
            ))
            .expect("lens and verifier together complete the iteration");
        }

        let fourth = assignment_for_iteration(
            &fixture,
            ReviewIteration::parse(4).expect("fourth iteration is bounded"),
            AssignmentAttempt::FIRST,
            "verifier-terminal-fourth",
        );
        let rejected = futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream, fourth),
            RetryPolicy::new(),
        ));
        let _error = rejected.expect_err("three lens-plus-verifier passes are terminal");
    }

    #[test]
    fn pending_delta_classification_blocks_the_terminal_clean_pass() {
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

        for iteration_number in 1..=2 {
            let iteration = ReviewIteration::parse(iteration_number).expect("pass is bounded");
            let lens = assignment_for_iteration(
                &fixture,
                iteration,
                AssignmentAttempt::FIRST,
                &format!("delta-race-lens-{iteration_number}"),
            );
            futures::executor::block_on(execute(
                &store,
                issue_assignment(fixture.stream.clone(), lens.clone()),
                RetryPolicy::new(),
            ))
            .expect("pre-delta lens assignment must succeed");
            futures::executor::block_on(execute(
                &store,
                accept_assignment_result(
                    fixture.stream.clone(),
                    clean_result(&lens, &format!("delta-race-result-{iteration_number}")),
                ),
                RetryPolicy::new(),
            ))
            .expect("pre-delta result must succeed");
            futures::executor::block_on(execute(
                &store,
                accept_clean_review(
                    fixture.stream.clone(),
                    fixture.snapshot.clone(),
                    parsed(
                        &format!("delta-race-pass-{iteration_number}"),
                        EvidenceId::parse,
                    ),
                ),
                RetryPolicy::new(),
            ))
            .expect("pre-delta pass must be recorded");
        }

        let third = assignment_for_iteration(
            &fixture,
            ReviewIteration::parse(3).expect("third iteration is bounded"),
            AssignmentAttempt::FIRST,
            "delta-race-lens-3",
        );
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), third.clone()),
            RetryPolicy::new(),
        ))
        .expect("third lens assignment must succeed");
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(
                fixture.stream.clone(),
                clean_result(&third, "delta-race-result-3"),
            ),
            RetryPolicy::new(),
        ))
        .expect("third lens result must succeed");

        let changed = parsed("delta-race-snapshot-b", ReviewSnapshotId::parse);
        let delta = assignment_for_snapshot_lens_kind(
            &fixture,
            &fixture.snapshot,
            &fixture.lens,
            "delta-race-agent",
            &parsed("delta-reviewer", ModelRole::parse),
            ReviewIteration::parse(3).expect("third iteration is bounded"),
            AssignmentAttempt::FIRST,
            AssignmentKind::DeltaRisk,
            "delta-race-context",
        )
        .with_target_snapshot(changed.clone());
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), delta.clone()),
            RetryPolicy::new(),
        ))
        .expect("delta assignment must succeed");

        let pending = futures::executor::block_on(execute(
            &store,
            accept_clean_review(
                fixture.stream.clone(),
                fixture.snapshot.clone(),
                parsed("delta-race-premature-clean", EvidenceId::parse),
            ),
            RetryPolicy::new(),
        ));
        let _error = pending.expect_err("issued delta work must fence terminal clean acceptance");

        let delta_result =
            clean_result(&delta, "delta-race-classification").with_delta_classifications(vec![
                LensDeltaClassification::new(fixture.lens.clone(), true),
            ]);
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(fixture.stream.clone(), delta_result),
            RetryPolicy::new(),
        ))
        .expect("delta classification result must succeed");
        let unapplied = futures::executor::block_on(execute(
            &store,
            accept_clean_review(
                fixture.stream.clone(),
                fixture.snapshot.clone(),
                parsed("delta-race-unapplied-clean", EvidenceId::parse),
            ),
            RetryPolicy::new(),
        ));
        let _error = unapplied.expect_err("accepted but unapplied delta evidence remains pending");

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
        .expect("delta reassessment must succeed");
        let fresh = assignment_for_snapshot_lens_kind(
            &fixture,
            &changed,
            &fixture.lens,
            "delta-race-fresh-agent",
            &fixture.reviewer_role,
            ReviewIteration::parse(4).expect("fourth iteration is bounded"),
            AssignmentAttempt::FIRST,
            AssignmentKind::Lens,
            "delta-race-fresh-context",
        );
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream, fresh),
            RetryPolicy::new(),
        ))
        .expect("material delta requires fresh post-delta work");
    }

    #[test]
    fn closed_iteration_rejects_pending_delta_reassessment() {
        let store = InMemoryEventStore::new();
        let fixture = fixture();
        let architecture = parsed("architecture", ReviewLens::parse);
        let risk = RiskAssessment::parse(
            parsed("risk-evidence-stale-delta", EvidenceId::parse),
            parsed("delta-reviewer", ModelRole::parse),
            vec![
                LensRoute::new(
                    fixture.lens.clone(),
                    fixture.reviewer_role.clone(),
                    VerifierRoute::NotRequired,
                    parsed("remediation-reviewer", ModelRole::parse),
                ),
                LensRoute::new(
                    architecture.clone(),
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
            assess_risk(fixture.stream.clone(), fixture.snapshot.clone(), risk),
            RetryPolicy::new(),
        ))
        .expect("risk assessment must succeed");

        let peer = assignment_for_lens_iteration(
            &fixture,
            &architecture,
            "stale-delta-peer-agent",
            ReviewIteration::FIRST,
            AssignmentAttempt::FIRST,
            "stale-delta-peer",
        );
        let changed = parsed("stale-delta-snapshot-b", ReviewSnapshotId::parse);
        let delta = assignment_for_snapshot_lens_kind(
            &fixture,
            &fixture.snapshot,
            &fixture.lens,
            "stale-delta-agent",
            &parsed("delta-reviewer", ModelRole::parse),
            ReviewIteration::FIRST,
            AssignmentAttempt::FIRST,
            AssignmentKind::DeltaRisk,
            "stale-delta-context",
        )
        .with_target_snapshot(changed.clone());
        for issued in [&peer, &delta] {
            futures::executor::block_on(execute(
                &store,
                issue_assignment(fixture.stream.clone(), issued.clone()),
                RetryPolicy::new(),
            ))
            .expect("concurrent peer and delta assignments must succeed");
        }
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(
                fixture.stream.clone(),
                clean_result(&delta, "stale-delta-result").with_delta_classifications(vec![
                    LensDeltaClassification::new(fixture.lens.clone(), true),
                    LensDeltaClassification::new(architecture, false),
                ]),
            ),
            RetryPolicy::new(),
        ))
        .expect("delta classification result must be accepted before iteration closure");
        let peer_finding = FindingOccurrenceId::new(
            peer.id().clone(),
            parsed("stale-delta-peer-finding", EvidenceId::parse),
        );
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(
                fixture.stream.clone(),
                AssignmentResult::new(
                    peer.id().clone(),
                    peer.snapshot().clone(),
                    peer.agent_id().clone(),
                    peer.model_role().clone(),
                    peer.context_receipt().clone(),
                    peer.lifecycle_receipt().clone(),
                    parsed("stale-delta-peer-result", EvidenceId::parse),
                    vec![FindingOccurrence::new(
                        peer_finding,
                        FindingSeverity::Observation,
                    )],
                ),
            ),
            RetryPolicy::new(),
        ))
        .expect("finding-bearing peer result must close the iteration");

        let stale = futures::executor::block_on(execute(
            &store,
            reassess_delta(
                fixture.stream,
                fixture.snapshot,
                changed,
                delta.id().clone(),
            ),
            RetryPolicy::new(),
        ));
        let _error =
            stale.expect_err("delta evidence from a closed iteration cannot mutate the snapshot");
    }

    #[test]
    fn closed_iteration_rejects_pending_finding_resolution() {
        for malformed_peer in [false, true] {
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

            let original = assignment_for_iteration(
                &fixture,
                ReviewIteration::FIRST,
                AssignmentAttempt::FIRST,
                "stale-resolution-original",
            );
            futures::executor::block_on(execute(
                &store,
                issue_assignment(fixture.stream.clone(), original.clone()),
                RetryPolicy::new(),
            ))
            .expect("original finding assignment must succeed");
            let finding_id = FindingOccurrenceId::new(
                original.id().clone(),
                parsed("stale-resolution-blocker", EvidenceId::parse),
            );
            futures::executor::block_on(execute(
                &store,
                accept_assignment_result(
                    fixture.stream.clone(),
                    AssignmentResult::new(
                        original.id().clone(),
                        original.snapshot().clone(),
                        original.agent_id().clone(),
                        original.model_role().clone(),
                        original.context_receipt().clone(),
                        original.lifecycle_receipt().clone(),
                        parsed("stale-resolution-original-result", EvidenceId::parse),
                        vec![FindingOccurrence::new(
                            finding_id.clone(),
                            FindingSeverity::Blocking,
                        )],
                    ),
                ),
                RetryPolicy::new(),
            ))
            .expect("blocking finding must be accepted");

            let second_iteration = ReviewIteration::parse(2).expect("second iteration is bounded");
            let fresh_review = assignment_for_iteration(
                &fixture,
                second_iteration,
                AssignmentAttempt::FIRST,
                "stale-resolution-fresh-review",
            );
            futures::executor::block_on(execute(
                &store,
                issue_assignment(fixture.stream.clone(), fresh_review.clone()),
                RetryPolicy::new(),
            ))
            .expect("fresh post-finding review assignment must succeed");
            futures::executor::block_on(execute(
                &store,
                accept_assignment_result(
                    fixture.stream.clone(),
                    clean_result(&fresh_review, "stale-resolution-fresh-review-result"),
                ),
                RetryPolicy::new(),
            ))
            .expect("fresh post-finding review evidence must succeed");
            let remediation_base = assignment_for_snapshot_lens_kind(
                &fixture,
                &fixture.snapshot,
                &fixture.lens,
                "stale-resolution-remediation-agent",
                &parsed("remediation-reviewer", ModelRole::parse),
                second_iteration,
                AssignmentAttempt::FIRST,
                AssignmentKind::RemediationVerifier,
                "stale-resolution-remediation",
            )
            .with_finding_target(finding_id.clone());
            let remediation = ReviewAssignment::new(
                remediation_base
                    .id()
                    .clone()
                    .with_occurrence_key(finding_id.evidence_id().clone()),
                remediation_base.snapshot().clone(),
                remediation_base.agent_id().clone(),
                remediation_base.model_role().clone(),
                remediation_base.context_receipt().clone(),
                remediation_base.lifecycle_receipt().clone(),
            )
            .with_finding_target(finding_id.clone());
            let closing_delta = assignment_for_snapshot_lens_kind(
                &fixture,
                &fixture.snapshot,
                &fixture.lens,
                "stale-resolution-delta-agent",
                &parsed("delta-reviewer", ModelRole::parse),
                second_iteration,
                AssignmentAttempt::FIRST,
                AssignmentKind::DeltaRisk,
                "stale-resolution-delta",
            )
            .with_target_snapshot(parsed(
                "stale-resolution-snapshot-b",
                ReviewSnapshotId::parse,
            ));
            futures::executor::block_on(execute(
                &store,
                issue_assignment(fixture.stream.clone(), remediation.clone()),
                RetryPolicy::new(),
            ))
            .expect("remediation assignment must succeed");
            futures::executor::block_on(execute(
                &store,
                accept_assignment_result(
                    fixture.stream.clone(),
                    clean_result(&remediation, "stale-resolution-remediation-result"),
                ),
                RetryPolicy::new(),
            ))
            .expect("remediation result must be accepted before peer closure");
            futures::executor::block_on(execute(
                &store,
                issue_assignment(fixture.stream.clone(), closing_delta.clone()),
                RetryPolicy::new(),
            ))
            .expect("closing delta assignment must succeed after remediation evidence");

            let closing_result = if malformed_peer {
                AssignmentResult::new(
                    closing_delta.id().clone(),
                    closing_delta.snapshot().clone(),
                    closing_delta.agent_id().clone(),
                    parsed("wrong-role", ModelRole::parse),
                    closing_delta.context_receipt().clone(),
                    closing_delta.lifecycle_receipt().clone(),
                    parsed("stale-resolution-malformed-peer", EvidenceId::parse),
                    vec![],
                )
            } else {
                let peer_finding = FindingOccurrenceId::new(
                    closing_delta.id().clone(),
                    parsed("stale-resolution-peer-finding", EvidenceId::parse),
                );
                AssignmentResult::new(
                    closing_delta.id().clone(),
                    closing_delta.snapshot().clone(),
                    closing_delta.agent_id().clone(),
                    closing_delta.model_role().clone(),
                    closing_delta.context_receipt().clone(),
                    closing_delta.lifecycle_receipt().clone(),
                    parsed("stale-resolution-finding-peer", EvidenceId::parse),
                    vec![FindingOccurrence::new(
                        peer_finding,
                        FindingSeverity::Observation,
                    )],
                )
                .with_delta_classifications(vec![LensDeltaClassification::new(
                    fixture.lens.clone(),
                    true,
                )])
            };
            futures::executor::block_on(execute(
                &store,
                accept_assignment_result(fixture.stream.clone(), closing_result),
                RetryPolicy::new(),
            ))
            .expect("peer evidence must durably close the remediation iteration");

            let stale = futures::executor::block_on(execute(
                &store,
                verify_finding_resolution(fixture.stream, finding_id, remediation.id().clone()),
                RetryPolicy::new(),
            ));
            let _error =
                stale.expect_err("closed-iteration remediation evidence cannot resolve a finding");
        }
    }

    #[test]
    fn blocker_survives_peer_rejection_and_delta_until_exact_resolution() {
        let store = InMemoryEventStore::new();
        let fixture = fixture();
        let architecture = parsed("architecture", ReviewLens::parse);
        let risk = RiskAssessment::parse(
            parsed("risk-evidence-durable-blocker", EvidenceId::parse),
            parsed("delta-reviewer", ModelRole::parse),
            vec![
                LensRoute::new(
                    fixture.lens.clone(),
                    fixture.reviewer_role.clone(),
                    VerifierRoute::NotRequired,
                    parsed("remediation-reviewer", ModelRole::parse),
                ),
                LensRoute::new(
                    architecture.clone(),
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
            assess_risk(fixture.stream.clone(), fixture.snapshot.clone(), risk),
            RetryPolicy::new(),
        ))
        .expect("risk assessment must succeed");

        let correctness = assignment_for_iteration(
            &fixture,
            ReviewIteration::FIRST,
            AssignmentAttempt::FIRST,
            "durable-blocker-correctness",
        );
        let architecture_assignment = assignment_for_lens_iteration(
            &fixture,
            &architecture,
            "durable-blocker-architecture-agent",
            ReviewIteration::FIRST,
            AssignmentAttempt::FIRST,
            "durable-blocker-architecture",
        );
        for issued in [&correctness, &architecture_assignment] {
            futures::executor::block_on(execute(
                &store,
                issue_assignment(fixture.stream.clone(), issued.clone()),
                RetryPolicy::new(),
            ))
            .expect("first-iteration assignment must succeed");
        }
        let finding_id = FindingOccurrenceId::new(
            correctness.id().clone(),
            parsed("durable-blocker", EvidenceId::parse),
        );
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(
                fixture.stream.clone(),
                AssignmentResult::new(
                    correctness.id().clone(),
                    correctness.snapshot().clone(),
                    correctness.agent_id().clone(),
                    correctness.model_role().clone(),
                    correctness.context_receipt().clone(),
                    correctness.lifecycle_receipt().clone(),
                    parsed("durable-blocker-result", EvidenceId::parse),
                    vec![FindingOccurrence::new(
                        finding_id.clone(),
                        FindingSeverity::Blocking,
                    )],
                ),
            ),
            RetryPolicy::new(),
        ))
        .expect("blocking result must be accepted");
        let stale_peer = futures::executor::block_on(execute(
            &store,
            accept_assignment_result(
                fixture.stream.clone(),
                AssignmentResult::new(
                    architecture_assignment.id().clone(),
                    architecture_assignment.snapshot().clone(),
                    architecture_assignment.agent_id().clone(),
                    parsed("wrong-role", ModelRole::parse),
                    architecture_assignment.context_receipt().clone(),
                    architecture_assignment.lifecycle_receipt().clone(),
                    parsed("durable-blocker-malformed-peer", EvidenceId::parse),
                    vec![],
                ),
            ),
            RetryPolicy::new(),
        ));
        let _error = stale_peer.expect_err("the finding must already have invalidated its peer");

        let changed = parsed("durable-blocker-snapshot-b", ReviewSnapshotId::parse);
        let delta = assignment_for_snapshot_lens_kind(
            &fixture,
            &fixture.snapshot,
            &fixture.lens,
            "durable-blocker-delta-agent",
            &parsed("delta-reviewer", ModelRole::parse),
            ReviewIteration::parse(2).expect("second iteration is bounded"),
            AssignmentAttempt::FIRST,
            AssignmentKind::DeltaRisk,
            "durable-blocker-delta",
        )
        .with_target_snapshot(changed.clone());
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), delta.clone()),
            RetryPolicy::new(),
        ))
        .expect("delta assignment must succeed after the finding reset");
        let delta_result = clean_result(&delta, "durable-blocker-delta-result")
            .with_delta_classifications(vec![
                LensDeltaClassification::new(fixture.lens.clone(), true),
                LensDeltaClassification::new(architecture.clone(), false),
            ]);
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(fixture.stream.clone(), delta_result),
            RetryPolicy::new(),
        ))
        .expect("delta result must succeed");
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
        .expect("material delta must succeed");

        let iteration = ReviewIteration::parse(3).expect("third iteration is bounded");
        let post_delta_correctness = assignment_for_snapshot_lens_kind(
            &fixture,
            &changed,
            &fixture.lens,
            "post-delta-correctness-agent",
            &fixture.reviewer_role,
            iteration,
            AssignmentAttempt::FIRST,
            AssignmentKind::Lens,
            "post-delta-correctness",
        );
        let post_delta_architecture = assignment_for_snapshot_lens_kind(
            &fixture,
            &changed,
            &architecture,
            "post-delta-architecture-agent",
            &fixture.reviewer_role,
            iteration,
            AssignmentAttempt::FIRST,
            AssignmentKind::Lens,
            "post-delta-architecture",
        );
        for (issued, evidence) in [
            (&post_delta_correctness, "post-delta-correctness-result"),
            (&post_delta_architecture, "post-delta-architecture-result"),
        ] {
            futures::executor::block_on(execute(
                &store,
                issue_assignment(fixture.stream.clone(), issued.clone()),
                RetryPolicy::new(),
            ))
            .expect("post-delta lens assignment must succeed");
            futures::executor::block_on(execute(
                &store,
                accept_assignment_result(fixture.stream.clone(), clean_result(issued, evidence)),
                RetryPolicy::new(),
            ))
            .expect("post-delta clean result must succeed");
        }
        let blocked = futures::executor::block_on(execute(
            &store,
            accept_clean_review(
                fixture.stream.clone(),
                changed.clone(),
                parsed("durable-blocker-premature-clean", EvidenceId::parse),
            ),
            RetryPolicy::new(),
        ));
        let _error = blocked.expect_err("the original exact blocker must survive every reset");

        let remediation_base = assignment_for_snapshot_lens_kind(
            &fixture,
            &changed,
            &fixture.lens,
            "durable-blocker-remediation-agent",
            &parsed("remediation-reviewer", ModelRole::parse),
            iteration,
            AssignmentAttempt::FIRST,
            AssignmentKind::RemediationVerifier,
            "durable-blocker-remediation",
        )
        .with_finding_target(finding_id.clone());
        let remediation = ReviewAssignment::new(
            remediation_base
                .id()
                .clone()
                .with_occurrence_key(finding_id.evidence_id().clone()),
            remediation_base.snapshot().clone(),
            remediation_base.agent_id().clone(),
            remediation_base.model_role().clone(),
            remediation_base.context_receipt().clone(),
            remediation_base.lifecycle_receipt().clone(),
        )
        .with_finding_target(finding_id.clone());
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), remediation.clone()),
            RetryPolicy::new(),
        ))
        .expect("the persisted blocker must authorize exact remediation work");
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(
                fixture.stream.clone(),
                clean_result(&remediation, "durable-blocker-remediation-result"),
            ),
            RetryPolicy::new(),
        ))
        .expect("remediation verifier result must succeed");
        futures::executor::block_on(execute(
            &store,
            verify_finding_resolution(fixture.stream.clone(), finding_id, remediation.id().clone()),
            RetryPolicy::new(),
        ))
        .expect("only exact verified resolution removes the blocker");
        futures::executor::block_on(execute(
            &store,
            accept_clean_review(
                fixture.stream,
                changed,
                parsed("durable-blocker-resolved-pass", EvidenceId::parse),
            ),
            RetryPolicy::new(),
        ))
        .expect("the complete clean pass is authorized after exact resolution");
    }

    #[test]
    fn material_delta_invalidates_an_unaffected_open_peer() {
        let store = InMemoryEventStore::new();
        let fixture = fixture();
        let architecture = parsed("architecture", ReviewLens::parse);
        let risk = RiskAssessment::parse(
            parsed("risk-evidence-delta-peer", EvidenceId::parse),
            parsed("delta-reviewer", ModelRole::parse),
            vec![
                LensRoute::new(
                    fixture.lens.clone(),
                    fixture.reviewer_role.clone(),
                    VerifierRoute::NotRequired,
                    parsed("remediation-reviewer", ModelRole::parse),
                ),
                LensRoute::new(
                    architecture.clone(),
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
            assess_risk(fixture.stream.clone(), fixture.snapshot.clone(), risk),
            RetryPolicy::new(),
        ))
        .expect("risk assessment must succeed");

        let affected = assignment_for_iteration(
            &fixture,
            ReviewIteration::FIRST,
            AssignmentAttempt::FIRST,
            "delta-peer-affected",
        );
        let unaffected = assignment_for_lens_iteration(
            &fixture,
            &architecture,
            "delta-peer-unaffected-agent",
            ReviewIteration::FIRST,
            AssignmentAttempt::FIRST,
            "delta-peer-unaffected",
        );
        for issued in [&affected, &unaffected] {
            futures::executor::block_on(execute(
                &store,
                issue_assignment(fixture.stream.clone(), issued.clone()),
                RetryPolicy::new(),
            ))
            .expect("peer assignment must succeed");
        }

        let changed = parsed("delta-peer-snapshot-b", ReviewSnapshotId::parse);
        let delta = assignment_for_snapshot_lens_kind(
            &fixture,
            &fixture.snapshot,
            &fixture.lens,
            "delta-peer-agent",
            &parsed("delta-reviewer", ModelRole::parse),
            ReviewIteration::FIRST,
            AssignmentAttempt::FIRST,
            AssignmentKind::DeltaRisk,
            "delta-peer-classification",
        )
        .with_target_snapshot(changed.clone());
        futures::executor::block_on(execute(
            &store,
            issue_assignment(fixture.stream.clone(), delta.clone()),
            RetryPolicy::new(),
        ))
        .expect("delta assignment must succeed");
        let delta_result =
            clean_result(&delta, "delta-peer-result").with_delta_classifications(vec![
                LensDeltaClassification::new(fixture.lens.clone(), true),
                LensDeltaClassification::new(architecture, false),
            ]);
        futures::executor::block_on(execute(
            &store,
            accept_assignment_result(fixture.stream.clone(), delta_result),
            RetryPolicy::new(),
        ))
        .expect("delta result must succeed");
        futures::executor::block_on(execute(
            &store,
            reassess_delta(
                fixture.stream.clone(),
                fixture.snapshot,
                changed,
                delta.id().clone(),
            ),
            RetryPolicy::new(),
        ))
        .expect("material delta must succeed");

        let stale_unaffected = futures::executor::block_on(execute(
            &store,
            accept_assignment_result(
                fixture.stream.clone(),
                clean_result(&unaffected, "delta-peer-stale-result"),
            ),
            RetryPolicy::new(),
        ));
        let _error = stale_unaffected
            .expect_err("the whole invalidated iteration closes even an unaffected peer");
        let stale_supersession = futures::executor::block_on(execute(
            &store,
            supersede_assignment(
                fixture.stream,
                unaffected.id().clone(),
                parsed("delta-peer-stale-supersession", EvidenceId::parse),
            ),
            RetryPolicy::new(),
        ));
        let _error = stale_supersession
            .expect_err("a material delta closes supersession for every open peer");
    }

    #[test]
    fn delta_after_two_clean_passes_requires_three_fresh_complete_passes() {
        for delta_affected in [true, false] {
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

            for iteration_number in 1..=2 {
                let iteration = ReviewIteration::parse(iteration_number).expect("pass is bounded");
                let lens = assignment_for_iteration(
                    &fixture,
                    iteration,
                    AssignmentAttempt::FIRST,
                    &format!("pre-material-lens-{iteration_number}"),
                );
                futures::executor::block_on(execute(
                    &store,
                    issue_assignment(fixture.stream.clone(), lens.clone()),
                    RetryPolicy::new(),
                ))
                .expect("pre-material assignment must succeed");
                futures::executor::block_on(execute(
                    &store,
                    accept_assignment_result(
                        fixture.stream.clone(),
                        clean_result(&lens, &format!("pre-material-result-{iteration_number}")),
                    ),
                    RetryPolicy::new(),
                ))
                .expect("pre-material result must succeed");
                futures::executor::block_on(execute(
                    &store,
                    accept_clean_review(
                        fixture.stream.clone(),
                        fixture.snapshot.clone(),
                        parsed(
                            &format!("pre-material-pass-{iteration_number}"),
                            EvidenceId::parse,
                        ),
                    ),
                    RetryPolicy::new(),
                ))
                .expect("pre-material clean pass must be recorded");
            }

            let changed = parsed("three-fresh-snapshot-b", ReviewSnapshotId::parse);
            let delta = assignment_for_snapshot_lens_kind(
                &fixture,
                &fixture.snapshot,
                &fixture.lens,
                "three-fresh-delta-agent",
                &parsed("delta-reviewer", ModelRole::parse),
                ReviewIteration::parse(3).expect("third iteration is bounded"),
                AssignmentAttempt::FIRST,
                AssignmentKind::DeltaRisk,
                "three-fresh-delta",
            )
            .with_target_snapshot(changed.clone());
            futures::executor::block_on(execute(
                &store,
                issue_assignment(fixture.stream.clone(), delta.clone()),
                RetryPolicy::new(),
            ))
            .expect("material delta assignment must succeed");
            let delta_result =
                clean_result(&delta, "three-fresh-delta-result").with_delta_classifications(vec![
                    LensDeltaClassification::new(fixture.lens.clone(), delta_affected),
                ]);
            futures::executor::block_on(execute(
                &store,
                accept_assignment_result(fixture.stream.clone(), delta_result),
                RetryPolicy::new(),
            ))
            .expect("material delta result must succeed");
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
            .expect("material delta reassessment must succeed");

            for iteration_number in 4..=6 {
                let iteration = ReviewIteration::parse(iteration_number).expect("pass is bounded");
                let lens = assignment_for_snapshot_lens_kind(
                    &fixture,
                    &changed,
                    &fixture.lens,
                    &format!("post-material-agent-{iteration_number}"),
                    &fixture.reviewer_role,
                    iteration,
                    AssignmentAttempt::FIRST,
                    AssignmentKind::Lens,
                    &format!("post-material-context-{iteration_number}"),
                );
                futures::executor::block_on(execute(
                    &store,
                    issue_assignment(fixture.stream.clone(), lens.clone()),
                    RetryPolicy::new(),
                ))
                .expect("fresh post-material assignment must succeed");
                futures::executor::block_on(execute(
                    &store,
                    accept_assignment_result(
                        fixture.stream.clone(),
                        clean_result(&lens, &format!("post-material-result-{iteration_number}")),
                    ),
                    RetryPolicy::new(),
                ))
                .expect("fresh post-material result must succeed");
                futures::executor::block_on(execute(
                    &store,
                    accept_clean_review(
                        fixture.stream.clone(),
                        changed.clone(),
                        parsed(
                            &format!("post-material-pass-{iteration_number}"),
                            EvidenceId::parse,
                        ),
                    ),
                    RetryPolicy::new(),
                ))
                .expect("fresh post-material pass must be recorded");
            }

            let seventh = assignment_for_snapshot_lens_kind(
                &fixture,
                &changed,
                &fixture.lens,
                "post-material-terminal-agent",
                &fixture.reviewer_role,
                ReviewIteration::parse(7).expect("seventh iteration is bounded"),
                AssignmentAttempt::FIRST,
                AssignmentKind::Lens,
                "post-material-terminal-context",
            );
            let rejected = futures::executor::block_on(execute(
                &store,
                issue_assignment(fixture.stream, seventh),
                RetryPolicy::new(),
            ));
            let _error = rejected.expect_err("only three fresh post-delta passes are terminal");
        }
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
