use std::convert::Infallible;
use std::env;
use std::ffi::OsString;
use std::process::ExitCode;
use std::str::FromStr;

use clap::{error::ErrorKind, ArgGroup, Args, CommandFactory, Parser, Subcommand};

const ESCAPED_SUBTASK_TITLE_PREFIX: &str = "\0tiber-subtask-title\0";
const EXPLICIT_INSTALL_TARGET_PREFIX: &str = "\0tiber-install-target\0";

#[derive(Clone)]
struct CommaSeparatedValues(Vec<String>);

impl CommaSeparatedValues {
    fn into_values(self) -> Vec<String> {
        self.0
    }
}

impl FromStr for CommaSeparatedValues {
    type Err = Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self(
            value
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_string)
                .collect(),
        ))
    }
}

#[derive(Clone)]
struct SubtaskTitle(String);

impl SubtaskTitle {
    fn into_value(self) -> String {
        self.0
    }
}

impl FromStr for SubtaskTitle {
    type Err = Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self(
            value
                .strip_prefix(ESCAPED_SUBTASK_TITLE_PREFIX)
                .unwrap_or(value)
                .to_string(),
        ))
    }
}

#[derive(Clone)]
struct InstallTargetDir(String);

impl InstallTargetDir {
    fn into_value(self) -> String {
        self.0
    }
}

impl FromStr for InstallTargetDir {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if let Some(explicit) = value.strip_prefix(EXPLICIT_INSTALL_TARGET_PREFIX) {
            return Ok(Self(explicit.to_string()));
        }
        if matches!(value, "--dry-run" | "--apply") {
            return Err(format!(
                "{value} is an install mode; use --target-dir={value} for that literal path"
            ));
        }
        Ok(Self(value.to_string()))
    }
}

#[derive(Parser)]
#[command(name = "tiber", about = "Repository-local task board")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize the tasks branch.
    Init,
    /// Synchronize local task state with origin/tasks.
    Sync,
    /// Show Codex sandbox approval guidance.
    CodexSandbox(CodexSandboxArgs),
    /// Run the dashboard with count-neutral backlog priority reordering.
    Dashboard(DashboardArgs),
    /// Run the MCP server.
    Mcp(McpArgs),
    /// Preview or install the bundled tiber launcher.
    InstallBin(InstallBinArgs),
    /// Create a backlog task.
    Create {
        /// Task title.
        #[arg(allow_hyphen_values = true)]
        title: String,
    },
    /// Show a task document.
    Show(TaskRefArgs),
    /// Show task metadata.
    Metadata(TaskRefArgs),
    /// List open tasks.
    List,
    /// Print the next available task.
    Next,
    /// Move a task to a status.
    Transition(TransitionArgs),
    /// Reorder a task before another task.
    Prioritize(PrioritizeArgs),
    /// Add a task dependency relation.
    Link(RelationArgs),
    /// Remove a task dependency relation.
    Unlink(RelationArgs),
    /// Edit task subtasks.
    Subtask(SubtaskArgs),
    /// Update structured task fields.
    Update(UpdateArgs),
    /// Edit task acceptance criteria.
    Acceptance(AcceptanceArgs),
    /// Append a dated task note.
    Note(NoteArgs),
    /// Validate and repair safe task-board issues.
    Validate(ValidateArgs),
    /// Close tasks from Git trailers.
    CloseFromTrailers,
    /// Scaffold repository integration.
    Scaffold(ScaffoldArgs),
}

#[derive(Args)]
struct CodexSandboxArgs {
    /// Print a dry-run preview.
    #[arg(long, required = true)]
    dry_run: bool,
}

#[derive(Args)]
struct DashboardArgs {
    #[command(subcommand)]
    command: DashboardCommand,
}

#[derive(Subcommand)]
enum DashboardCommand {
    /// Serve the dashboard on localhost.
    Serve,
}

#[derive(Args)]
struct McpArgs {
    #[command(subcommand)]
    transport: McpTransport,
}

#[derive(Subcommand)]
enum McpTransport {
    /// Use stdio transport.
    Stdio,
}

#[derive(Args)]
#[command(group(ArgGroup::new("mode").required(true).args(["dry_run", "apply"])))]
struct InstallBinArgs {
    /// Directory where the launcher should be installed.
    #[arg(long, allow_hyphen_values = true)]
    target_dir: InstallTargetDir,
    /// Preview without writing.
    #[arg(long)]
    dry_run: bool,
    /// Install the launcher.
    #[arg(long)]
    apply: bool,
}

