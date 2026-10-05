//! One explicitly approved host invocation, one advisory operation, one sandbox.
//!
//! This is not an MCP service and never grants its own host approval. The caller
//! runs the public helper through the harness's supported approval mechanism.
use super::*;
use std::io::Read;
use std::process::Stdio;

const PUBLIC_FLAG: &str = "--replay-review-operation";
const CHILD_FLAG: &str = "--replay-review-operation-child";

pub(super) fn dispatch(arguments: &[String]) -> Option<i32> {
    let flag = arguments.first()?.as_str();
    if !matches!(flag, PUBLIC_FLAG | CHILD_FLAG) {
        return None;
    }
    Some(match run(arguments, flag == CHILD_FLAG) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("development_system.review_replay_failed {error}");
            1
        }
    })
}

fn allowed(tool: &str) -> bool {
    matches!(
        tool,
        "final_review.assess_risk"
            | "final_review.plan"
            | "final_review.advance"
            | "final_review.confirm_split"
            | "final_review.reopen"
            | "final_review.continue_review"
            | "final_review.resume_latest"
            | "final_review.pending_assignments"
            | "final_review.clean_status"
            | "final_review.out_of_scope_report"
            | "final_review.filter_findings"
            | "final_review.yield_report"
            | "final_review.evidence"
    )
}

