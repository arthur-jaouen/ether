//! `ether-forge check` — run the workspace verification suite.
//!
//! Runs clippy, then `cargo nextest` for unit/integration tests, then
//! `cargo test --doc` to cover the doctest gap nextest leaves. Each child's
//! stdio streams to the parent and the sequence aborts on the first failure.
//!
//! Requires `cargo-nextest` to be installed (`cargo install cargo-nextest`).

use std::process::{Command, ExitStatus, Stdio};

use anyhow::{bail, Context, Result};

/// Environment variables applied to every spawned cargo command.
pub const CARGO_ENV: &[(&str, &str)] = &[("CARGO_TERM_COLOR", "never")];

/// The fixed verification sequence. Exposed for tests that assert command
/// assembly without spawning real `cargo` processes.
pub fn commands() -> Vec<Vec<&'static str>> {
    vec![
        vec![
            "cargo",
            "clippy",
            "--workspace",
            "--all-targets",
            "--message-format=short",
            "-q",
            "--",
            "-D",
            "warnings",
        ],
        vec![
            "cargo",
            "nextest",
            "run",
            "--workspace",
            "--failure-output=final",
            "--status-level=fail",
            "--hide-progress-bar",
        ],
        vec!["cargo", "test", "--doc", "--workspace"],
    ]
}

/// Run the verification suite with the real `cargo` binary.
pub fn run() -> Result<()> {
    execute(&commands(), &mut spawn_real)
}

/// Execute the command sequence with an injected runner. Stops on the first
/// non-zero exit status and reports which command failed.
pub(crate) fn execute<R>(commands: &[Vec<&str>], runner: &mut R) -> Result<()>
where
    R: FnMut(&[&str]) -> Result<ExitStatus>,
{
    for argv in commands {
        let status = runner(argv).with_context(|| format!("spawning `{}`", argv.join(" ")))?;
        if !status.success() {
            bail!("`{}` failed with {}", argv.join(" "), status);
        }
    }
    Ok(())
}

fn spawn_real(argv: &[&str]) -> Result<ExitStatus> {
    let (program, args) = argv
        .split_first()
        .ok_or_else(|| anyhow::anyhow!("empty command"))?;
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    for (k, v) in CARGO_ENV {
        cmd.env(k, v);
    }
    Ok(cmd.status()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;

    #[test]
    fn commands_assembled_in_order() {
        let cmds = commands();
        assert_eq!(cmds.len(), 3);
        assert_eq!(cmds[0][0..2], ["cargo", "clippy"]);
        assert!(cmds[0].contains(&"--all-targets"));
        assert!(cmds[0].contains(&"--message-format=short"));
        assert!(cmds[0].contains(&"-D"));
        assert!(cmds[0].contains(&"warnings"));
        assert_eq!(cmds[1][0..3], ["cargo", "nextest", "run"]);
        assert!(cmds[1].contains(&"--failure-output=final"));
        assert!(cmds[1].contains(&"--status-level=fail"));
        assert!(cmds[1].contains(&"--hide-progress-bar"));
        assert_eq!(cmds[2], ["cargo", "test", "--doc", "--workspace"]);
    }

    #[test]
    fn cargo_env_forces_no_color() {
        assert!(CARGO_ENV
            .iter()
            .any(|(k, v)| *k == "CARGO_TERM_COLOR" && *v == "never"));
    }

    #[test]
    fn execute_runs_every_command_on_success() {
        let cmds = commands();
        let mut seen: Vec<String> = Vec::new();
        let mut runner = |argv: &[&str]| {
            seen.push(argv[1].to_string());
            Ok(ExitStatus::from_raw(0))
        };
        execute(&cmds, &mut runner).unwrap();
        assert_eq!(seen, vec!["clippy", "nextest", "test"]);
    }

    #[test]
    fn execute_stops_on_first_failure() {
        let cmds = commands();
        let mut seen: Vec<String> = Vec::new();
        let mut runner = |argv: &[&str]| {
            seen.push(argv[1].to_string());
            let code = if argv[1] == "nextest" { 1 } else { 0 };
            Ok(ExitStatus::from_raw(code << 8))
        };
        let err = execute(&cmds, &mut runner).unwrap_err();
        assert_eq!(seen, vec!["clippy", "nextest"]);
        assert!(err.to_string().contains("nextest"));
    }

    #[test]
    fn execute_propagates_spawn_error() {
        let cmds = commands();
        let mut runner = |_argv: &[&str]| Err(anyhow::anyhow!("boom"));
        let err = execute(&cmds, &mut runner).unwrap_err();
        assert!(format!("{err:#}").contains("cargo clippy"));
    }
}
