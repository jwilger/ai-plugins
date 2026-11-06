# Preserve trunk WIP during CI recovery

Use this protocol in the single configured trunk checkout. Keep the existing
Tiber incident owner, epoch, and global hold; a stash does not grant recovery
authority. Serialize all mutation, including helpers and formatters. Read-only
review can continue. Do not create branches, worktrees, or parallel source
checkouts without explicit user authorization.

1. Before changing Git state, record recovery identity under Git metadata or
   another durable local recovery location outside the worktree: incident/epoch,
   checkout path, branch, original full HEAD, configured origin/ref, fresh remote
   OID, staged/unstaged/untracked inventory, and the checkpoint identity. Retain
   file names safely (NUL-delimited inventories handle spaces and newlines).
   Do not publish private WIP contents or credentials in shared incident notes.
   Record ignored files that could obstruct repair; do not include them blindly.
   If the index already has unresolved conflicts or a merge/rebase is active,
   preserve it and stop normal stash recovery: ordinary stash may fail. Resolve
   that state deliberately without resetting, cleaning, or discarding work.
2. Fetch the explicit configured remote/ref and inspect the ancestry and all
   unpublished commits, including commits made after the failed CI SHA. Stash
   preserves working changes; it neither removes nor isolates those commits.
   Never push unrelated unpublished commits as part of a CI repair. Retain their
   exact OIDs and durable reachability before any authorized history movement.
   If recovery cannot proceed from current HEAD without publishing them, retain
   the hold and resolve the unpublished-history disposition explicitly before
   resetting or rewriting history. Remote divergence requires reconciliation,
   fresh verification and affected review; never infer force-push permission.
3. For dirty staged, unstaged, or untracked work, create an identifiable stash
   with `git stash push --include-untracked -m <incident-and-original-HEAD>`.
   Record the prior stash OID first, then verify command success, a new stash
   OID, its original HEAD parent, and the expected clean index/worktree. Record
   that exact stash OID and an apply/drop status before repair. A label alone
   or an unchanged `stash@{0}` is not proof of preservation. Keep ignored data
   intact; if it obstructs repair, arrange explicit preservation. If the checkout
   was clean, record that fact instead of claiming a stash was created.
4. Diagnose and reproduce the failure, then commit only the tested causal repair
   on the configured trunk with signing, normal pre-commit gates, exact identity
   checks and proportional independent review. Push only to origin's explicit
   trunk ref after fresh remote readback and fast-forward checks. Track the exact
   repair SHA and terminal-success replacement run. An unchanged-SHA rerun
   creates no repair commit. Keep the global hold through pending/running CI.
5. After recovery reaches terminal success, restore the exact retained stash
   with `git stash apply --index <recorded-OID>` so the staged/unstaged split is
   preserved. Use apply, not pop: retain recovery evidence until restoration is
   verified. If apply fails or conflicts, retain the stash and original recovery
   record; inspect the current index, conflict markers and untracked collisions.
   Do not blindly reapply, reset hard, clean files, or drop the stash. Resolve
   deliberately against the repair and original WIP, retaining both sides and
   restoring the intended index state. Renew focused tests and affected review
   for source-changing resolutions; conflicts grant no review or verification
   credit. Report a bounded unresolved blocker if intent cannot be determined.
6. Verify restoration of staged, unstaged and untracked content against the saved
   inventory and stash, accounting for intentional repair/conflict changes.
   Record final HEAD, stash OID, restoration result and next checkpoint. Drop
   only the matching stash entry after successful verification (resolve its
   current selector and recheck its OID first); never clear all stashes. If an
   interruption occurred, reconcile actual HEAD/index/worktree and stash before
   repeating any operation. A retained record prevents duplicate apply or drop.
   Continue the paused WIP through normal immediate-test and commit checkpoints.

Shared incident notes retain sanitized recovery status and exact commit/run IDs;
local recovery metadata retains private file inventories and stash identity. WIP
restoration is neither final feature acceptance nor a claim of delivered value.
