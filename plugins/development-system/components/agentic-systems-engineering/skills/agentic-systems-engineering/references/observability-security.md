# Observability And Security

Production agentic systems need traces that explain behavior and controls that
limit damage when the model is wrong or manipulated.

## Observability

- Emit trace spans for every model, retrieval, rerank, tool, policy, handoff, and
  sub-agent call.
- Attach model and prompt/template versions, input/output token counts, latency,
  cost, tool name, routing or policy decision, and typed failure reason where
  available.
- Redact or control access to trace stores because prompts and tool I/O can
  contain sensitive data.
- Use traces for root-cause analysis instead of guessing from the final answer.

## Security

- Treat user input, retrieved content, web content, tool output, and third-party
  tool descriptions as untrusted.
- Indirect prompt injection happens when untrusted data is allowed to behave
  like instructions.
- Prefer least-privilege tool scopes, allowlisted actions, egress controls,
  sandboxed execution, and human approval for actions the deployment threat
  model classifies as high impact, such as irreversible or externally visible
  writes, credential use, or privileged data access.
- Test confused-deputy paths, data exfiltration, memory poisoning,
  over-permissive tools, and unsafe autonomous actions.
