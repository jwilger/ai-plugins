//! Core vocabulary for launching a side-quest: the goal it pursues and the git
//! branch its work lands on. Pure — no I/O.

use nutype::nutype;

/// A side-quest's objective, as the user phrased it. Trimmed and non-empty.
#[nutype(
    sanitize(trim),
    validate(not_empty),
    derive(Debug, Clone, PartialEq, Eq, Display, AsRef, Serialize, Deserialize)
)]
pub struct Goal(String);

/// A git branch name for a side-quest's work: non-empty and restricted to
/// ref-safe slug characters (`a-z`, `0-9`, `-`, `/`).
#[nutype(
    validate(not_empty, predicate = is_ref_safe),
    derive(Debug, Clone, PartialEq, Eq, Display, AsRef, Serialize, Deserialize)
)]
pub struct BranchName(String);

fn is_ref_safe(candidate: &str) -> bool {
    // Non-empty `/`-separated segments, each a lowercase ASCII slug. Rejects
    // leading/trailing/repeated slashes (e.g. an empty slug yielding
    // `"side-quest/"`).
    !candidate.starts_with('/')
        && !candidate.ends_with('/')
        && candidate.split('/').all(|segment| {
            !segment.is_empty()
                && segment
                    .chars()
                    .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
        })
}

/// The longest slug `branch_for_goal` will produce (excluding the
/// `side-quest/` prefix). Chosen well under filesystem/ref name limits (e.g.
/// ext4's 255-byte `NAME_MAX`), since the slug also becomes the worktree's
/// leaf directory name.
const MAX_SLUG_LEN: usize = 50;

/// The length, in hex digits, of the uniqueness suffix appended to a
/// truncated slug.
const HASH_SUFFIX_LEN: usize = 8;

/// Derive the side-quest branch for a goal, e.g. `"Fix the Action Buttons!"` ->
/// `"side-quest/fix-the-action-buttons"`.
///
/// # Errors
///
/// Returns a `BranchNameError` when the goal slugifies to nothing (it contained
/// no alphanumeric characters).
pub fn branch_for_goal(goal: &Goal) -> Result<BranchName, BranchNameError> {
    let slug = bound_slug(&slugify(goal.as_ref()));
    BranchName::try_new(format!("side-quest/{slug}"))
}

/// Lowercase ASCII-alphanumeric slug, words joined by single dashes, with no
/// leading, trailing, or repeated dashes.
fn slugify(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Cap `slug` at [`MAX_SLUG_LEN`] bytes. A slug that must be truncated gets a
/// short hash of its untruncated form appended, since `BranchName` is a
/// side-quest's sole registry identity: two long goals that happen to share a
/// prefix must not collapse onto the same branch.
fn bound_slug(slug: &str) -> String {
    if slug.len() <= MAX_SLUG_LEN {
        return slug.to_owned();
    }
    let suffix = format!("{:08x}", slug_fingerprint(slug));
    let budget = MAX_SLUG_LEN - HASH_SUFFIX_LEN - 1;
    let truncated = slug[..budget].trim_end_matches('-');
    format!("{truncated}-{suffix}")
}

/// A short, deterministic fingerprint of `slug`, used only to disambiguate
/// truncated slugs (not a security or storage-format concern).
#[expect(
    clippy::cast_possible_truncation,
    reason = "only a short disambiguating fingerprint is needed, not the full hash"
)]
fn slug_fingerprint(slug: &str) -> u32 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    slug.hash(&mut hasher);
    hasher.finish() as u32
}

#[cfg(test)]
#[expect(clippy::expect_used, reason = "unit tests use expect() for clarity")]
mod tests {
    use super::*;

    #[test]
    fn goal_is_trimmed_and_required() {
        let goal = Goal::try_new("  fix the buttons  ").expect("a non-empty goal is valid");
        assert_eq!(
            goal.as_ref(),
            "fix the buttons",
            "surrounding whitespace is trimmed"
        );
        assert!(
            Goal::try_new("   ").is_err(),
            "a whitespace-only goal is rejected"
        );
    }

