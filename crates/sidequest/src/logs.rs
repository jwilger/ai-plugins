//! Where a side-quest's session output is logged (imperative shell).
//!
//! Stored under the project's `.git/` directory, alongside the registry, so
//! it is shared across the main checkout and its worktrees and never shows up
//! as a tracked change.

use std::path::{Path, PathBuf};

use sidequest_core::launch::BranchName;
use tokio::io::{AsyncReadExt as _, AsyncSeekExt as _};

/// The most of a log this crate will ever read into memory in one call,
/// regardless of how large the file on disk has grown.
const MAX_TAIL_BYTES: u64 = 256 * 1024;

/// The path a side-quest's session output is (or will be) logged to.
#[must_use]
pub fn path(project_root: &Path, branch: &BranchName) -> PathBuf {
    let file_name = branch.as_ref().replace('/', "-");
    project_root
        .join(".git")
        .join("sidequest")
        .join("logs")
        .join(format!("{file_name}.log"))
}

/// Read up to the last [`MAX_TAIL_BYTES`] of the file at `path`, regardless of
/// its total size, so polling a long-running side-quest's log never re-reads
/// (or re-allocates) the whole file.
///
/// # Errors
///
/// Returns the underlying [`std::io::Error`] if the file cannot be opened,
/// seeked, or read (including `NotFound` if it does not exist yet).
pub async fn read_tail(path: &Path) -> std::io::Result<String> {
    let mut file = tokio::fs::File::open(path).await?;
    let len = file.metadata().await?.len();
    file.seek(std::io::SeekFrom::Start(len.saturating_sub(MAX_TAIL_BYTES)))
        .await?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).await?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

#[cfg(test)]
#[expect(clippy::expect_used, reason = "unit tests use expect() for clarity")]
mod tests {
    use super::*;

    #[test]
    fn path_is_filesystem_safe_and_stable() {
        let branch = BranchName::try_new("side-quest/fix-the-buttons").expect("a ref-safe branch");
        let path = path(Path::new("/repo"), &branch);
        assert_eq!(
            path,
            Path::new("/repo/.git/sidequest/logs/side-quest-fix-the-buttons.log"),
            "the branch's slashes are flattened into a single filename"
        );
    }

    #[tokio::test]
    async fn read_tail_never_exceeds_the_byte_cap() {
        let dir = tempfile::tempdir().expect("a temp dir is creatable");
        let file_path = dir.path().join("big.log");
        // 300_000 and 262_144 are independent, hardcoded expectations (not
        // derived from MAX_TAIL_BYTES), so a mutation to the cap's own
        // definition still gets caught here.
        let oversized = "A".repeat(300_000);
        std::fs::write(&file_path, &oversized).expect("the log file is writable");

        let tail = read_tail(&file_path)
            .await
            .expect("the log file is readable");

        assert_eq!(
            tail.len(),
            262_144,
            "a log larger than the 256 KiB cap is read back as exactly 256 KiB of trailing bytes"
        );
        assert!(
            oversized.ends_with(&tail),
            "the bytes returned are the file's suffix, not an arbitrary slice"
        );
    }

    #[tokio::test]
    async fn read_tail_returns_the_whole_file_when_under_the_cap() {
        let dir = tempfile::tempdir().expect("a temp dir is creatable");
        let file_path = dir.path().join("small.log");
        std::fs::write(&file_path, "hello").expect("the log file is writable");

        let tail = read_tail(&file_path)
            .await
            .expect("the log file is readable");

        assert_eq!(
            tail, "hello",
            "a log under the cap is returned in full, unmodified"
        );
    }
}
