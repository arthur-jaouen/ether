//! `ether-forge grep <recipe>` — named ripgrep recipes for project-specific
//! textual searches (unsafe blocks, HashMap iteration, TODOs, dead code, …).
//!
//! Recipes live as YAML files under `.claude/rules/grep/<name>.yml`:
//!
//! ```yaml
//! name: unsafe-without-safety
//! pattern: "unsafe\\s*\\{"
//! path: crates           # optional — restricts ripgrep to this subtree
//! description: Flag unsafe blocks for manual SAFETY-comment audit
//! ```
//!
//! Output is deterministic (`rg --sort path`) so diffs between runs only
//! reflect code changes, not filesystem walk order. Requires `rg` on `PATH`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;

/// Directory (relative to CWD) where grep recipe files live.
pub const RECIPES_DIR: &str = ".claude/rules/grep";

/// A parsed recipe file — the inputs `rg` needs plus a human-readable label
/// for `--list`.
#[derive(Debug, Clone, Deserialize)]
pub struct Recipe {
    pub name: String,
    pub pattern: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Default recipes directory resolved against the process CWD. Used by `run`
/// when invoked as a top-level CLI command.
pub fn default_dir() -> PathBuf {
    PathBuf::from(RECIPES_DIR)
}

/// Resolve a recipe name to its YAML file under `dir` and parse it. Taking an
/// explicit `dir` keeps tests thread-safe — they can point at a tempdir
/// without mutating the process CWD.
pub fn load_recipe(dir: &Path, name: &str) -> Result<Recipe> {
    let path = dir.join(format!("{name}.yml"));
    if !path.exists() {
        bail!("grep recipe `{name}` not found at {}", path.display());
    }
    let raw =
        fs::read_to_string(&path).with_context(|| format!("reading recipe {}", path.display()))?;
    let recipe: Recipe =
        serde_yaml::from_str(&raw).with_context(|| format!("parsing recipe {}", path.display()))?;
    Ok(recipe)
}

/// Discover every recipe under `dir`, sorted by name. A missing directory
/// yields an empty list so `--list` can run on repos without recipes.
pub fn list_recipes(dir: &Path) -> Result<Vec<Recipe>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out: Vec<Recipe> = fs::read_dir(dir)
        .with_context(|| format!("reading recipes dir {}", dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("yml"))
        .map(|p| {
            let raw = fs::read_to_string(&p)
                .with_context(|| format!("reading recipe {}", p.display()))?;
            let recipe: Recipe = serde_yaml::from_str(&raw)
                .with_context(|| format!("parsing recipe {}", p.display()))?;
            Ok::<Recipe, anyhow::Error>(recipe)
        })
        .collect::<Result<_>>()?;
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Render the deterministic `rg` argv for a recipe. Exposed for tests so we
/// can assert flag shape without invoking ripgrep.
pub fn build_argv(recipe: &Recipe) -> Vec<String> {
    let mut argv = vec![
        "rg".to_string(),
        "--sort".to_string(),
        "path".to_string(),
        "--color".to_string(),
        "never".to_string(),
        "-n".to_string(),
        "-e".to_string(),
        recipe.pattern.clone(),
    ];
    if let Some(path) = &recipe.path {
        argv.push(path.clone());
    }
    argv
}

/// Render the `--list` output as a single string so tests can assert on it
/// directly (stdout capture during cargo test is unreliable across runners).
pub fn render_list(recipes: &[Recipe]) -> String {
    if recipes.is_empty() {
        return format!("No recipes found in {RECIPES_DIR}.\n");
    }
    let mut out = String::new();
    for r in recipes {
        let desc = r.description.as_deref().unwrap_or("");
        if desc.is_empty() {
            out.push_str(&format!("{}\n", r.name));
        } else {
            out.push_str(&format!("{}  —  {}\n", r.name, desc));
        }
    }
    out
}

/// Run `ether-forge grep`. With `list`, prints the recipe catalogue; with a
/// recipe name, shells out to `rg` using [`build_argv`].
pub fn run(recipe: Option<&str>, list: bool) -> Result<()> {
    let dir = default_dir();
    if list {
        let recipes = list_recipes(&dir)?;
        print!("{}", render_list(&recipes));
        return Ok(());
    }
    let name = recipe.ok_or_else(|| anyhow!("grep: provide a recipe name or --list"))?;
    let recipe = load_recipe(&dir, name)?;
    let argv = build_argv(&recipe);
    let status = spawn(&argv)?;
    // rg exits 1 when there are no matches — treat that as a clean "no hits"
    // rather than propagating as a hard error, so recipes can be used in CI
    // audits that expect zero findings.
    match status.code() {
        Some(0) | Some(1) => Ok(()),
        _ => bail!("`{}` exited with {}", argv.join(" "), status),
    }
}

fn spawn(argv: &[String]) -> Result<ExitStatus> {
    let (program, args) = argv.split_first().ok_or_else(|| anyhow!("empty command"))?;
    Command::new(program)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("spawning `{}` (is ripgrep installed?)", argv.join(" ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_recipe(dir: &Path, name: &str, body: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join(format!("{name}.yml")), body).unwrap();
    }

    #[test]
    fn build_argv_uses_sorted_line_numbered_rg() {
        let r = Recipe {
            name: "todo".into(),
            pattern: "TODO".into(),
            path: None,
            description: None,
        };
        let argv = build_argv(&r);
        assert_eq!(argv[0], "rg");
        assert!(argv.windows(2).any(|w| w == ["--sort", "path"]));
        assert!(argv.windows(2).any(|w| w == ["-e", "TODO"]));
        assert!(argv.contains(&"-n".to_string()));
        assert!(argv.contains(&"never".to_string()));
    }

    #[test]
    fn build_argv_appends_path_when_set() {
        let r = Recipe {
            name: "x".into(),
            pattern: "foo".into(),
            path: Some("crates/ether-core".into()),
            description: None,
        };
        let argv = build_argv(&r);
        assert_eq!(argv.last().unwrap(), "crates/ether-core");
    }

    #[test]
    fn build_argv_omits_path_when_none() {
        let r = Recipe {
            name: "x".into(),
            pattern: "foo".into(),
            path: None,
            description: None,
        };
        let argv = build_argv(&r);
        assert!(!argv.iter().any(|a| a == "crates/ether-core"));
        // Last positional is the pattern when no path is supplied.
        assert_eq!(argv.last().unwrap(), "foo");
    }

    #[test]
    fn load_recipe_parses_yaml() {
        let tmp = tempfile::tempdir().unwrap();
        write_recipe(
            tmp.path(),
            "todo",
            "name: todo\npattern: \"TODO|FIXME\"\npath: crates\ndescription: Flag work markers\n",
        );
        let r = load_recipe(tmp.path(), "todo").unwrap();
        assert_eq!(r.name, "todo");
        assert_eq!(r.pattern, "TODO|FIXME");
        assert_eq!(r.path.as_deref(), Some("crates"));
        assert_eq!(r.description.as_deref(), Some("Flag work markers"));
    }

    #[test]
    fn load_recipe_errors_on_missing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let err = load_recipe(tmp.path(), "nope").unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("nope"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn list_recipes_is_sorted_and_deterministic() {
        let tmp = tempfile::tempdir().unwrap();
        write_recipe(tmp.path(), "zebra", "name: zebra\npattern: z\n");
        write_recipe(tmp.path(), "apple", "name: apple\npattern: a\n");
        write_recipe(tmp.path(), "mango", "name: mango\npattern: m\n");
        let recipes = list_recipes(tmp.path()).unwrap();
        let names: Vec<_> = recipes.iter().map(|r| r.name.clone()).collect();
        assert_eq!(names, vec!["apple", "mango", "zebra"]);
    }

    #[test]
    fn list_recipes_empty_when_dir_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("nonexistent");
        let recipes = list_recipes(&missing).unwrap();
        assert!(recipes.is_empty());
    }

    #[test]
    fn render_list_includes_description_when_present() {
        let recipes = vec![
            Recipe {
                name: "todo".into(),
                pattern: "TODO".into(),
                path: None,
                description: Some("Work markers".into()),
            },
            Recipe {
                name: "bare".into(),
                pattern: "foo".into(),
                path: None,
                description: None,
            },
        ];
        let out = render_list(&recipes);
        assert!(out.contains("todo  —  Work markers"));
        assert!(out.lines().any(|l| l == "bare"));
    }

    #[test]
    fn render_list_reports_empty_catalogue() {
        let out = render_list(&[]);
        assert!(out.contains("No recipes found"));
    }

    #[test]
    fn run_errors_when_neither_recipe_nor_list() {
        let err = run(None, false).unwrap_err();
        assert!(format!("{err:#}").contains("recipe name or --list"));
    }
}