#[derive(Args)]
struct TaskRefArgs {
    /// Task id, nickname, or full stem.
    task_ref: String,
}

#[derive(Args)]
struct TransitionArgs {
    /// Task id, nickname, or full stem.
    task_ref: String,
    /// Target status.
    status: String,
}

#[derive(Args)]
struct PrioritizeArgs {
    /// Task id, nickname, or full stem.
    task_ref: String,
    /// Place the task before this task.
    #[arg(long)]
    before: String,
}

#[derive(Args)]
struct RelationArgs {
    /// Source task id, nickname, or full stem.
    from_ref: String,
    /// Dependency relation.
    #[command(subcommand)]
    relation: RelationCommand,
}

#[derive(Subcommand)]
enum RelationCommand {
    /// The source task blocks another task.
    Blocks {
        /// Target task id, nickname, or full stem.
        to_ref: String,
    },
}

#[derive(Args)]
struct SubtaskArgs {
    #[command(subcommand)]
    command: SubtaskCommand,
}

#[derive(Subcommand)]
enum SubtaskCommand {
    /// Add a subtask.
    Add(SubtaskAddArgs),
    /// Mark a subtask complete.
    Check {
        /// Task id, nickname, or full stem.
        task_ref: String,
        /// 1-based subtask index.
        index: String,
    },
    /// Mark a subtask incomplete.
    Uncheck {
        /// Task id, nickname, or full stem.
        task_ref: String,
        /// 1-based subtask index.
        index: String,
    },
}

#[derive(Args)]
struct SubtaskAddArgs {
    /// Task id, nickname, or full stem.
    task_ref: String,
    /// Subtask title.
    #[arg(allow_hyphen_values = true)]
    title: SubtaskTitle,
    /// Comma-separated predecessor subtask refs.
    #[arg(long)]
    after: Option<CommaSeparatedValues>,
}

#[derive(Args)]
#[command(args_override_self = true)]
#[command(
    after_help = "To use a recognized option token as a literal field value, attach it with `=` (for example, `--summary=--tags`)."
)]
#[command(group(
    ArgGroup::new("field")
        .required(true)
        .multiple(true)
        .args(["title", "summary", "context", "tags", "pr_mr_url", "pr_mr_status"])
))]
struct UpdateArgs {
    /// Task id, nickname, or full stem.
    task_ref: String,
    #[arg(long, allow_hyphen_values = true)]
    title: Option<String>,
    #[arg(long, allow_hyphen_values = true)]
    summary: Option<String>,
    #[arg(long, allow_hyphen_values = true)]
    context: Option<String>,
    #[arg(long, allow_hyphen_values = true)]
    tags: Option<CommaSeparatedValues>,
    #[arg(long, allow_hyphen_values = true)]
    pr_mr_url: Option<String>,
    #[arg(long, allow_hyphen_values = true)]
    pr_mr_status: Option<String>,
}

#[derive(Args)]
struct AcceptanceArgs {
    #[command(subcommand)]
    command: AcceptanceCommand,
}

#[derive(Subcommand)]
enum AcceptanceCommand {
    /// Add an acceptance criterion.
    Add {
        /// Task id, nickname, or full stem.
        task_ref: String,
        /// Criterion text.
        #[arg(allow_hyphen_values = true)]
        criterion: String,
    },
    /// Mark an acceptance criterion complete.
    Check {
        /// Task id, nickname, or full stem.
        task_ref: String,
        /// 1-based criterion index.
        index: String,
    },
    /// Mark an acceptance criterion incomplete.
    Uncheck {
        /// Task id, nickname, or full stem.
        task_ref: String,
        /// 1-based criterion index.
        index: String,
    },
    /// Remove an acceptance criterion.
    Remove {
        /// Task id, nickname, or full stem.
        task_ref: String,
        /// 1-based criterion index.
        index: String,
    },
}

#[derive(Args)]
struct NoteArgs {
    #[command(subcommand)]
    command: NoteCommand,
}

#[derive(Subcommand)]
enum NoteCommand {
    /// Add a dated note.
    Add {
        /// Task id, nickname, or full stem.
        task_ref: String,
        /// Note text.
        #[arg(allow_hyphen_values = true)]
        note: String,
    },
}

