# App-server authority compatibility spike

## Question

Can Tiber use `codex app-server` for inference while exposing only
Tiber-declared tools and ensuring every model-requested operation remains inert
until Tiber authorizes it?

## Environment

- Platform: x86_64 Linux
- Codex CLI: 0.147.0
- Protocol source: `codex app-server generate-json-schema --experimental`
- Official reference: <https://learn.chatgpt.com/docs/app-server>

## Reproduction

```shell
schema_dir="$(mktemp -d)"
codex app-server generate-json-schema --experimental --out "$schema_dir"
bash tiber/scripts/check-app-server-authority-fixture.sh \
  "$schema_dir/codex_app_server_protocol.v2.schemas.json"
cargo run --manifest-path tiber/Cargo.toml -p tiber -- \
  app-server-probe \
  tiber/crates/tiber-app-server/tests/fixtures/codex-0.147.0-authority-surface.json
```

Expected result for Codex 0.147.0:

```text
app_server_tool_isolation_unverified: the verified app-server schema has operation item types but no reviewed isolation proof: thread-item:commandExecution:no-isolation-proof, thread-item:fileChange:no-isolation-proof
```

The command exits nonzero. The pinned-schema verifier must pass before the
deterministic projection is used as decision evidence; ordinary CI checks the
projection behavior but intentionally cannot regenerate a Codex-owned schema.

## Evidence

The generated protocol contains `commandExecution` and `fileChange` thread
items. `ThreadStartParams` includes additive `dynamicTools`, sandbox, and
approval settings but supplies no documented proof that built-in model tools
can be disabled or allowlisted. The probe does not infer tool ownership from
arbitrary schema strings: it accepts only the provenance-bound projection of
the exact reviewed V2 `ThreadStartParams` fields and `ThreadItem`
discriminators, then fails closed on every structural or version change.
The official documentation describes dynamic tools as experimental
client-executed additions and separately documents built-in tool behavior.

The deterministic fixture is a projection of the generated 0.147.0 schema,
records the SHA-256 of its 609,050-byte source, and can be regenerated with the
checked-in jq program:

```shell
schema="$schema_dir/codex_app_server_protocol.v2.schemas.json"
schema_sha256="$(sha256sum "$schema" | cut -d' ' -f1)"
jq --arg codex_version 0.147.0 --arg schema_sha256 "$schema_sha256" \
  -f tiber/scripts/extract-app-server-authority-surface.jq "$schema" \
  | prettier --parser json

bash tiber/scripts/check-app-server-authority-fixture.sh "$schema"
```

Client refusal to answer an approval request is insufficient: sandboxed
operations can execute without an escalation request. Neither setting proves
that the model cannot receive, or app-server cannot own, operations not declared
by Tiber. A read-only sandbox reduces impact but does not establish Tiber's
required authority boundary.

## Decision

The spike fails the locked authority contract. Do not implement conversation,
authentication, TUI, or later roadmap phases on this transport. ADR-0005 is
marked rejected by the spike. Resume only after app-server supplies a
documented built-in-tool policy that the probe can verify, or after the owner
approves a replacement inference ADR.

## Review-orchestration invariant

This stop does not remove or narrow review orchestration. The native contract
remains recorded in `tiber/ARCHITECTURE.md`, and the installed advisory plugin
continues to provide the existing multi-agent final-review behavior while the
harness transport decision is unresolved.
