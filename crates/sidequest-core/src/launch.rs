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

/// Derive the side-quest branch for a goal, e.g. `"Fix the Action Buttons!"` ->
/// `"side-quest/fix-the-action-buttons"`.
///
/// # Errors
///
/// Returns a `BranchNameError` when the goal slugifies to nothing (it contained
/// no alphanumeric characters).
pub fn branch_for_goal(goal: &Goal) -> Result<BranchName, BranchNameError> {
    BranchName::try_new(format!("side-quest/{}", slugify(goal.as_ref())))
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
