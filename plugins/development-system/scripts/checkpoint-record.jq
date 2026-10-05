  def exact_keys($expected): (keys | sort) == ($expected | sort);
  def string_or_null: type == "string" or . == null;
  def nonblank: type == "string" and test("\\S");
  def nonblank_or_null: . == null or nonblank;
  def oid: type == "string" and test("^[0-9a-f]{40}([0-9a-f]{24})?$");
  def sha256: type == "string" and test("^[0-9a-f]{64}$");
  . as $record |
  exact_keys(["generation", "predecessor_sha256", "baseline_oid", "snapshot", "state", "test", "gates", "delivery", "ci", "next_action"]) and
  .generation == $generation and
  (if $generation == 0 then .predecessor_sha256 == null else .predecessor_sha256 == $predecessor end) and
  (.baseline_oid | oid) and
  (.snapshot | exact_keys(["head_oid", "tracked_sha256", "untracked_sha256"]) and (.head_oid | oid) and (.tracked_sha256 | sha256) and (.untracked_sha256 | sha256)) and
  .snapshot.head_oid == $current_head and .snapshot.tracked_sha256 == $current_tracked and .snapshot.untracked_sha256 == $current_untracked and
  (.state | IN("failing", "passing-awaiting-gates-or-review", "committed", "pushed-or-delivery-mode-equivalent")) and
  (.test == null or (.test | exact_keys(["command", "receipt_ref", "outcome", "failure_kind"]) and (.command | nonblank) and (.receipt_ref | nonblank) and (.outcome | IN("pass", "fail")) and (.failure_kind | string_or_null))) and
  (.gates | exact_keys(["lightweight_review_receipt", "fast_gate_receipt", "exact_identity_verification_receipt"]) and
    (.lightweight_review_receipt | nonblank_or_null) and (.fast_gate_receipt | nonblank_or_null) and
    (.exact_identity_verification_receipt == null or
      (.exact_identity_verification_receipt | exact_keys(["receipt_ref", "outcome"]) and
       (.receipt_ref | nonblank) and (.outcome | IN("pass", "fail"))))) and
  (.delivery == null or (.delivery | exact_keys(["mode", "commit_oid", "pushed_oid", "local_snapshot"]) and (.mode | IN("local-only", "direct-to-trunk", "pull-request")) and (.commit_oid | . == null or oid) and (.pushed_oid | . == null or oid) and (.local_snapshot | nonblank_or_null))) and
  (.ci | exact_keys(["runs", "terminal_success_run_id"]) and (.runs | type == "array") and all(.runs[]; exact_keys(["provider", "run_id", "commit_oid", "status"]) and (.provider | nonblank) and (.run_id | nonblank) and (.commit_oid | oid) and (.status | IN("queued", "running", "success", "failure"))) and (.terminal_success_run_id | nonblank_or_null)) and
  (.next_action | nonblank) and
  ($generation != 0 or .state == "pushed-or-delivery-mode-equivalent") and
  (.ci.terminal_success_run_id == null or
   ((.ci.runs | length) > 0 and .ci.runs[-1].run_id == .ci.terminal_success_run_id and
    .ci.runs[-1].status == "success" and .ci.runs[-1].commit_oid == $record.delivery.pushed_oid)) and
  (if .state == "failing" then
     .test != null and .delivery == null and all(.gates[]; . == null) and
     (if .test.outcome == "pass" then
        .test.failure_kind == "invalid-test" and (.next_action | test("^rewrite-invalid-test: \\S"))
      else
        (.test.failure_kind | nonblank) and (.next_action | test("^causal-edit: \\S"))
      end)
   elif .state == "passing-awaiting-gates-or-review" then
     .test != null and .test.outcome == "pass" and .delivery == null and .gates.exact_identity_verification_receipt == null and
     (if .gates.lightweight_review_receipt == null then
        .gates.fast_gate_receipt == null and .next_action == "lightweight-review"
      elif .gates.fast_gate_receipt == null then
        (.next_action | IN("fast-gate", "commit-through-pre-commit-hook"))
      else .next_action == "commit-or-record-local-snapshot" end)
   elif .state == "committed" then
     .test != null and .test.outcome == "pass" and .delivery != null and .delivery.commit_oid == .snapshot.head_oid and
     (.gates.lightweight_review_receipt | type == "string") and
     (.gates.fast_gate_receipt | type == "string") and
     ((.gates.exact_identity_verification_receipt == null and .next_action == "verify-exact-commit") or
      (.gates.exact_identity_verification_receipt.outcome == "pass" and
       (if .delivery.mode == "local-only" then .next_action == "record-local-delivery" else .next_action == "push" end)) or
      (.gates.exact_identity_verification_receipt.outcome == "fail" and .next_action == "repair-exact-identity-verification"))
   elif .state == "pushed-or-delivery-mode-equivalent" and .generation == 0 then
     .baseline_oid == .snapshot.head_oid and
     .snapshot.tracked_sha256 == "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855" and
     .snapshot.untracked_sha256 == "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855" and
     .test == null and all(.gates[]; . == null) and .delivery != null and
     (.next_action | test("^causal-edit: \\S")) and
     (if .delivery.mode == "local-only" then
        .delivery.pushed_oid == null and (.delivery.local_snapshot | type == "string") and
        (.ci.runs | length) == 0 and .ci.terminal_success_run_id == null
      else .delivery.local_snapshot == null and .delivery.pushed_oid == .snapshot.head_oid end)
   else
     .test != null and .test.outcome == "pass" and .delivery != null and
     (.gates.lightweight_review_receipt | type == "string") and
     (.gates.fast_gate_receipt | type == "string") and
     .gates.exact_identity_verification_receipt.outcome == "pass" and
     (if .delivery.mode == "local-only" then
        .delivery.pushed_oid == null and (.delivery.local_snapshot | type == "string") and
        (.ci.runs | length) == 0 and .ci.terminal_success_run_id == null and (.next_action | IN("terminal-review", "complete"))
      else
        .delivery.local_snapshot == null and .delivery.commit_oid == .snapshot.head_oid and
        .delivery.pushed_oid == .snapshot.head_oid and
        ((.ci.runs | map(select(.commit_oid == $record.delivery.pushed_oid))) as $current_runs |
         if ($current_runs | length) == 0 then .next_action == "register-exact-sha-ci-monitor"
         elif .ci.terminal_success_run_id != null then (.next_action | IN("terminal-review", "complete"))
         elif any($current_runs[]; .status == "failure") then .next_action == "enter-ci-recovery"
         else .next_action == "monitor-exact-sha-ci" end)
      end)
   end)
