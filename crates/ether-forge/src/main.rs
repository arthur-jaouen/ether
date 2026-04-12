use std::path::PathBuf;

use clap::{Parser, Subcommand};

// FIXME(T6): drop `allow(dead_code)` once a subcommand wires Task::load_all in.
#[allow(dead_code)]
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
        Some(Command::Done {
            id,
            commit,
            backlog_dir,
        }) => cmd::done::run(&backlog_dir, &id, commit.as_deref()),
    }
}
