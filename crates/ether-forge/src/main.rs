use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod task;

mod cmd;
mod frontmatter;

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
        #[arg(long, default_value = "backlog")]
        backlog_dir: PathBuf,
    },
    /// List backlog tasks sorted by priority then id.
    List {
        /// Filter by status (draft|ready|blocked|done).
        #[arg(long)]
        status: Option<String>,
        /// Backlog directory (defaults to `./backlog`).
        #[arg(long, default_value = "backlog")]
        backlog_dir: PathBuf,
    },
    /// Print the next ready task (priority then id).
    Next {
        /// Backlog directory (defaults to `./backlog`).
        #[arg(long, default_value = "backlog")]
        backlog_dir: PathBuf,
    },
    /// Print a task's full contents (active or done).
    Get {
        /// Task id (e.g. `T8`).
        id: String,
        /// Backlog directory (defaults to `./backlog`).
        #[arg(long, default_value = "backlog")]
        backlog_dir: PathBuf,
    },
    /// Case-insensitive substring match on id, title, and body.
    Search {
        /// Search query.
        query: String,
        /// Backlog directory (defaults to `./backlog`).
        #[arg(long, default_value = "backlog")]
        backlog_dir: PathBuf,
    },
    /// Show upward and downward dependencies for a task.
    Deps {
        /// Task id (e.g. `T8`).
        id: String,
        /// Backlog directory (defaults to `./backlog`).
        #[arg(long, default_value = "backlog")]
        backlog_dir: PathBuf,
    },
    /// Compact backlog summary — counts by status and next ready task.
    Status {
        /// Backlog directory (defaults to `./backlog`).
        #[arg(long, default_value = "backlog")]
        backlog_dir: PathBuf,
    },
    /// Mark a task done and cascade dependency updates across the backlog.
    Done {
        /// Task id (e.g. `T8`).
        id: String,
        /// Commit sha to record in the task's frontmatter.
        #[arg(long)]
        commit: Option<String>,
        /// Backlog directory (defaults to `./backlog`).
        #[arg(long, default_value = "backlog")]
        backlog_dir: PathBuf,
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
        Some(Command::Search { query, backlog_dir }) => cmd::search::run(&backlog_dir, &query),
        Some(Command::Deps { id, backlog_dir }) => cmd::deps::run(&backlog_dir, &id),
        Some(Command::Status { backlog_dir }) => cmd::status::run(&backlog_dir),
        Some(Command::Done {
            id,
            commit,
            backlog_dir,
        }) => cmd::done::run(&backlog_dir, &id, commit.as_deref()),
    }
}
