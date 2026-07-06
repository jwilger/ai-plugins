use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
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
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
