# Completion evidence

- Normal commit must run its actual Lefthook gate. Failed hooks preserve staged work and require causal repair; no bypass or fabricated receipt.
- Manifest/format authority: `nix develop -c just validate-marketplace` performs JSON checks, portable schema/version synchronization, model-routing checks, and Prettier.
- Full gate: `nix develop -c just ci`; GitHub CI runs it plus manifest/eval-wiring checks and exposes the required `CI gate` aggregator.
- Match verification to changed behavior; verify actual command exit/results, exact committed source/message/signature, pushed SHA readback and terminal CI for that SHA.
- Model-facing instruction/schema/context changes require proportionate provider-backed evals; deterministic infrastructure can use meaningful deterministic tests plus wiring dry run. Dry run is not behavior evidence.
- Owner authorization covers repository-owned Codex subscription evals. Isolate generated authentication, preserve source logins, run secret-leak checks, and never transmit unrelated private material.
- Required review policy, configured delivery mode, authorized cleanup, and any required runtime uptake remain part of completion. Open PR alone is insufficient; do not merge without authorization.
