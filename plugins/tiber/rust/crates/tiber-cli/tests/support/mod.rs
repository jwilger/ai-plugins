use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn assert_success(output: Output) {
    assert_success_ref(&output);
}

pub fn assert_success_ref(output: &Output) {
    assert!(
        output.status.success(),
        "command failed\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[allow(dead_code)]
pub fn task_stem(repo: &TempRepo, status: &str, nickname: &str) -> String {
    let tree = repo.git_output(["ls-tree", "-r", "--name-only", "tasks", status]);
    assert_success_ref(&tree);
    let mut matches = String::from_utf8(tree.stdout)
        .expect("tree output should be utf8")
        .lines()
        .filter_map(|path| {
            path.strip_prefix(&format!("{status}/"))
                .and_then(|name| name.strip_suffix(".md"))
                .filter(|stem| stem.ends_with(&format!("-{nickname}")))
                .map(str::to_string)
        })
        .collect::<Vec<_>>();
    matches.sort();
    assert_eq!(matches.len(), 1, "expected one task matching {nickname}");
    matches.remove(0)
}

pub struct TempRepo {
    path: PathBuf,
}

impl TempRepo {
    pub fn new() -> Self {
        static TEMP_REPO_SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos();
        let sequence = TEMP_REPO_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "tiber-cli-test-{}-{unique}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create temp repo");
        Self { path }
    }

    pub fn initialized() -> Self {
        let repo = Self::new();
        repo.git(["init", "-b", "main"]);
        repo.git(["config", "user.email", "tiber@example.test"]);
        repo.git(["config", "user.name", "Tiber Test"]);
        repo.git(["config", "commit.gpgsign", "false"]);
        fs::write(repo.path().join("README.md"), "# test repo\n").expect("write readme");
        repo.git(["add", "README.md"]);
        repo.git(["commit", "-m", "Initial commit"]);
        repo
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    #[allow(dead_code)]
    pub fn tiber<I, S>(&self, args: I) -> Output
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        self.command(env!("CARGO_BIN_EXE_tiber"), args)
    }

    #[allow(dead_code)]
    pub fn tiber_with_env<I, S, E, K, V>(&self, args: I, envs: E) -> Output
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
        E: IntoIterator<Item = (K, V)>,
        K: AsRef<std::ffi::OsStr>,
        V: AsRef<std::ffi::OsStr>,
    {
        let mut command = Command::new(env!("CARGO_BIN_EXE_tiber"));
        command.args(args).envs(envs).current_dir(&self.path);
        command.output().expect("run tiber")
    }

    pub fn command<I, S>(&self, program: &str, args: I) -> Output
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        Command::new(program)
            .args(args)
            .current_dir(&self.path)
            .output()
            .expect("run command")
    }

    pub fn command_with_env<I, S, E, K, V>(&self, program: &str, args: I, envs: E) -> Output
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
        E: IntoIterator<Item = (K, V)>,
        K: AsRef<std::ffi::OsStr>,
        V: AsRef<std::ffi::OsStr>,
    {
        Command::new(program)
            .args(args)
            .envs(envs)
            .current_dir(&self.path)
            .output()
            .expect("run command")
    }

    pub fn git<I, S>(&self, args: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        assert_success(self.git_output(args));
    }

    pub fn git_output<I, S>(&self, args: I) -> Output
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        self.command("git", args)
    }

    #[allow(dead_code)]
    pub fn task_file(&self, status: &str, stem: &str) -> String {
        let output = self.git_output(["show", &format!("tasks:{status}/{stem}.md")]);
        assert_success_ref(&output);
        String::from_utf8(output.stdout).expect("task file should be utf8")
    }

    #[allow(dead_code)]
    pub fn order_file(&self) -> String {
        let output = self.git_output(["show", "tasks:order.md"]);
        assert_success_ref(&output);
        String::from_utf8(output.stdout).expect("order file should be utf8")
    }

    #[allow(dead_code)]
    pub fn insert_task_file(&self, status: &str, stem: &str, contents: &str) {
        self.insert_tasks_tree_file(&format!("{status}/{stem}.md"), contents);
    }

    #[allow(dead_code)]
    pub fn insert_tasks_tree_file(&self, path: &str, contents: &str) {
        let blob = Command::new("git")
            .args(["hash-object", "-w", "--stdin"])
            .current_dir(&self.path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child
                    .stdin
                    .as_mut()
                    .expect("hash-object stdin")
                    .write_all(contents.as_bytes())?;
                child.wait_with_output()
            })
            .expect("write task blob");
        assert_success_ref(&blob);
        let blob = String::from_utf8(blob.stdout)
            .expect("blob should be utf8")
            .trim()
            .to_string();
        let index = self.path.join(".git").join("tiber-test-index");
        assert_success(self.command_with_env(
            "git",
            ["read-tree", "tasks"],
            [("GIT_INDEX_FILE", index.as_os_str())],
        ));
        assert_success(self.command_with_env(
            "git",
            [
                "update-index",
                "--add",
                "--cacheinfo",
                "100644",
                &blob,
                path,
            ],
            [("GIT_INDEX_FILE", index.as_os_str())],
        ));
        let tree = self.command_with_env(
            "git",
            ["write-tree"],
            [("GIT_INDEX_FILE", index.as_os_str())],
        );
        assert_success_ref(&tree);
        let tree = String::from_utf8(tree.stdout)
            .expect("tree should be utf8")
            .trim()
            .to_string();
        let commit = self.git_output([
            "commit-tree",
            &tree,
            "-p",
            "tasks",
            "-m",
            "Insert test task",
        ]);
        assert_success_ref(&commit);
        let commit = String::from_utf8(commit.stdout)
            .expect("commit should be utf8")
            .trim()
            .to_string();
        assert_success(self.git_output(["update-ref", "refs/heads/tasks", &commit]));
        let _ = fs::remove_file(index);
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
