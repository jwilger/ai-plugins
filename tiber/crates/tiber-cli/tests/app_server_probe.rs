#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, process::Command};

    #[test]
    #[expect(
        clippy::implicit_return,
        reason = "the Result adapters are clearest as expression closures in this black-box assertion"
    )]
    fn authority_probe_rejects_codex_0_147_with_actionable_evidence() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tiber-app-server/tests/fixtures/codex-0.147.0-authority-surface.json");
        let output = Command::new(env!("CARGO_BIN_EXE_tiber"))
            .arg("app-server-probe")
            .arg(fixture)
            .output();

        assert_eq!(
            output
                .map(|result| {
                    (
                        result.status.success(),
                        String::from_utf8_lossy(&result.stderr).into_owned(),
                    )
                })
                .map_err(|error| error.to_string()),
            Ok((
                false,
                "app_server_tool_isolation_unverified: the verified app-server schema has operation item types but no reviewed isolation proof: thread-item:commandExecution:no-isolation-proof, thread-item:fileChange:no-isolation-proof\n".to_owned()
            ))
        );
    }
}
