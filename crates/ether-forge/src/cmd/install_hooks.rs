//! `ether-forge install-hooks` — install the pre-commit git hook that runs
//! `ether-forge check`. Idempotent: detects the marker line and replaces an
//! existing ether-forge managed hook in place.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

/// Marker line embedded in hooks we manage. Presence means the hook belongs
/// to ether-forge and may be overwritten on reinstall.
pub const MARKER: &str = "# ether-forge:install-hooks";

/// The pre-commit hook script body written by `install-hooks`.
pub fn pre_commit_script() -> String {
    format!(
        "#!/bin/sh\n\
         {MARKER}\n\
         exec ether-forge check\n"
    )
}

/// Install the pre-commit hook into the repository at `repo_root`.
pub fn run(repo_root: &Path) -> Result<()> {
    let hooks_dir = resolve_hooks_dir(repo_root)?;
    fs::create_dir_all(&hooks_dir)
        .with_context(|| format!("creating hooks dir {}", hooks_dir.display()))?;
    let hook_path = hooks_dir.join("pre-commit");
    install_pre_commit(&hook_path)?;
    println!("installed pre-commit hook at {}", hook_path.display());
    Ok(())
}

/// Resolve the hooks directory for `repo_root`. Returns `<repo>/.git/hooks`
/// for a standard checkout. Fails if `.git` is missing or is a worktree
/// gitdir file (we refuse to install into a linked worktree to avoid
/// polluting the main checkout's hooks from a transient worktree).
pub fn resolve_hooks_dir(repo_root: &Path) -> Result<PathBuf> {
    let git = repo_root.join(".git");
    let meta = fs::metadata(&git)
        .with_context(|| format!("{} is not a git repository", repo_root.display()))?;
    if !meta.is_dir() {
        bail!(
            "{} is a linked worktree (.git is a file); run install-hooks from the main checkout",
            repo_root.display()
        );
    }
    Ok(git.join("hooks"))
}

/// Write the pre-commit hook at `hook_path`, replacing an existing
/// ether-forge managed hook or refusing to clobber a foreign one.
pub fn install_pre_commit(hook_path: &Path) -> Result<()> {
    if hook_path.exists() {
        let existing = fs::read_to_string(hook_path)
            .with_context(|| format!("reading {}", hook_path.display()))?;
        if !existing.contains(MARKER) {
            bail!(
                "{} exists and is not managed by ether-forge (missing marker `{}`); \
                 move it aside and retry",
                hook_path.display(),
                MARKER
            );
        }
    }
    fs::write(hook_path, pre_commit_script())
        .with_context(|| format!("writing {}", hook_path.display()))?;
    set_executable(hook_path)?;
    Ok(())
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).with_context(|| format!("chmod +x {}", path.display()))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_contains_marker_and_check_invocation() {
        let s = pre_commit_script();
        assert!(s.starts_with("#!/bin/sh"));
        assert!(s.contains(MARKER));
        assert!(s.contains("ether-forge check"));
    }

    #[test]
    fn resolve_hooks_dir_rejects_missing_git() {
        let tmp = tempfile::tempdir().unwrap();
        let err = resolve_hooks_dir(tmp.path()).unwrap_err();
        assert!(format!("{err:#}").contains("not a git repository"));
    }

    #[test]
    fn resolve_hooks_dir_rejects_linked_worktree() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join(".git"), "gitdir: /somewhere\n").unwrap();
        let err = resolve_hooks_dir(tmp.path()).unwrap_err();
        assert!(format!("{err:#}").contains("linked worktree"));
    }

    #[test]
    fn install_writes_fresh_hook() {
        let tmp = tempfile::tempdir().unwrap();
        let hook = tmp.path().join("pre-commit");
        install_pre_commit(&hook).unwrap();
        let body = fs::read_to_string(&hook).unwrap();
        assert!(body.contains(MARKER));
        assert!(body.contains("ether-forge check"));
    }

    #[test]
    fn install_replaces_managed_hook_idempotently() {
        let tmp = tempfile::tempdir().unwrap();
        let hook = tmp.path().join("pre-commit");
        fs::write(&hook, format!("#!/bin/sh\n{MARKER}\necho old\n")).unwrap();
        install_pre_commit(&hook).unwrap();
        let body = fs::read_to_string(&hook).unwrap();
        assert!(body.contains("ether-forge check"));
        assert!(!body.contains("echo old"));
        // Second install is a no-op in effect.
        install_pre_commit(&hook).unwrap();
        let again = fs::read_to_string(&hook).unwrap();
        assert_eq!(body, again);
    }

    #[test]
    fn install_refuses_to_clobber_foreign_hook() {
        let tmp = tempfile::tempdir().unwrap();
        let hook = tmp.path().join("pre-commit");
        fs::write(&hook, "#!/bin/sh\necho user hook\n").unwrap();
        let err = install_pre_commit(&hook).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("not managed by ether-forge"));
        // Original is preserved.
        let body = fs::read_to_string(&hook).unwrap();
        assert!(body.contains("user hook"));
    }

    #[cfg(unix)]
    #[test]
    fn install_sets_executable_bit() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let hook = tmp.path().join("pre-commit");
        install_pre_commit(&hook).unwrap();
        let mode = fs::metadata(&hook).unwrap().permissions().mode();
        assert_eq!(mode & 0o111, 0o111);
    }
}