    #[test]
    fn branch_is_a_slug_under_side_quest() {
        let goal = Goal::try_new("Fix the Action Buttons!").expect("a non-empty goal is valid");
        let branch = branch_for_goal(&goal).expect("an alphanumeric goal yields a branch");
        assert_eq!(
            branch.as_ref(),
            "side-quest/fix-the-action-buttons",
            "the goal is slugified under the side-quest/ prefix"
        );
    }

    #[test]
    fn slug_at_the_length_cap_is_left_unchanged() {
        let goal = Goal::try_new("x".repeat(MAX_SLUG_LEN)).expect("a non-empty goal is valid");
        let branch = branch_for_goal(&goal).expect("a run of alphanumerics yields a branch");
        assert_eq!(
            branch.as_ref(),
            format!("side-quest/{}", "x".repeat(MAX_SLUG_LEN)),
            "a slug exactly at the length cap is not truncated or hashed"
        );
    }

    #[test]
    fn slug_over_the_length_cap_is_truncated_with_a_hash_suffix() {
        let goal = Goal::try_new("x".repeat(MAX_SLUG_LEN + 1)).expect("a non-empty goal is valid");
        let branch =
            branch_for_goal(&goal).expect("an overlong alphanumeric goal still yields a branch");
        let slug = branch
            .as_ref()
            .strip_prefix("side-quest/")
            .expect("the branch keeps the side-quest/ prefix");
        assert_eq!(
            slug.len(),
            MAX_SLUG_LEN,
            "a dash-free overlong slug fills the truncation budget exactly, pinning the \
             length/hash-suffix arithmetic: {slug:?}"
        );
        assert_ne!(
            slug,
            "x".repeat(MAX_SLUG_LEN + 1),
            "an overlong slug is actually truncated, not merely accepted as-is"
        );
        assert!(
            slug.chars()
                .rev()
                .take(HASH_SUFFIX_LEN)
                .all(|ch| ch.is_ascii_hexdigit()),
            "a truncated slug ends with a hash suffix: {slug:?}"
        );
    }

    #[test]
    fn distinct_overlong_goals_sharing_a_prefix_yield_distinct_branches() {
        let shared_prefix = "shared prefix words repeated ".repeat(5);
        let goal_a = Goal::try_new(format!("{shared_prefix} alpha tail"))
            .expect("a non-empty goal is valid");
        let goal_b =
            Goal::try_new(format!("{shared_prefix} beta tail")).expect("a non-empty goal is valid");
        let branch_a = branch_for_goal(&goal_a).expect("a non-empty goal yields a branch");
        let branch_b = branch_for_goal(&goal_b).expect("a non-empty goal yields a branch");
        assert_ne!(
            branch_a, branch_b,
            "two long goals truncating to the same prefix must still get distinct branches, \
             since the branch is the side-quest's sole registry identity"
        );
    }

    #[test]
    fn branch_for_goal_is_deterministic() {
        let goal = Goal::try_new("x".repeat(MAX_SLUG_LEN + 1)).expect("a non-empty goal is valid");
        assert_eq!(
            branch_for_goal(&goal).expect("valid branch"),
            branch_for_goal(&goal).expect("valid branch"),
            "deriving a branch from the same overlong goal twice is deterministic"
        );
    }

    #[test]
    fn goal_without_alphanumerics_has_no_branch() {
        let goal = Goal::try_new("!!! ???").expect("punctuation is still a non-empty goal");
        assert!(
            branch_for_goal(&goal).is_err(),
            "an empty slug cannot form a branch"
        );
    }

    #[test]
    fn branch_name_requires_ref_safe_slug_segments() {
        for bad in [
            "FOO",            // an uppercase (non-slug) character
            "side-quest/Bad", // an invalid character in a later segment
            "foo//bar",       // an empty interior segment (repeated slash)
            "/leading",       // a leading slash
            "trailing/",      // a trailing slash
            "has space",      // whitespace is not ref-safe
        ] {
            assert!(
                BranchName::try_new(bad).is_err(),
                "{bad:?} is not a ref-safe branch name"
            );
        }
        assert!(
            BranchName::try_new("side-quest/fix-the-buttons").is_ok(),
            "a lowercase slug under a prefix is ref-safe"
        );
    }
}
