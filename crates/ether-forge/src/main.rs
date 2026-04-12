use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod task;

mod cmd;
mod frontmatter;
mod repo;
mod roadmap;

/// Default `--backlog` path: `<repo_root>/backlog`, falling back to the
/// literal `backlog` relative to cwd if `git rev-parse` fails.
fn default_backlog_dir() -> PathBuf {
    repo::repo_root()
        .map(|r| r.join("backlog"))
        .unwrap_or_else(|_| PathBuf::from("backlog"))
}

/// Default `--roadmap` path: `<repo_root>/ROADMAP.md`, falling back to the
/// literal `ROADMAP.md` relative to cwd if `git rev-parse` fails.
fn default_roadmap() -> PathBuf {
    repo::repo_root()
        .map(|r| r.join("ROADMAP.md"))
        .unwrap_or_else(|_| PathBuf::from("ROADMAP.md"))
}

/// Ether development process CLI — backlog management and workflow automation.
#[derive(Parser, Debug)]
#[command(name = "ether-forge", about, version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run the workspace verification suite (test, clippy, fmt).
    Check,
    /// Validate backlog integrity (ids, depends_on, cycles, filenames).
    Validate {
        /// Backlog directory (defaults to `./backlog`).
        #[arg(long, default_value_os_t = default_backlog_dir())]
        backlog_dir: PathBuf,
    },
    /// List backlog tasks sorted by priority then id.
    List {
        /// Filter by status (draft|ready|blocked|done).
        #[arg(long)]
        status: Option<String>,
        /// Backlog directory (defaults to `./backlog`).
        #[arg(long, default_value_os_t = default_backlog_dir())]
        backlog_dir: PathBuf,
    },
    /// Print the next ready task (priority then id).
    Next {
        /// Backlog directory (defaults to `./backlog`).
        #[arg(long, default_value_os_t = default_backlog_dir())]
        backlog_dir: PathBuf,
    },
    /// Print a task's full contents (active or done).
    Get {
        /// Task id (e.g. `T8`).
        id: String,
        /// Backlog directory (defaults to `./backlog`).
        #[arg(long, default_value_os_t = default_backlog_dir())]
        backlog_dir: PathBuf,
    },
    /// Print a task file, optionally appending its linked ROADMAP section.
    Task {
        /// Task id (e.g. `T21`).
        id: String,
        /// Also emit the matching ROADMAP section as one blob.
        #[arg(long)]
        context: bool,
        /// Backlog directory (defaults to `./backlog`).
        #[arg(long, default_value_os_t = default_backlog_dir())]
        backlog_dir: PathBuf,
        /// Path to ROADMAP.md (defaults to `./ROADMAP.md`).
        #[arg(long, default_value_os_t = default_roadmap())]
        roadmap: PathBuf,
    },
    /// Case-insensitive substring match on id, title, and body.
    Search {
        /// Search query.
        query: String,
        /// Backlog directory (defaults to `./backlog`).
        #[arg(long, default_value_os_t = default_backlog_dir())]
        backlog_dir: PathBuf,
    },
    /// Show upward and downward dependencies for a task.
    Deps {
        /// Task id (e.g. `T8`).
        id: String,
        /// Backlog directory (defaults to `./backlog`).
        #[arg(long, default_value_os_t = default_backlog_dir())]
        backlog_dir: PathBuf,
    },
    /// Compact backlog summary — counts by status and next ready task.
    Status {
        /// Backlog directory (defaults to `./backlog`).
        #[arg(long, default_value_os_t = default_backlog_dir())]
        backlog_dir: PathBuf,
    },
    /// Print a review-scoped `git diff main` (strips lockfiles, caps size).
    Diff {
        /// Task id (optional). If given, runs inside that task's worktree.
        id: Option<String>,
        /// Backlog directory (defaults to `./backlog`).
        #[arg(long, default_value_os_t = default_backlog_dir())]
        backlog_dir: PathBuf,
    },
    /// Create a worktree and branch for a task.
    Worktree {
        /// Task id (e.g. `T9`).
        id: String,
        /// Backlog directory (defaults to `./backlog`).
        #[arg(long, default_value_os_t = default_backlog_dir())]
        backlog_dir: PathBuf,
    },
    /// Run `check`, then `git commit` with the task's title as the message.
    Commit {
        /// Task id (e.g. `T9`).
        id: String,
        /// Backlog directory (defaults to `./backlog`).
        #[arg(long, default_value_os_t = default_backlog_dir())]
        backlog_dir: PathBuf,
        /// Bypass the reviewer-blocker gate. Appends a `Reviewed-by-force`
        /// trailer so the override is recorded in the commit message.
        #[arg(long)]
        force_review: bool,
        /// Extra args forwarded to `git commit` (e.g. `-a`, additional `-m`).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        extra: Vec<String>,
    },
    /// Mark a task done and cascade dependency updates across the backlog.
    Done {
        /// Task id (e.g. `T8`).
        id: String,
        /// Commit sha to record in the task's frontmatter.
        #[arg(long)]
        commit: Option<String>,
        /// Backlog directory (defaults to `./backlog`).
        #[arg(long, default_value_os_t = default_backlog_dir())]
        backlog_dir: PathBuf,
    },
    /// Audit coverage vs ROADMAP, lint backlog, flag drift. Dry-run by default.
    Groom {
        /// Backlog directory (defaults to `./backlog`).
        #[arg(long, default_value_os_t = default_backlog_dir())]
        backlog_dir: PathBuf,
        /// Path to ROADMAP.md (defaults to `./ROADMAP.md`).
        #[arg(long, default_value_os_t = default_roadmap())]
        roadmap: PathBuf,
        /// Apply cascade fix-ups to the backlog (default is dry-run reporting).
        #[arg(long)]
        apply: bool,
        /// Emit a JSON report instead of human-readable output.
        #[arg(long)]
        json: bool,
    },
    /// List shared test helpers under `crates/*/tests/common/mod.rs`.
    ///
    /// Used by the review subagent to check for duplicated test fixtures
    /// across crates — each entry is `<crate>\t<fn_name>`, with
    /// `[DUPLICATE]` appended whenever the same helper name appears in more
    /// than one crate.
    Helpers {
        /// Crates directory (defaults to `./crates`).
        #[arg(long, default_value = "crates")]
        crates_dir: PathBuf,
    },
    /// Run a named ripgrep recipe from `.claude/rules/grep/`.
    Grep {
        /// Recipe name (file stem under `.claude/rules/grep/`).
        recipe: Option<String>,
        /// List available recipes instead of running one.
        #[arg(long)]
        list: bool,
    },
    /// Structural search via `ast-grep`.
    Find {
        /// ast-grep pattern (e.g. `$X.unwrap()`). Omit when using `--rule`.
        pattern: Option<String>,
        /// Language to parse (`ast-grep --lang`). Defaults to `rust`.
        #[arg(long, default_value = "rust")]
        lang: String,
        /// Resolve a rule file from `.claude/rules/sg/<name>.yml`.
        #[arg(long)]
        rule: Option<String>,
        /// Path to search (defaults to `ast-grep`'s default of CWD).
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Structural rewrite via `ast-grep` (applies edits in place).
    Rewrite {
        /// ast-grep pattern to match.
        pattern: String,
        /// Replacement pattern.
        #[arg(long = "to")]
        to: String,
        /// Language to parse (`ast-grep --lang`). Defaults to `rust`.
        #[arg(long, default_value = "rust")]
        lang: String,
        /// Path to rewrite (defaults to `ast-grep`'s default of CWD).
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Verify workspace is safe to create a skill worktree (clean main, fresh base, no stale claim).
    Preflight {
        /// Task id (optional). When set, also refuses if a branch already claims it.
        #[arg(long)]
        task: Option<String>,
        /// Backlog directory (defaults to `./backlog`).
        #[arg(long, default_value_os_t = default_backlog_dir())]
        backlog_dir: PathBuf,
    },
    /// Install the pre-commit git hook that runs `ether-forge check`.
    InstallHooks {
        /// Repository root (defaults to the current directory).
        #[arg(long, default_value = ".")]
        repo_root: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        None => {
            println!("ether-forge: no subcommand given. See --help.");
            Ok(())
        }
        Some(Command::Check) => cmd::check::run(),
        Some(Command::Validate { backlog_dir }) => cmd::validate::run(&backlog_dir),
        Some(Command::List {
            status,
            backlog_dir,
        }) => cmd::list::run(&backlog_dir, status.as_deref()),
        Some(Command::Next { backlog_dir }) => cmd::next::run(&backlog_dir),
        Some(Command::Get { id, backlog_dir }) => cmd::get::run(&backlog_dir, &id),
        Some(Command::Task {
            id,
            context,
            backlog_dir,
            roadmap,
        }) => cmd::task::run(&backlog_dir, &roadmap, &id, context),
        Some(Command::Search { query, backlog_dir }) => cmd::search::run(&backlog_dir, &query),
        Some(Command::Deps { id, backlog_dir }) => cmd::deps::run(&backlog_dir, &id),
        Some(Command::Status { backlog_dir }) => cmd::status::run(&backlog_dir),
        Some(Command::Diff { id, backlog_dir }) => cmd::diff::run(&backlog_dir, id.as_deref()),
        Some(Command::Worktree { id, backlog_dir }) => cmd::worktree::run(&backlog_dir, &id),
        Some(Command::Commit {
            id,
            backlog_dir,
            force_review,
            extra,
        }) => cmd::commit::run(&backlog_dir, &id, &extra, force_review),
        Some(Command::Done {
            id,
            commit,
            backlog_dir,
        }) => cmd::done::run(&backlog_dir, &id, commit.as_deref()),
        Some(Command::Groom {
            backlog_dir,
            roadmap,
            apply,
            json,
        }) => cmd::groom::run(&backlog_dir, &roadmap, apply, json),
        Some(Command::Helpers { crates_dir }) => cmd::helpers::run(&crates_dir),
        Some(Command::Grep { recipe, list }) => cmd::grep::run(recipe.as_deref(), list),
        Some(Command::Find {
            pattern,
            lang,
            rule,
            path,
        }) => cmd::find::run(pattern.as_deref(), &lang, rule.as_deref(), path.as_deref()),
        Some(Command::Rewrite {
            pattern,
            to,
            lang,
            path,
        }) => cmd::rewrite::run(&pattern, &to, &lang, path.as_deref()),
        Some(Command::Preflight { task, backlog_dir }) => {
            cmd::preflight::run(&backlog_dir, task.as_deref())
        }
        Some(Command::InstallHooks { repo_root }) => cmd::install_hooks::run(&repo_root),
    }
}
