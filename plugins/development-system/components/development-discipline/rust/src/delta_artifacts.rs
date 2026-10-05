//! Durable, project-scoped cache of immutable patches derived from Git snapshots.
use super::*;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};

pub(super) fn directory(project_root: &Path) -> Result<PathBuf, String> {
    let root = project_root
        .canonicalize()
        .map_err(|error| format!("delta_artifact_root_failed source={error}"))?;
    #[cfg(test)]
    let state = test_temp_root().join("state");
    #[cfg(not(test))]
    let state = durable_report_state_root(
        env::var_os("XDG_STATE_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from),
        env::var_os("HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from),
    )?;
    let raw_root = root
        .as_os_str()
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(state
        .join("development-discipline/final-review-delta-evidence")
        .join(stable_storage_digest(&[
            "development-discipline-final-review-delta-v1",
            &raw_root,
        ])))
}

fn prepare(project_root: &Path) -> Result<PathBuf, String> {
    let path = directory(project_root)?;
    let resolved = review_replay::prospective_directory(&path)?;
    let root = project_root
        .canonicalize()
        .map_err(|error| format!("delta_artifact_root_failed source={error}"))?;
    if resolved.starts_with(&root) || root.starts_with(&resolved) {
        return Err("delta_artifact_state_inside_source_forbidden=true".into());
    }
    review_replay::ensure_directory(&path, "delta-artifact")?;
    Ok(path)
}

fn regular_file(path: &Path) -> Result<fs::File, String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        format!(
            "delta_artifact_stat_failed path={} source={error}",
            path.display()
        )
    })?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.nlink() != 1 {
        return Err(format!(
            "delta_artifact_regular_unaliased_file_required path={}",
            path.display()
        ));
    }
    // O_NOFOLLOW | O_NONBLOCK on the required Linux target: even a swapped
    // FIFO cannot block before the descriptor metadata check. Aliases cannot redirect a
    // published artifact or lock to unrelated state. No hostile same-uid races.
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(0x20800)
        .open(path)
        .map_err(|error| {
            format!(
                "delta_artifact_read_failed path={} source={error}",
                path.display()
            )
        })?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("delta_artifact_stat_failed source={error}"))?;
    if !metadata.is_file() || metadata.nlink() != 1 {
        return Err(format!(
            "delta_artifact_regular_unaliased_file_required path={}",
            path.display()
        ));
    }
    Ok(file)
}

pub(super) fn digest(project_root: &Path, path: &Path) -> Result<String, String> {
    let file = regular_file(path)?;
    let output = git_command(project_root)
        .args(["hash-object", "--stdin"])
        .stdin(Stdio::from(file))
        .output()
        .map_err(|error| format!("delta_artifact_digest_failed source={error}"))?;
    let digest = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !output.status.success() || !valid_git_object_id(&digest) {
        return Err("delta_artifact_digest_failed=true".into());
    }
    Ok(digest)
}

