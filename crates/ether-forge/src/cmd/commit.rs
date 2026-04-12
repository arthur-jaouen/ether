//! `ether-forge commit T<n>` — run `check`, then `git commit` with a
//! task-derived message.
//!
//! The commit message is `T<n>: <title>` pulled from the task's frontmatter.
//! Extra positional args after the id are forwarded to `git commit` verbatim,
//! so callers can pass `-a`, `-s`, additional `-m` lines, etc.

use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};

use anyhow::{anyhow, bail, Context, Result};

use crate::cmd::check;
use crate::cmd::worktree::find_task;

/// Assemble the `git commit` argv with a message and extra passthrough args.
pub(crate) fn commit_argv<'a>(message: &'a str, extra: &'a [String]) -> Vec<&'a str> {
    let mut argv: Vec<&str> = vec!["git", "commit", "-m", message];
    for a in extra {
        argv.push(a.as_str());
    }
    argv
}

/// Run the commit subcommand against the real binaries.
pub fn run(backlog_dir: &Path, id: &str, extra: &[String]) -> Result<()> {
    check::run().context("ether-forge check failed — commit aborted")?;
    let task = find_task(backlog_dir, id)?;
    let message = format!("{}: {}", task.id, task.title);
    let argv = commit_argv(&message, extra);
    let status = spawn_real(&argv)?;
    if !status.success() {
        bail!("`{}` failed with {}", argv.join(" "), status);
    }
    Ok(())
}

fn spawn_real(argv: &[&str]) -> Result<ExitStatus> {
    let (program, args) = argv.split_first().ok_or_else(|| anyhow!("empty command"))?;
    let status = Command::new(program)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("spawning `{}`", argv.join(" ")))?;
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_argv_basic_message() {
        let extra: Vec<String> = Vec::new();
        let argv = commit_argv("T9: title here", &extra);
        assert_eq!(argv, vec!["git", "commit", "-m", "T9: title here"]);
    }

    #[test]
    fn commit_argv_forwards_extra_args() {
        let extra = vec!["-a".to_string(), "-m".to_string(), "more".to_string()];
        let argv = commit_argv("T9: x", &extra);
        assert_eq!(
            argv,
            vec!["git", "commit", "-m", "T9: x", "-a", "-m", "more"]
        );
    }
}