#[derive(Args)]
struct ValidateArgs {
    /// Apply safe repairs.
    #[arg(long, required = true)]
    fix: bool,
}

#[derive(Args)]
struct ScaffoldArgs {
    #[command(subcommand)]
    command: ScaffoldCommand,
}

#[derive(Subcommand)]
enum ScaffoldCommand {
    /// Scaffold repository integration.
    Repo(ScaffoldRepoArgs),
}

#[derive(Args)]
#[command(group(ArgGroup::new("mode").required(true).args(["dry_run", "apply"])))]
struct ScaffoldRepoArgs {
    /// Preview without writing.
    #[arg(long)]
    dry_run: bool,
    /// Apply scaffold changes.
    #[arg(long)]
    apply: bool,
    /// Replace integration files reported as conflicts.
    #[arg(long, requires = "apply")]
    replace_conflicts: bool,
}

fn main() -> ExitCode {
    let cli = match parse_cli_arguments(env::args_os()) {
        Ok(cli) => cli,
        Err(error) => {
            let _ = error.print();
            return ExitCode::from(error.exit_code() as u8);
        }
    };
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn parse_cli_arguments(arguments: impl IntoIterator<Item = OsString>) -> Result<Cli, clap::Error> {
    let arguments = arguments.into_iter().collect::<Vec<_>>();
    if arguments.get(1).is_some_and(|value| value == "update")
        && !has_standalone_help(&arguments, is_bare_update_value_option)
    {
        if let Some(pair) = arguments
            .windows(2)
            .find(|pair| is_bare_update_value_option(&pair[0]) && is_update_option_token(&pair[1]))
        {
            let option = pair[0].to_string_lossy();
            let value = pair[1].to_string_lossy();
            return Err(command_error(
                &["update"],
                "tiber update",
                ErrorKind::InvalidValue,
                &format!("{option} requires a value; use {option}={value} for that literal value"),
            ));
        }
    }
    if arguments.get(1).is_some_and(|value| value == "subtask")
        && arguments.get(2).is_some_and(|value| value == "add")
        && arguments.len() >= 6
        && arguments
            .last()
            .is_some_and(|value| value == "--after" || value == "--after=")
    {
        return Err(command_error(
            &["subtask", "add"],
            "tiber subtask add",
            ErrorKind::InvalidValue,
            "--after requires a comma-separated predecessor value",
        ));
    }
    if arguments.get(1).is_some_and(|value| value == "install-bin")
        && !has_standalone_help(&arguments, is_bare_install_bin_value_option)
    {
        if let Some(pair) = arguments.windows(2).find(|pair| {
            is_bare_install_bin_value_option(&pair[0]) && is_install_bin_option_token(&pair[1])
        }) {
            let value = pair[1].to_string_lossy();
            return Err(command_error(
                &["install-bin"],
                "tiber install-bin",
                ErrorKind::InvalidValue,
                &format!(
                    "--target-dir requires a value; use --target-dir={value} for that literal path"
                ),
            ));
        }
    }

    Cli::try_parse_from(normalized_cli_arguments(arguments))
}

fn has_standalone_help(
    arguments: &[OsString],
    is_bare_value_option: fn(&OsString) -> bool,
) -> bool {
    arguments.iter().enumerate().any(|(index, value)| {
        is_help_request(value)
            && (index == 0 || !is_bare_value_option(&arguments[index.saturating_sub(1)]))
    })
}

fn is_help_request(value: &OsString) -> bool {
    value
        .to_str()
        .is_some_and(|value| value == "--help" || value.starts_with("-h"))
}

fn is_bare_update_value_option(value: &OsString) -> bool {
    value.to_str().is_some_and(is_update_value_option_name)
}

fn is_bare_install_bin_value_option(value: &OsString) -> bool {
    value == "--target-dir"
}

fn is_update_option_token(value: &OsString) -> bool {
    let Some(value) = value.to_str() else {
        return false;
    };
    if value.starts_with("-h") {
        return true;
    }
    let option = value.split_once('=').map_or(value, |(option, _)| option);
    option == "--help" || is_update_value_option_name(option)
}

fn is_update_value_option_name(option: &str) -> bool {
    matches!(
        option,
        "--title" | "--summary" | "--context" | "--tags" | "--pr-mr-url" | "--pr-mr-status"
    )
}

fn is_install_bin_option_token(value: &OsString) -> bool {
    let Some(value) = value.to_str() else {
        return false;
    };
    if value.starts_with("-h") {
        return true;
    }
    let option = value.split_once('=').map_or(value, |(option, _)| option);
    matches!(option, "--target-dir" | "--dry-run" | "--apply" | "--help")
}

fn command_error(
    path: &[&str],
    bin_name: &'static str,
    kind: ErrorKind,
    message: &str,
) -> clap::Error {
    let mut root = Cli::command();
    let mut command = &mut root;
    for segment in path {
        command = command
            .find_subcommand_mut(segment)
            .expect("parser error path is part of the CLI grammar");
    }
    let mut command = command.clone().bin_name(bin_name);
    command.error(kind, message)
}

fn normalized_cli_arguments(arguments: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
    let mut arguments = arguments.into_iter().collect::<Vec<_>>();

    // The legacy grammar always treated the first token after the task ref as
    // the title, even when it looked like the command-local --after option.
    if arguments.get(1).is_some_and(|value| value == "subtask")
        && arguments.get(2).is_some_and(|value| value == "add")
        && arguments
            .get(4)
            .and_then(|value| value.to_str())
            .is_some_and(|value| value == "--after" || value.starts_with("--after="))
    {
        let title = arguments[4].to_string_lossy();
        arguments[4] = OsString::from(format!("{ESCAPED_SUBTASK_TITLE_PREFIX}{title}"));
    }

    // Clap intentionally discards whether an option value used `=`. Retain
    // that distinction so mode-looking paths stay available only explicitly.
    if arguments.get(1).is_some_and(|value| value == "install-bin") {
        for argument in arguments.iter_mut().skip(2) {
            let Some(value) = argument
                .to_str()
                .and_then(|value| value.strip_prefix("--target-dir="))
            else {
                continue;
            };
            if matches!(value, "--dry-run" | "--apply") {
                *argument = OsString::from(format!(
                    "--target-dir={EXPLICIT_INSTALL_TARGET_PREFIX}{value}"
                ));
            }
        }
    }

    arguments
}

fn run(cli: Cli) -> Result<(), tiber_git::Error> {
    match cli.command {
        Command::Init => tiber_git::init_repository(),
        Command::Sync => tiber_git::sync_repository(),
        Command::CodexSandbox(_) => {
            print!("{}", tiber_mcp::codex_sandbox_setup());
            Ok(())
        }
        Command::Dashboard(DashboardArgs {
            command: DashboardCommand::Serve,
        }) => {
            let runtime = tokio::runtime::Runtime::new().map_err(tiber_git::Error::Io)?;
            runtime.block_on(async {
                let port = env::var("TIBER_DASHBOARD_PORT")
                    .ok()
                    .and_then(|value| value.parse::<u16>().ok())
                    .unwrap_or(0);
                let addr = format!("127.0.0.1:{port}");
                let listener = tokio::net::TcpListener::bind(&addr)
                    .await
                    .map_err(tiber_git::Error::Io)?;
                let addr = listener.local_addr().map_err(tiber_git::Error::Io)?;
                println!("tiber dashboard listening on http://{addr}");
                tiber_server::serve(listener).await
            })
        }
        Command::Mcp(McpArgs {
            transport: McpTransport::Stdio,
        }) => {
            let stdin = std::io::stdin();
            let stdout = std::io::stdout();
            tiber_mcp::run_stdio(stdin.lock(), stdout.lock())
        }
        Command::InstallBin(InstallBinArgs {
            target_dir, apply, ..
        }) => {
            let target_dir = target_dir.into_value();
            let installed = tiber_git::install_bin(&target_dir, apply)?;
            if apply {
                println!("installed {installed}");
            } else {
                println!("would install {installed}");
            }
            Ok(())
        }
        Command::Create { title } => {
            let created = tiber_git::create_task(&title)?;
            println!("created {}", created.path);
            Ok(())
        }
        Command::Show(TaskRefArgs { task_ref }) => {
            print!("{}", tiber_git::show_task(&task_ref)?);
            Ok(())
        }
        Command::Metadata(TaskRefArgs { task_ref }) => {
            let metadata = tiber_git::task_metadata(&task_ref)?;
            println!(
                "{}\t{}\tcommitted_at={}",
                metadata.path,
                metadata.title,
                metadata
                    .committed_at
                    .unwrap_or_else(|| "uncommitted".to_string())
            );
            Ok(())
        }
        Command::List => {
            for task in tiber_git::list_tasks()? {
                println!("{}\t{}", task.path, task.title);
            }
            Ok(())
        }
        Command::Next => {
            if let Some(task) = tiber_git::next_task()? {
                println!("{}\t{}", task.path, task.title);
            }
            Ok(())
        }
        Command::Transition(TransitionArgs { task_ref, status }) => {
            tiber_git::transition_task(&task_ref, &status)?;
            Ok(())
        }
        Command::Prioritize(PrioritizeArgs { task_ref, before }) => {
            tiber_git::prioritize_before(&task_ref, &before)?;
            Ok(())
        }
        Command::Link(RelationArgs {
            from_ref,
            relation: RelationCommand::Blocks { to_ref },
        }) => {
            tiber_git::link_blocks(&from_ref, &to_ref)?;
            Ok(())
        }
        Command::Unlink(RelationArgs {
            from_ref,
            relation: RelationCommand::Blocks { to_ref },
        }) => {
            tiber_git::unlink_blocks(&from_ref, &to_ref)?;
            Ok(())
        }
        Command::Subtask(SubtaskArgs {
            command:
                SubtaskCommand::Add(SubtaskAddArgs {
                    task_ref,
                    title,
                    after,
                }),
        }) => {
            let title = title.into_value();
            let after = after
                .map(CommaSeparatedValues::into_values)
                .unwrap_or_default();
            tiber_git::add_subtask(&task_ref, &title, &after)?;
            Ok(())
        }
        Command::Subtask(SubtaskArgs {
            command: SubtaskCommand::Check { task_ref, index },
        }) => {
            tiber_git::set_subtask_checked(&task_ref, &index, true)?;
            Ok(())
        }
        Command::Subtask(SubtaskArgs {
            command: SubtaskCommand::Uncheck { task_ref, index },
        }) => {
            tiber_git::set_subtask_checked(&task_ref, &index, false)?;
            Ok(())
        }
        Command::Update(update) => {
            tiber_git::update_task(
                &update.task_ref,
                tiber_git::TaskUpdate {
                    title: update.title.as_deref(),
                    summary: update.summary.as_deref(),
                    context: update.context.as_deref(),
                    tags: update.tags.map(CommaSeparatedValues::into_values),
                    pr_mr_url: update.pr_mr_url.as_deref(),
                    pr_mr_status: update.pr_mr_status.as_deref(),
                },
            )?;
            Ok(())
        }
        Command::Acceptance(AcceptanceArgs {
            command:
                AcceptanceCommand::Add {
                    task_ref,
                    criterion,
                },
        }) => {
            tiber_git::add_acceptance(&task_ref, &criterion)?;
            Ok(())
        }
        Command::Acceptance(AcceptanceArgs {
            command: AcceptanceCommand::Check { task_ref, index },
        }) => {
            tiber_git::set_acceptance_checked(&task_ref, &index, true)?;
            Ok(())
        }
        Command::Acceptance(AcceptanceArgs {
            command: AcceptanceCommand::Uncheck { task_ref, index },
        }) => {
            tiber_git::set_acceptance_checked(&task_ref, &index, false)?;
            Ok(())
        }
        Command::Acceptance(AcceptanceArgs {
            command: AcceptanceCommand::Remove { task_ref, index },
        }) => {
            tiber_git::remove_acceptance(&task_ref, &index)?;
            Ok(())
        }
        Command::Note(NoteArgs {
            command: NoteCommand::Add { task_ref, note },
        }) => {
            tiber_git::add_note(&task_ref, &note)?;
            Ok(())
        }
        Command::Validate(_) => {
            for message in tiber_git::validate_fix()? {
                println!("{message}");
            }
            Ok(())
        }
        Command::CloseFromTrailers => {
            for closed in tiber_git::close_from_trailers()? {
                println!("closed {closed}");
            }
            Ok(())
        }
        Command::Scaffold(ScaffoldArgs {
            command:
                ScaffoldCommand::Repo(ScaffoldRepoArgs {
                    apply,
                    replace_conflicts,
                    ..
                }),
        }) => {
            for message in tiber_git::scaffold_repo(apply, replace_conflicts)? {
                println!("{message}");
            }
            Ok(())
        }
    }
}
