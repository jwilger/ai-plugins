# Proportional threat modeling

Derive each review's blocking threat model from the actual intended deployment
and usage of the system, product, or component being changed. A security or
safety finding becomes blocking only when it identifies a concrete trust
boundary, a plausible in-model failure, and proportionate impact. Reject an
out-of-model premise or report it as a non-blocking observation; do not let it
expand implementation scope.

Local, single-owner development tools default to trusting the owner, repository,
development environment, installed toolchain, `PATH`, environment variables,
and local configuration. Ordinary mistakes, stale state, cooperative
concurrency, interruption, crashes, filesystem failures, and remote data loss
remain in scope. Malicious local processes, intentional self-bypass,
compromised local tools, adversarial open-file races, and crafted internal
metadata do not become blockers unless the project declares a stronger boundary.

Services, shared infrastructure, credential-handling systems, untrusted-input
processors, and explicitly security-sensitive projects commonly need stronger
boundaries. State those boundaries and protected assets explicitly. In every
context, prefer deleting unnecessary mechanisms and reducing surface area over
hardening mechanisms the intended use does not need.
