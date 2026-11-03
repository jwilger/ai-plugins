# Development System

Configured repository development, task tracking, and independent review for Codex.

## Start a project or adopt an existing one

Install the plugin, restart Codex, and open the repository's primary checkout.
Ask: “Use development-system setup to inspect this project and propose its
configuration.” Review the complete `.development-system.toml` and hook proposal
before approving it. Confirm Development System and the Tiber companion MCP servers are available; do not
register duplicate servers in your project.

### Greenfield projects

Describe the product, intended users, constraints, and first acceptance criteria.
For example: “Help me plan a small command-line tool that imports CSV expenses.
Choose tooling with me, configure development-system for the initial and planned
layout, and propose a first increment with an executable acceptance check.”
Approve tooling and setup choices before implementation. Build one useful slice
first, then expand with evidence from tests and actual use.

### Brownfield projects

Start with discovery: “Inspect this repository's architecture, instructions,
tests, hooks, and CI. Establish the current verification baseline and propose
how development-system should fit our conventions. Preserve our existing hooks
unless I approve replacing them.” Identify pre-existing failures separately from
regressions. Begin with a bounded bug fix or feature rather than a migration of
the whole repository.

## Get the most benefit

Skills are selected from the task description. You can also explicitly ask
Codex to use the named development-system skill when you want a particular
workflow. Installation alone does not configure a repository.

| Goal          | Example request                                                                                                                     |
| ------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| Shape an idea | “Use the advisor skill to challenge this idea and identify the smallest useful increment.”                                          |
| Implement     | “Add CSV validation. Reject missing amounts with a useful error. Preserve the public API; verify locally and leave delivery to me.” |
| Debug         | “Reproduce this failure, distinguish existing failures, and fix the cause with a regression check.”                                 |
| Prioritize    | “Review the Tiber backlog for overlap and prioritize by user impact and dependencies.”                                              |
| Review        | “Review this change against its acceptance criteria, including security and safety risks.”                                          |
| Deliver       | “Commit and deliver this verified change using the configured delivery mode, then monitor its required CI.”                         |
| Resume        | “Resume the interrupted task from its recorded checkpoint and explain the next required action.”                                    |

State acceptance criteria, compatibility constraints, and whether commits,
pushes, or PR publication are authorized. Keep one task focused; correct scope
early. Ask for gate progress and cancellation options during longer work.
Expect focused verification, independent review, signed delivery where configured,
and CI follow-through. Installed commit/push hooks run their gates; avoid asking
for duplicate manual runs. These workflows provide guidance, not a sandbox or
host permission boundary.

Development System’s Tiber integration, agentic-system engineering, and eval-case reporting require their
respective `features.tiber`, `features.agentic_systems`, and
`features.eval_case_reporting` settings in `.development-system.toml`. Ask setup
to propose enabling the features you need. For LLM work, try: “Define the boundary
contract and representative eval cases before optimizing this RAG pipeline.”
For surprising assistant behavior, request a minimal sanitized eval case. Review
and explicitly approve the exact sanitized issue before it is posted.

## Setup and recovery reference

See [troubleshooting](TROUBLESHOOTING.md) for binary repair, signing, model routing, checkpoint recovery.

See the [marketplace licensing explanation](../../README.md#license).
