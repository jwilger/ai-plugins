use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskTitle(String);

impl TaskTitle {
    pub fn parse(input: &str) -> Result<Self, CoreError> {
        let title = input.trim();
        if title.is_empty() {
            return Err(CoreError::EmptyTitle);
        }
        if title.chars().any(char::is_control) {
            return Err(CoreError::InvalidTitle);
        }
        Ok(Self(title.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn file_stem(&self) -> String {
        let mut slug = String::new();
        let mut previous_was_separator = true;

        for character in self.0.chars().flat_map(char::to_lowercase) {
            if character.is_ascii_alphanumeric() {
                slug.push(character);
                previous_was_separator = false;
            } else if !previous_was_separator {
                slug.push('-');
                previous_was_separator = true;
            }
        }

        if slug.ends_with('-') {
            slug.pop();
        }

        if slug.is_empty() {
            "task".to_string()
        } else {
            slug
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskSnapshot {
    path: String,
    title: String,
}

impl TaskSnapshot {
    pub fn new(path: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            title: title.into(),
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn title(&self) -> &str {
        &self.title
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoardSnapshot {
    ordered_tasks: Vec<TaskSnapshot>,
}

impl BoardSnapshot {
    pub fn from_ordered_tasks(ordered_tasks: Vec<TaskSnapshot>) -> Self {
        Self { ordered_tasks }
    }

    pub fn ordered_tasks(&self) -> &[TaskSnapshot] {
        &self.ordered_tasks
    }

    pub fn next_task(&self) -> Option<&TaskSnapshot> {
        self.ordered_tasks.first()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskDependencies {
    task_ref: String,
    blocks: Vec<String>,
}

impl TaskDependencies {
    pub fn new(task_ref: impl Into<String>, blocks: Vec<impl Into<String>>) -> Self {
        Self {
            task_ref: task_ref.into(),
            blocks: blocks.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyGraph {
    tasks: Vec<TaskDependencies>,
}

impl DependencyGraph {
    pub fn from_tasks(tasks: Vec<TaskDependencies>) -> Self {
        Self { tasks }
    }

    pub fn cycle_messages(&self) -> Vec<String> {
        self.cycle_messages_with_label("dependency")
    }

    pub fn cycle_messages_with_label(&self, label: &str) -> Vec<String> {
        let task_refs = self
            .tasks
            .iter()
            .map(|task| task.task_ref.clone())
            .collect::<Vec<_>>();
        let mut reported = Vec::new();
        for task_ref in &task_refs {
            let mut path = Vec::new();
            self.find_cycles(task_ref, task_ref, &task_refs, &mut path, &mut reported);
        }
        reported.sort();
        reported.dedup();
        reported
            .into_iter()
            .map(|cycle| format!("cycle {label} {}", cycle.join(" -> ")))
            .collect()
    }

    fn find_cycles(
        &self,
        start: &str,
        current: &str,
        task_refs: &[String],
        path: &mut Vec<String>,
        reported: &mut Vec<Vec<String>>,
    ) {
        if path.iter().any(|task_ref| task_ref == current) {
            return;
        }
        path.push(current.to_string());
        for blocked_ref in self.blocks_for(current) {
            if !task_refs.contains(blocked_ref) {
                continue;
            }
            if blocked_ref == start {
                let mut cycle = path.clone();
                cycle.push(start.to_string());
                if path.first().is_some_and(|first| first == start) {
                    if let Some(canonical) = canonical_cycle(&cycle) {
                        reported.push(canonical);
                    }
                }
            } else {
                self.find_cycles(start, blocked_ref, task_refs, path, reported);
            }
        }
        path.pop();
    }

    fn blocks_for(&self, task_ref: &str) -> &[String] {
        self.tasks
            .iter()
            .find(|task| task.task_ref == task_ref)
            .map(|task| task.blocks.as_slice())
            .unwrap_or_default()
    }
}

fn canonical_cycle(cycle: &[String]) -> Option<Vec<String>> {
    let nodes = cycle.split_last()?.1;
    let start_index = nodes
        .iter()
        .enumerate()
        .min_by(|(_left_index, left), (_right_index, right)| left.cmp(right))?
        .0;
    let mut canonical = nodes[start_index..].to_vec();
    canonical.extend_from_slice(&nodes[..start_index]);
    canonical.push(canonical.first()?.clone());
    Some(canonical)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderReconciliation {
    entries: Vec<String>,
    messages: Vec<String>,
}

impl OrderReconciliation {
    pub fn reconcile(
        existing_entries: Vec<impl Into<String>>,
        task_refs: Vec<impl Into<String>>,
    ) -> Self {
        let existing_entries = existing_entries
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>();
        let task_refs = task_refs.into_iter().map(Into::into).collect::<Vec<_>>();
        let mut entries = Vec::new();
        let mut messages = Vec::new();
        for task_ref in &existing_entries {
            if task_refs.contains(task_ref) {
                entries.push(task_ref.clone());
            } else {
                messages.push(format!("fixed order stale {task_ref}"));
            }
        }
        for task_ref in &task_refs {
            if !entries.contains(task_ref) {
                messages.push(format!("fixed order missing {task_ref}"));
                entries.push(task_ref.clone());
            }
        }
        Self { entries, messages }
    }

    pub fn entries(&self) -> &[String] {
        &self.entries
    }

    pub fn messages(&self) -> &[String] {
        &self.messages
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum CoreError {
    EmptyTitle,
    InvalidTitle,
}

impl fmt::Display for CoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTitle => write!(formatter, "tiber.empty_title"),
            Self::InvalidTitle => write!(formatter, "tiber.invalid_title"),
        }
    }
}

impl std::error::Error for CoreError {}

#[cfg(test)]
mod tests {
    use super::{
        BoardSnapshot, CoreError, DependencyGraph, OrderReconciliation, TaskDependencies,
        TaskSnapshot, TaskTitle,
    };

    #[test]
    fn task_title_parse_trims_input_and_rejects_empty_titles() {
        let title = TaskTitle::parse("  Ship tiber  ").expect("title should parse");

        assert_eq!(title.as_str(), "Ship tiber");
        assert_eq!(TaskTitle::parse(" \t\n"), Err(CoreError::EmptyTitle));
        assert_eq!(
            TaskTitle::parse("Ship\nTiber"),
            Err(CoreError::InvalidTitle)
        );
    }

    #[test]
    fn task_title_file_stem_slugifies_ascii_words() {
        let title = TaskTitle::parse(" Fix: API + UI handoff! ").expect("title should parse");

        assert_eq!(title.file_stem(), "fix-api-ui-handoff");
    }

    #[test]
    fn task_title_file_stem_falls_back_when_title_has_no_ascii_slug() {
        let title = TaskTitle::parse("✓✓✓").expect("title should parse");

        assert_eq!(title.file_stem(), "task");
    }

    #[test]
    fn core_error_display_is_stable_for_cli_errors() {
        assert_eq!(CoreError::EmptyTitle.to_string(), "tiber.empty_title");
        assert_eq!(CoreError::InvalidTitle.to_string(), "tiber.invalid_title");
    }

    #[test]
    fn board_snapshot_preserves_ordered_task_summaries_and_next_task() {
        let snapshot = BoardSnapshot::from_ordered_tasks(vec![
            TaskSnapshot::new("todo/write-docs.md", "Write docs"),
            TaskSnapshot::new("doing/review-docs.md", "Review docs"),
        ]);

        assert_eq!(
            snapshot.ordered_tasks(),
            [
                TaskSnapshot::new("todo/write-docs.md", "Write docs"),
                TaskSnapshot::new("doing/review-docs.md", "Review docs"),
            ]
        );
        assert_eq!(
            snapshot.next_task(),
            Some(&TaskSnapshot::new("todo/write-docs.md", "Write docs"))
        );
    }

    #[test]
    fn dependency_graph_reports_canonical_cycles_once() {
        let graph = DependencyGraph::from_tasks(vec![
            TaskDependencies::new("todo/cycle-b.md", vec!["todo/cycle-a.md"]),
            TaskDependencies::new("todo/cycle-a.md", vec!["todo/cycle-b.md"]),
        ]);

        assert_eq!(
            graph.cycle_messages(),
            ["cycle dependency todo/cycle-a.md -> todo/cycle-b.md -> todo/cycle-a.md"]
        );
    }

    #[test]
    fn order_reconciliation_reports_stale_entries_and_appends_missing_tasks() {
        let reconciliation = OrderReconciliation::reconcile(
            vec!["todo/build-api.md", "todo/stale.md"],
            vec!["todo/build-api.md", "todo/build-ui.md"],
        );

        assert_eq!(
            reconciliation.entries(),
            ["todo/build-api.md", "todo/build-ui.md"]
        );
        assert_eq!(
            reconciliation.messages(),
            [
                "fixed order stale todo/stale.md",
                "fixed order missing todo/build-ui.md"
            ]
        );
    }
}
