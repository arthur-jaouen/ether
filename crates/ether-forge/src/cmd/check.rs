//! `ether-forge check` — run the workspace verification suite.
//!
//! Executes `cargo test`, `cargo clippy`, and `cargo fmt` in sequence against
//! the whole workspace, streaming each child's stdio to the parent. Bails on
//! the first failure so subsequent checks are skipped.

use std::process::{Command, ExitStatus, Stdio};

use anyhow::{bail, Context, Result};

/// The fixed verification sequence. Exposed for tests that assert command
/// assembly without spawning real `cargo` processes.
pub fn commands() -> Vec<Vec<&'static str>> {
    vec![
        vec!["cargo", "test", "--workspace"],
        vec!["cargo", "clippy", "--workspace", "--", "-D", "warnings"],
        vec!["cargo", "fmt", "--all", "--", "--check"],
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
    let status = Command::new(program)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;

    #[test]
    fn commands_assembled_in_order() {
        let cmds = commands();
        assert_eq!(cmds.len(), 3);
        assert_eq!(cmds[0], ["cargo", "test", "--workspace"]);
        assert_eq!(
            cmds[1],
            ["cargo", "clippy", "--workspace", "--", "-D", "warnings"]
        );
        assert_eq!(cmds[2], ["cargo", "fmt", "--all", "--", "--check"]);
    }

    #[test]
    fn execute_runs_every_command_on_success() {
        let cmds = commands();
        let mut seen: Vec<Vec<String>> = Vec::new();
        let mut runner = |argv: &[&str]| {
            seen.push(argv.iter().map(|s| s.to_string()).collect());
            Ok(ExitStatus::from_raw(0))
        };
        execute(&cmds, &mut runner).unwrap();
        assert_eq!(seen.len(), 3);
        assert_eq!(seen[0][1], "test");
        assert_eq!(seen[1][1], "clippy");
        assert_eq!(seen[2][1], "fmt");
    }

    #[test]
    fn execute_stops_on_first_failure() {
        let cmds = commands();
        let mut seen: Vec<String> = Vec::new();
        let mut runner = |argv: &[&str]| {
            seen.push(argv[1].to_string());
            // Fail the clippy step (index 1).
            let code = if argv[1] == "clippy" { 1 } else { 0 };
            Ok(ExitStatus::from_raw(code << 8))
        };
        let err = execute(&cmds, &mut runner).unwrap_err();
        assert_eq!(seen, vec!["test", "clippy"]);
        assert!(err.to_string().contains("clippy"));
    }

    #[test]
    fn execute_propagates_spawn_error() {
        let cmds = commands();
        let mut runner = |_argv: &[&str]| Err(anyhow::anyhow!("boom"));
        let err = execute(&cmds, &mut runner).unwrap_err();
        assert!(format!("{err:#}").contains("cargo test"));
    }
}
