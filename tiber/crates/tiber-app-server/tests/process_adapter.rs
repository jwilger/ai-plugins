#![forbid(unsafe_code)]

#[cfg(test)]
#[expect(
    clippy::absolute_paths,
    clippy::expect_used,
    clippy::implicit_return,
    clippy::indexing_slicing,
    clippy::map_unwrap_or,
    clippy::shadow_reuse,
    clippy::std_instead_of_core,
    reason = "black-box fixture setup and assertions use fail-fast test ergonomics without entering shipping library code"
)]
mod tests {

    use std::{
        path::PathBuf,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use tiber_app_server::{AccountStatus, AppServerClient, AppServerConfig};

    const ISOLATED_CONFIG: &str = include_str!("../../../config/app-server.toml");

    fn fixture_config(mode: Option<&str>) -> AppServerConfig {
        let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .expect("test repository should canonicalize");
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test clock should follow the Unix epoch")
            .as_nanos();
        let codex_home = std::env::temp_dir().join(format!("tiber-app-server-test-{nonce}"));
        let node = std::env::var_os("TIBER_TEST_NODE")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/usr/bin/env"));
        let mut arguments = if node.ends_with("env") {
            vec!["node".to_owned()]
        } else {
            Vec::new()
        };
        arguments.push(
            repository
                .join("tiber/scripts/tests/fake-app-server.mjs")
                .to_string_lossy()
                .into_owned(),
        );
        if let Some(mode) = mode {
            arguments.push(format!("--mode={mode}"));
        }
        AppServerConfig::new(
            node,
            arguments,
            codex_home,
            repository,
            Duration::from_secs(2),
        )
        .expect("fixture configuration should satisfy semantic invariants")
    }

    #[test]
    fn isolated_adapter_delegates_auth_streams_text_and_keeps_tools_inert() {
        let mut client = AppServerClient::start(fixture_config(None), ISOLATED_CONFIG)
            .expect("fixture app-server should initialize");

        assert_eq!(client.account_status(), Ok(AccountStatus::SignedOut));
        let handoff = client
            .start_chatgpt_login()
            .expect("fixture should provide a browser login handoff");
        assert_eq!(handoff.login_id, "login-fixture");
        assert_eq!(handoff.auth_url, "https://example.invalid/login");
        client
            .await_chatgpt_login(&handoff.login_id)
            .expect("fixture browser login should complete");
        client
            .login_with_api_key("fixture-secret")
            .expect("app-server should accept API-key mode without Tiber retaining it");
        assert_eq!(client.account_status(), Ok(AccountStatus::ApiKey));

        let result = client
            .converse("request the declared Tiber tool")
            .expect("fixture conversation should complete");
        assert_eq!(result.text, "hello from Tiber");
        assert_eq!(result.inert_tool_requests.len(), 1);
        assert_eq!(result.inert_tool_requests[0].tool, "tiber_authority_probe");

        client.logout().expect("fixture logout should succeed");
        assert_eq!(client.account_status(), Ok(AccountStatus::SignedOut));
    }

    #[test]
    fn silent_child_returns_a_retryable_typed_timeout() {
        let error = AppServerClient::start(fixture_config(Some("silent")), ISOLATED_CONFIG)
            .err()
            .expect("silent fixture should time out");
        assert_eq!(error.code(), "app_server_timeout");
        assert!(error.is_retryable());
    }

    #[test]
    fn dropping_the_adapter_reaps_the_app_server_child() {
        let client = AppServerClient::start(fixture_config(None), ISOLATED_CONFIG)
            .expect("fixture app-server should initialize");
        let process_id = client.child_process_id();
        assert!(PathBuf::from(format!("/proc/{process_id}")).exists());

        drop(client);

        assert!(!PathBuf::from(format!("/proc/{process_id}")).exists());
    }
    #[test]
    fn chatty_child_cannot_extend_the_operation_deadline() {
        let error = AppServerClient::start(fixture_config(Some("chatty")), ISOLATED_CONFIG)
            .err()
            .expect("chatty fixture should hit the whole-operation deadline");
        assert_eq!(error.code(), "app_server_timeout");
    }

    #[test]
    fn incompatible_runtime_version_fails_closed() {
        let error = AppServerClient::start(fixture_config(Some("wrong-version")), ISOLATED_CONFIG)
            .err()
            .expect("unreviewed Codex version should be rejected");
        assert_eq!(error.code(), "app_server_version_incompatible");
    }

    #[test]
    fn credential_rejection_diagnostic_never_echoes_the_key() {
        let mut client = AppServerClient::start(
            fixture_config(Some("credential-rejection")),
            ISOLATED_CONFIG,
        )
        .expect("fixture app-server should initialize");
        let secret = "fixture-secret-that-must-not-leak";
        let error = client
            .login_with_api_key(secret)
            .expect_err("fixture should reject the credential");
        assert_eq!(error.code(), "app_server_request_rejected");
        assert!(!error.to_string().contains(secret));
    }

    #[test]
    fn idless_login_failure_is_reported_without_waiting_for_timeout() {
        let mut client = AppServerClient::start(
            fixture_config(Some("idless-login-failure")),
            ISOLATED_CONFIG,
        )
        .expect("fixture app-server should initialize");
        let handoff = client
            .start_chatgpt_login()
            .expect("fixture should start browser login");
        let error = client
            .await_chatgpt_login(&handoff.login_id)
            .expect_err("idless failure should terminate login");
        assert_eq!(error.code(), "app_server_authentication_failed");
    }

    #[test]
    fn colliding_server_request_is_rejected_before_the_client_response() {
        let mut client =
            AppServerClient::start(fixture_config(Some("id-collision")), ISOLATED_CONFIG)
                .expect("fixture app-server should initialize");
        let result = client
            .converse("exercise the colliding server request")
            .expect("adapter should reject the server request and continue");
        assert_eq!(result.text, "hello from Tiber");
    }

    #[test]
    fn oversized_prompt_is_rejected_before_transport_write() {
        let mut client = AppServerClient::start(fixture_config(None), ISOLATED_CONFIG)
            .expect("fixture app-server should initialize");
        let prompt = "x".repeat(20_000);
        let error = client
            .converse(&prompt)
            .expect_err("oversized prompt must not reach the child pipe");
        assert_eq!(error.code(), "app_server_prompt_too_large");
    }
}
