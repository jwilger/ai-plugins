# Observability And Security

Production agentic systems need traces that explain behavior and controls that
limit damage when the model is wrong or manipulated.

## Observability

- Trace each meaningful step: model call, retrieval, rerank, tool call,
  guardrail, handoff, and sub-agent.
- Attach model id, input/output token counts, latency, cost, tool name,
  decision metadata, and failure reason where available.
- Redact or control access to trace stores because prompts and tool I/O can
  contain sensitive data.
- Use traces for root-cause analysis instead of guessing from the final answer.

## Security

- Treat user input, retrieved content, web content, tool output, and third-party
  tool descriptions as untrusted.
- Indirect prompt injection happens when untrusted data is allowed to behave
  like instructions.
- Prefer least-privilege tool scopes, allowlisted actions, egress controls,
  sandboxed execution, and human approval for high-impact actions.
- Test confused-deputy paths, data exfiltration, memory poisoning,
  over-permissive tools, and unsafe autonomous actions.
