mod support;

use std::fs;

use support::{assert_success, assert_success_ref, TempRepo};

#[test]
fn install_bin_dry_run_previews_without_writing_and_apply_installs_launcher() {
    let repo = TempRepo::initialized();
    let target_dir = repo.path().join("bin");
    let launcher = repo.path().join("plugin/bin/tiber");
    fs::create_dir_all(launcher.parent().expect("launcher parent")).expect("create launcher dir");
    fs::write(&launcher, "#!/usr/bin/env bash\n").expect("write fake launcher");

    let dry_run = repo.tiber_with_env(
        [
            "install-bin",
            "--target-dir",
            target_dir.to_str().expect("target dir utf8"),
            "--dry-run",
        ],
        [(
            "TIBER_LAUNCHER_PATH",
            launcher.to_str().expect("launcher path utf8"),
        )],
    );

    assert_success_ref(&dry_run);
    assert_eq!(
        String::from_utf8(dry_run.stdout).expect("dry-run output should be utf8"),
        format!(
            "would install {} -> {}\n",
            target_dir.join("tiber").display(),
            launcher.display()
        )
    );
    assert!(!target_dir.join("tiber").exists());

    let apply = repo.tiber_with_env(
        [
            "install-bin",
            "--target-dir",
            target_dir.to_str().expect("target dir utf8"),
            "--apply",
        ],
        [(
            "TIBER_LAUNCHER_PATH",
            launcher.to_str().expect("launcher path utf8"),
        )],
    );

    assert_success(apply);
    assert_eq!(
        fs::read_link(target_dir.join("tiber")).expect("installed tiber should be symlink"),
        launcher
    );
}