fn run(cli: &[String], child: bool) -> Result<i32, String> {
    if cli.len() != 3 {
        return Err("phase=input replay_usage=REPOSITORY_ROOT_TOOL_WITH_ARGUMENTS_JSON_ON_STDIN retryable=false".into());
    }
    let tool = &cli[2];
    if !allowed(tool) {
        return Err("phase=input replay_operation_not_allowed=true retryable=false".into());
    }
    let root = fs::canonicalize(&cli[1])
        .map_err(|error| failure("repository", Path::new(&cli[1]), error))?;
    let mut bytes = Vec::new();
    io::stdin()
        .take(MAX_REQUEST_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("phase=input source={error} retryable=false"))?;
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err("phase=input request_too_large=true retryable=false".into());
    }
    let arguments: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("phase=input source={error} retryable=false"))?;
    if !arguments.is_object() {
        return Err("phase=input arguments_object_required=true retryable=false".into());
    }
    // Never allow a payload or state reference to route outside the reviewed root.
    for pointer in [
        "/project_root",
        "/state_ref/project_root",
        "/state/scope/project_root",
    ] {
        if let Some(value) = arguments.pointer(pointer) {
            let path = value
                .as_str()
                .ok_or("phase=input replay_project_root_invalid=true retryable=false")?;
            if fs::canonicalize(path)
                .map_err(|error| failure("repository", Path::new(path), error))?
                != root
            {
                return Err("phase=input replay_project_root_mismatch=true retryable=false".into());
            }
        }
    }
    if child {
        return execute(&root, tool, arguments);
    }
    let actual_root = git_path(&root, "--show-toplevel")?;
    if actual_root != root {
        return Err("phase=input replay_repository_root_required=true retryable=false".into());
    }
    let common = git_path(&root, "--git-common-dir")?;
    let root_text = root
        .to_str()
        .ok_or("phase=input replay_utf8_root_required=true retryable=false")?;
    let work_item = [
        "/work_item_id",
        "/state_ref/work_item_id",
        "/state/work_item_id",
    ]
    .into_iter()
    .filter_map(|pointer| arguments.pointer(pointer).and_then(Value::as_str))
    .collect::<Vec<_>>();
    if work_item.windows(2).any(|pair| pair[0] != pair[1]) {
        return Err("phase=input replay_work_item_mismatch=true retryable=false".into());
    }
    let database = durable_report_database_path_for(
        root_text,
        work_item.first().copied(),
        ReviewPersistence::PluginAdvisoryLocal,
    )?;
    let projection = database
        .parent()
        .ok_or("phase=projection replay_parent_missing=true")?;
    // The layout is per-key specifically because SQLite must create/unlink WALs.
    if projection.file_name().and_then(|name| name.to_str()) == Some("final-review-reports") {
        return Err(
            "phase=projection replay_scoped_projection_required=true retryable=false".into(),
        );
    }
    let state_root = durable_report_state_root(
        env::var_os("XDG_STATE_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from),
        env::var_os("HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from),
    )?;
    let snapshot = state_root
        .join("development-discipline/final-review-snapshot-objects")
        .join(stable_storage_digest(&[
            "development-discipline-final-review-snapshot-v1",
            root_text,
        ]));
    // Validate every state mount before preparing any directory. Resolve existing
    // ancestors so aliases such as an external path containing `..` cannot hide
    // source overlap; missing final directories need not be created to do this.
    for path in [projection, snapshot.as_path()] {
        let resolved = prospective_directory(path)?;
        if resolved.starts_with(&root) || root.starts_with(&resolved) {
            return Err("phase=scope replay_state_inside_source_forbidden=true retryable=configure_external_XDG_STATE_HOME".into());
        }
    }
    let writable = [
        common.join("objects"),
        common.join("refs/tiber"),
        common.join("logs/refs/tiber"),
        common.join("plugin-advisory-final-review"),
        projection.to_path_buf(),
        snapshot,
    ];
    for path in &writable {
        ensure_directory(path, "scope")?;
    }
    let scratch = Scratch::new()?;
    let executable = env::current_exe().map_err(|error| format!("phase=binary source={error}"))?;
    let mut command = ProcessCommand::new("bwrap");
    scrub_git_environment(&mut command);
    command.args([
        "--die-with-parent",
        "--new-session",
        "--unshare-net",
        "--unshare-pid",
        "--unshare-ipc",
        "--unshare-uts",
        "--ro-bind",
        "/",
        "/",
        "--proc",
        "/proc",
        "--dev",
        "/dev",
    ]);
    for path in writable.iter().chain(std::iter::once(&scratch.0)) {
        command.arg("--bind").arg(path).arg(path);
    }
    command
        .arg("--chdir")
        .arg(&root)
        .arg("--")
        .arg(executable)
        .arg(CHILD_FLAG)
        .arg(&root)
        .arg(tool)
        .env("TMPDIR", &scratch.0)
        .env("TIBER_EVENT_STORE_DIAGNOSTICS", "1")
        .stdin(Stdio::piped());
    let mut process = command.spawn().map_err(|error| {
        format!(
            "phase=sandbox source={error} retryable=after_supported_host_approval_or_bwrap_install"
        )
    })?;
    process
        .stdin
        .take()
        .ok_or("phase=sandbox stdin_missing=true")?
        .write_all(&bytes)
        .map_err(|error| format!("phase=sandbox source={error} retryable=inspect_child_failure"))?;
    let status = process.wait().map_err(|error| {
        format!("phase=sandbox source={error} retryable=read_authoritative_status_first")
    })?;
    if !status.success() {
        eprintln!("development_system.review_replay_stopped phase=sandbox_or_operation status={status} retryable=read_authoritative_status_first approval_denial_is_final=true");
    }
    Ok(status.code().unwrap_or(1))
}

fn execute(root: &Path, tool: &str, arguments: Value) -> Result<i32, String> {
    // Resolve an interrupted publication using the adapter's existing lock and
    // candidate ancestry logic before submitting exactly one coordinator intent.
    let store = tiber_git::git_event_store::GitEventStore::open_for_authority(
        root,
        tiber_git::git_event_store::GitEventStoreAuthority::PluginAdvisoryFinalReview,
    )
    .map_err(|error| {
        format!(
            "phase=authority_open path={} source={error} retryable=after_path_permission_repair",
            root.display()
        )
    })?;
    let reconciled = review_runtime()?.block_on(store.synchronize())
        .map_err(|error| format!("phase=publication_reconciliation source={error} retryable=read_authoritative_status_first"))?;
    if reconciled == tiber_git::git_event_store::SynchronizeOutcome::PublishedPending {
        // A retained candidate is now confirmed. Do not submit the caller's
        // intent again: its previous response may have been lost after commit.
        let recovery = json!({"publication_reconciled":true,"operation_replayed":false,
            "required_action":"final_review.resume_latest","project_root":root});
        write_json_rpc_response(
            &mut io::stdout().lock(),
            json!({"jsonrpc":"2.0","id":1,
            "result":{"content":[{"type":"text","text":recovery.to_string()}]}}),
        )
        .map_err(|error| {
            format!("phase=response source={error} retryable=read_authoritative_status_first")
        })?;
        return Ok(0);
    }
    let mut coordinator = ReviewCoordinator::with_service_surface(ServiceSurface::PluginAdvisory);
    let request = json!({"jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{"name":tool,"arguments":arguments}});
    let response = with_mcp_repository_root(Some(root), || coordinator.handle_json_rpc(&request))?;
    let failed = response.get("error").is_some()
        || response.pointer("/result/isError") == Some(&json!(true));
    write_json_rpc_response(&mut io::stdout().lock(), response).map_err(|error| {
        format!("phase=response source={error} retryable=read_authoritative_status_first")
    })?;
    Ok(i32::from(failed))
}

fn scrub_git_environment(command: &mut ProcessCommand) {
    // Repository and user Git configuration (including signing) remain intact;
    // injected environment cannot select another repository, index, or object DB.
    for (name, _) in env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }
}

fn git_path(root: &Path, option: &str) -> Result<PathBuf, String> {
    let mut command = ProcessCommand::new("git");
    scrub_git_environment(&mut command);
    let output = command
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--path-format=absolute", option])
        .output()
        .map_err(|error| failure("repository", root, error))?;
    if !output.status.success() {
        return Err(format!(
            "phase=repository path={} source={} retryable=false",
            root.display(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let text = String::from_utf8(output.stdout)
        .map_err(|error| format!("phase=repository source={error}"))?;
    let path = Path::new(text.strip_suffix('\n').unwrap_or(&text));
    fs::canonicalize(path).map_err(|error| failure("repository", path, error))
}

fn failure(phase: &str, path: &Path, error: impl std::fmt::Display) -> String {
    format!(
        "phase={phase} path={} source={error} retryable=after_path_permission_repair",
        path.display()
    )
}

fn prospective_directory(path: &Path) -> Result<PathBuf, String> {
    let mut ancestor = path;
    let mut missing = Vec::new();
    loop {
        match fs::canonicalize(ancestor) {
            Ok(mut resolved) => {
                for component in missing.into_iter().rev() {
                    resolved.push(component);
                }
                return Ok(resolved);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                missing.push(ancestor.file_name().ok_or_else(|| {
                    format!(
                        "phase=scope path={} replay_unresolvable_directory=true retryable=false",
                        path.display()
                    )
                })?);
                ancestor = ancestor.parent().ok_or_else(|| {
                    format!(
                        "phase=scope path={} replay_parent_missing=true retryable=false",
                        path.display()
                    )
                })?;
            }
            Err(error) => return Err(failure("scope", ancestor, error)),
        }
    }
}

fn ensure_directory(path: &Path, phase: &str) -> Result<(), String> {
    // Reject symlinks at every existing component before creating anything.
    // The supported threat model is cooperative local processes, not a hostile
    // same-uid mount race against the approved host invocation.
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => {},
            Ok(_) => return Err(format!("phase={phase} path={} replay_directory_symlink_or_file_forbidden=true retryable=false", current.display())),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                fs::create_dir(&current).map_err(|error| failure(phase, &current, error))?;
            },
            Err(error) => return Err(failure(phase, &current, error)),
        }
    }
    Ok(())
}

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Result<Self, String> {
        use std::os::unix::fs::DirBuilderExt;
        for nonce in 0..100 {
            let path = PathBuf::from(format!(
                "/tmp/development-review-replay-{}-{nonce}",
                std::process::id()
            ));
            match fs::DirBuilder::new().mode(0o700).create(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(failure("scratch", &path, error)),
            }
        }
        Err("phase=scratch replay_unique_directory_unavailable=true retryable=true".into())
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