pub(super) struct Candidate {
    pub path: PathBuf,
    _publication_lock: fs::File,
}
impl Candidate {
    pub fn create(project_root: &Path) -> Result<(Self, fs::File), String> {
        let parent = prepare(project_root)?;
        let lock_path = parent.join(".publication.lock");
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(0x20800)
            .open(&lock_path)
            .map_err(|error| format!("delta_artifact_lock_failed source={error}"))?;
        let meta = lock
            .metadata()
            .map_err(|error| format!("delta_artifact_lock_failed source={error}"))?;
        if !meta.is_file() || meta.nlink() != 1 {
            return Err("delta_artifact_lock_alias_forbidden=true".into());
        }
        fs2::FileExt::lock_exclusive(&lock)
            .map_err(|error| format!("delta_artifact_lock_failed source={error}"))?;
        let path = parent.join(".candidate.patch");
        // The lock spans creation, Git generation, publication and Drop. Thus
        // this exact staging path can only belong to an interrupted predecessor.
        match fs::symlink_metadata(&path) {
            Ok(_) => {
                regular_file(&path)?;
                fs::remove_file(&path).map_err(|error| {
                    format!("delta_artifact_staging_cleanup_failed source={error}")
                })?;
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "delta_artifact_staging_check_failed source={error}"
                ))
            }
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .map_err(|error| format!("delta_artifact_create_failed source={error}"))?;
        Ok((
            Self {
                path,
                _publication_lock: lock,
            },
            file,
        ))
    }
    pub fn publish(
        &self,
        project_root: &Path,
        expected_digest: Option<&str>,
    ) -> Result<(PathBuf, String), String> {
        let digest = digest(project_root, &self.path)?;
        if expected_digest.is_some_and(|expected| expected != digest) {
            return Err("delta_artifact_snapshot_digest_mismatch=true recovery=inspect_recorded_snapshot_evidence".into());
        }
        let parent = self
            .path
            .parent()
            .ok_or("delta_artifact_parent_required=true")?;
        let target = parent.join(format!("{digest}.patch"));
        match fs::symlink_metadata(&target) {
            Ok(_) => {
                if self::digest(project_root, &target)? != digest {
                    return Err(format!("delta_artifact_existing_digest_mismatch=true path={} recovery=quarantine_tampered_cache_then_retrieve_pending_assignment",target.display()));
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let candidate = regular_file(&self.path)?;
                candidate
                    .set_permissions(fs::Permissions::from_mode(0o400))
                    .map_err(|error| format!("delta_artifact_permissions_failed source={error}"))?;
                candidate
                    .sync_all()
                    .map_err(|error| format!("delta_artifact_sync_failed source={error}"))?;
                // All supported publishers hold this project lock. Rename only
                // after bytes are flushed; interrupted staging is never visible.
                fs::rename(&self.path, &target)
                    .map_err(|error| format!("delta_artifact_publish_failed source={error}"))?;
                fs::File::open(parent)
                    .and_then(|directory| directory.sync_all())
                    .map_err(|error| {
                        format!("delta_artifact_directory_sync_failed source={error}")
                    })?;
            }
            Err(error) => return Err(format!("delta_artifact_target_failed source={error}")),
        }
        Ok((target, digest))
    }
}
impl Drop for Candidate {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Version-one rendering reads only immutable snapshot objects and attributes.
/// The scratch bare repository excludes source config/info attributes, user
/// config, custom diff drivers and working-tree attributes. Its directory stays
/// allocated for the whole operation, preventing cross-process name reuse.
pub(super) struct SnapshotRenderer {
    directory: PathBuf,
    attribute_source: String,
    repository_prefix: String,
    work_tree: PathBuf,
}
impl SnapshotRenderer {
    fn isolated_command() -> ProcessCommand {
        let mut command = ProcessCommand::new("git");
        for (key, _) in env::vars_os() {
            if key.as_bytes().starts_with(b"GIT_") {
                command.env_remove(key);
            }
        }
        command
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_ATTR_NOSYSTEM", "1")
            .env("LC_ALL", "C")
            .args([
                "-c",
                "core.attributesFile=/dev/null",
                "-c",
                "core.quotePath=true",
            ]);
        command
    }
    pub fn new(project_root: &Path, current: &str) -> Result<Self, String> {
        use std::os::unix::fs::DirBuilderExt;
        // Unlike git_text, remove only Git's record terminator: a real
        // repository-relative prefix may begin with whitespace.
        let prefix = run_git(
            project_root,
            &["rev-parse".into(), "--show-prefix".into()],
            None,
            None,
            "delta_artifact_repository_prefix",
        )?;
        let prefix = prefix
            .strip_suffix(b"\n")
            .ok_or("delta_artifact_repository_prefix_invalid=true")?;
        let repository_prefix = String::from_utf8(prefix.to_vec()).map_err(|error| {
            format!("delta_artifact_repository_prefix_utf8_failed source={error}")
        })?;
        let top = run_git(
            project_root,
            &["rev-parse".into(), "--show-toplevel".into()],
            None,
            None,
            "delta_artifact_work_tree",
        )?;
        let top = top
            .strip_suffix(b"\n")
            .ok_or("delta_artifact_work_tree_invalid=true")?;
        let work_tree = PathBuf::from(
            String::from_utf8(top.to_vec())
                .map_err(|error| format!("delta_artifact_work_tree_utf8_failed source={error}"))?,
        );
        let format = git_text(
            project_root,
            &["rev-parse".into(), "--show-object-format".into()],
            None,
            None,
            "delta_artifact_object_format",
        )?;
        if !matches!(format.as_str(), "sha1" | "sha256") {
            return Err("delta_artifact_object_format_unsupported=true".into());
        }
        for nonce in 0..100 {
            let directory = env::temp_dir().join(format!(
                "development-delta-render-{}-{nonce}",
                std::process::id()
            ));
            match fs::DirBuilder::new().mode(0o700).create(&directory) {
                Ok(()) => {
                    let renderer = Self {
                        directory,
                        attribute_source: current.into(),
                        repository_prefix,
                        work_tree,
                    };
                    let output = Self::isolated_command()
                        .args([
                            "init",
                            "--bare",
                            "--quiet",
                            "--template=",
                            &format!("--object-format={format}"),
                        ])
                        .arg(&renderer.directory)
                        .output()
                        .map_err(|error| {
                            format!("delta_artifact_renderer_init_failed source={error}")
                        })?;
                    if !output.status.success() {
                        return Err(format!(
                            "delta_artifact_renderer_init_failed detail={}",
                            String::from_utf8_lossy(&output.stderr).trim()
                        ));
                    }
                    return Ok(renderer);
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(format!(
                        "delta_artifact_renderer_scratch_failed source={error}"
                    ))
                }
            }
        }
        Err("delta_artifact_renderer_scratch_unavailable=true retryable=true".into())
    }
    pub fn pathspec(&self, path: &str) -> Result<String, String> {
        if normalize_review_path(path, None).as_deref() != Some(path) {
            return Err("delta_artifact_snapshot_path_invalid=true".into());
        }
        // Git applies the project prefix internally via the explicit work-tree
        // context, instead of multiplying it across up to 20,000 argv entries.
        Ok(format!(":(literal){path}"))
    }
    pub fn project_path(&self, path: &str) -> Result<String, String> {
        let path = path
            .strip_prefix(&self.repository_prefix)
            .ok_or("delta_artifact_inventory_outside_project=true")?;
        if normalize_review_path(path, None).as_deref() != Some(path) {
            return Err("delta_artifact_inventory_path_invalid=true".into());
        }
        Ok(path.into())
    }
    pub fn command(&self, project_root: &Path) -> Result<ProcessCommand, String> {
        let mut command = Self::isolated_command();
        command
            .arg("--git-dir")
            .arg(&self.directory)
            .arg("--work-tree")
            .arg(&self.work_tree)
            .arg("-C")
            .arg(project_root)
            .env("GIT_ATTR_SOURCE", &self.attribute_source);
        configure_scope_snapshot_objects(&mut command, project_root)?;
        Ok(command)
    }
}
impl Drop for SnapshotRenderer {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

pub(super) fn recover_assignment(
    project_root: &Path,
    assignment: &mut Value,
) -> Result<(), String> {
    let Some(raw) = assignment.get("delta_evidence") else {
        return Ok(());
    };
    let evidence: ReviewDeltaEvidenceFacts = serde_json::from_value(raw.clone())
        .map_err(|error| format!("delta_artifact_evidence_invalid source={error}"))?;
    let Some(reference) = evidence.artifact_reference.as_ref() else {
        return Ok(());
    };
    let expected = evidence
        .artifact_digest
        .as_deref()
        .filter(|digest| valid_git_object_id(digest))
        .ok_or("delta_artifact_digest_required=true")?;
    let canonical = directory(project_root)?.join(format!("{expected}.patch"));
    let parts = reference.split('/').collect::<Vec<_>>();
    let replay_legacy = parts.len() == 5
        && parts[0].is_empty()
        && parts[1] == "tmp"
        && parts[2]
            .strip_prefix("development-review-replay-")
            .is_some_and(|suffix| {
                suffix.split_once('-').is_some_and(|(pid, nonce)| {
                    !pid.is_empty()
                        && !nonce.is_empty()
                        && pid.bytes().all(|byte| byte.is_ascii_digit())
                        && nonce.bytes().all(|byte| byte.is_ascii_digit())
                })
            })
        && parts[3] == "development-discipline-delta-evidence"
        && parts[4] == format!("{expected}.patch");
    let ordinary_legacy =
        reference == &format!("/tmp/development-discipline-delta-evidence/{expected}.patch");
    if reference
        != canonical
            .to_str()
            .ok_or("delta_artifact_path_utf8_required=true")?
        && !replay_legacy
        && !ordinary_legacy
    {
        return Err("delta_artifact_reference_alias_forbidden=true".into());
    }
    // The recorded digest binds immutable bytes. Reusing verified bytes must
    // not depend on today's Git settings or require old rendering inputs.
    prepare(project_root)?;
    let path = match fs::symlink_metadata(&canonical) {
        Ok(_) => {
            if digest(project_root, &canonical)? != expected {
                return Err(format!("delta_artifact_existing_digest_mismatch=true path={} recovery=quarantine_tampered_cache_then_retrieve_pending_assignment", canonical.display()));
            }
            canonical
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let (candidate, file) = Candidate::create(project_root)?;
            write_delta_snapshot_patch(
                project_root,
                &evidence.prior_snapshot_commit,
                &evidence.current_snapshot_commit,
                &evidence.changed_paths,
                evidence.patch_rendering.as_deref(),
                file,
            )?;
            candidate.publish(project_root, Some(expected)).map_err(|error| {
                if evidence.patch_rendering.is_none() && error.contains("snapshot_digest_mismatch") {
                    format!("{error} legacy_rendering_unavailable=true recovery=restore_original_git_rendering_configuration_or_restart_review_with_current_scope")
                } else { error }
            })?.0
        }
        Err(error) => return Err(format!("delta_artifact_target_failed source={error}")),
    };
    if Path::new(reference) != path {
        let mut updated = raw.clone();
        updated["artifact_reference"] = json!(path);
        assignment["delta_evidence"] = updated.clone();
        if let Some(prompt) = assignment.get("prompt").and_then(Value::as_str) {
            let mut prompt: Value = serde_json::from_str(prompt)
                .map_err(|error| format!("delta_artifact_prompt_invalid source={error}"))?;
            prompt["current_review"]["delta_evidence"] = updated;
            assignment["prompt"] = json!(prompt.to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::ffi::OsStringExt;
    #[test]
    fn cache_keys_bind_raw_canonical_root_bytes() {
        let fixture = tempfile::tempdir_in("/tmp").unwrap();
        let first = fixture
            .path()
            .join(std::ffi::OsString::from_vec(vec![b'r', 0xfe]));
        let second = fixture
            .path()
            .join(std::ffi::OsString::from_vec(vec![b'r', 0xff]));
        fs::create_dir(&first).unwrap();
        fs::create_dir(&second).unwrap();
        assert_ne!(directory(&first).unwrap(), directory(&second).unwrap());
    }
    #[test]
    fn interrupted_unpublished_candidate_is_bounded_and_recovered() {
        let fixture = tempfile::tempdir_in("/tmp").unwrap();
        let parent = prepare(fixture.path()).unwrap();
        let interrupted = parent.join(".candidate.patch");
        fs::write(&interrupted, b"partially written unpublished bytes").unwrap();
        let (candidate, mut file) = Candidate::create(fixture.path()).unwrap();
        file.write_all(b"complete new bytes").unwrap();
        drop(file);
        drop(candidate);
        assert!(
            !interrupted.exists(),
            "next locked operation removes abandoned staging, never active artifacts"
        );
    }
}
