//! Line-based YAML frontmatter editor.
//!
//! Preserves field order and comments in untouched entries. Supports the
//! narrow subset of YAML used in backlog task files: top-level scalars and
//! simple block lists (`key:\n  - item`).

use std::fmt;

use anyhow::{anyhow, Result};

/// A single top-level key and the raw lines that belong to it (including
/// any indented continuation lines for block lists).
#[derive(Debug, Clone)]
struct Entry {
    key: String,
    lines: Vec<String>,
}

/// Editable view over a frontmatter block.
#[derive(Debug, Clone)]
pub struct Frontmatter {
    entries: Vec<Entry>,
}

impl Frontmatter {
    /// Parse a frontmatter block (the text between the `---` fences, without
    /// the fences themselves).
    pub fn parse(raw: &str) -> Result<Self> {
        let mut entries: Vec<Entry> = Vec::new();
        for line in raw.lines() {
            if line.is_empty() || line.starts_with(' ') || line.starts_with('\t') {
                // Continuation of the previous entry (indented list item or
                // blank line inside a block).
                match entries.last_mut() {
                    Some(e) => e.lines.push(line.to_string()),
                    None => {
                        return Err(anyhow!(
                            "frontmatter begins with indented or blank line: {line:?}"
                        ))
                    }
                }
                continue;
            }
            let colon = line
                .find(':')
                .ok_or_else(|| anyhow!("frontmatter line missing ':' — {line:?}"))?;
            let key = line[..colon].trim().to_string();
            entries.push(Entry {
                key,
                lines: vec![line.to_string()],
            });
        }
        Ok(Frontmatter { entries })
    }

    // Display impl below serializes back to text (no trailing newline).

    /// Return the inline scalar value for `key`, if any.
    pub fn scalar(&self, key: &str) -> Option<&str> {
        let entry = self.entries.iter().find(|e| e.key == key)?;
        let first = entry.lines.first()?;
        let rest = first.split_once(':').map(|(_, v)| v.trim())?;
        if rest.is_empty() || entry.lines.len() > 1 {
            // Block form (list) — no inline scalar.
            if rest.is_empty() {
                return None;
            }
        }
        Some(rest)
    }

    /// Replace the scalar value for `key`. Appends the key if missing.
    pub fn set_scalar(&mut self, key: &str, value: &str) {
        let line = format!("{key}: {value}");
        if let Some(entry) = self.entries.iter_mut().find(|e| e.key == key) {
            entry.lines = vec![line];
        } else {
            self.entries.push(Entry {
                key: key.to_string(),
                lines: vec![line],
            });
        }
    }

    /// Remove `key` entirely.
    pub fn remove(&mut self, key: &str) {
        self.entries.retain(|e| e.key != key);
    }

    /// Return the items of a block-list field (`key:\n  - item`).
    pub fn list_items(&self, key: &str) -> Vec<String> {
        let Some(entry) = self.entries.iter().find(|e| e.key == key) else {
            return Vec::new();
        };
        entry
            .lines
            .iter()
            .skip(1)
            .filter_map(|l| {
                l.trim_start()
                    .strip_prefix("- ")
                    .map(|s| s.trim().to_string())
            })
            .collect()
    }

    /// Remove the first list item matching `item` from `key`.
    ///
    /// Returns `Ok(true)` if the list is now empty (caller may want to drop
    /// the key), `Ok(false)` if items remain, and an error if the key is not
    /// present or the item is not found.
    pub fn remove_list_item(&mut self, key: &str, item: &str) -> Result<bool> {
        let entry = self
            .entries
            .iter_mut()
            .find(|e| e.key == key)
            .ok_or_else(|| anyhow!("frontmatter has no `{key}` field"))?;
        let idx = entry
            .lines
            .iter()
            .position(|l| {
                l.trim_start()
                    .strip_prefix("- ")
                    .map(|s| s.trim() == item)
                    .unwrap_or(false)
            })
            .ok_or_else(|| anyhow!("`{key}` does not contain item `{item}`"))?;
        entry.lines.remove(idx);
        // A block list entry with only the header line left is empty.
        let empty = entry.lines.len() <= 1;
        Ok(empty)
    }
}

impl fmt::Display for Frontmatter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let last = self.entries.len().saturating_sub(1);
        for (i, entry) in self.entries.iter().enumerate() {
            let line_last = entry.lines.len().saturating_sub(1);
            for (j, line) in entry.lines.iter().enumerate() {
                f.write_str(line)?;
                if !(i == last && j == line_last) {
                    f.write_str("\n")?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "id: T11\ntitle: install-hooks subcommand\nsize: S\nstatus: blocked\ndepends_on:\n  - T7\n  - T9\npriority: 8";

    #[test]
    fn roundtrip_preserves_lines() {
        let fm = Frontmatter::parse(SAMPLE).unwrap();
        assert_eq!(fm.to_string(), SAMPLE);
    }

    #[test]
    fn reads_scalars_and_list() {
        let fm = Frontmatter::parse(SAMPLE).unwrap();
        assert_eq!(fm.scalar("id"), Some("T11"));
        assert_eq!(fm.scalar("status"), Some("blocked"));
        assert_eq!(fm.scalar("priority"), Some("8"));
        assert_eq!(fm.list_items("depends_on"), vec!["T7", "T9"]);
    }

    #[test]
    fn set_scalar_replaces_in_place() {
        let mut fm = Frontmatter::parse(SAMPLE).unwrap();
        fm.set_scalar("status", "ready");
        assert!(fm.to_string().contains("status: ready"));
        // Priority line untouched.
        assert!(fm.to_string().contains("priority: 8"));
    }

    #[test]
    fn set_scalar_appends_when_missing() {
        let mut fm = Frontmatter::parse(SAMPLE).unwrap();
        fm.set_scalar("commit", "abc1234");
        assert!(fm.to_string().ends_with("commit: abc1234"));
    }

    #[test]
    fn remove_list_item_reports_empty() {
        let mut fm = Frontmatter::parse(SAMPLE).unwrap();
        assert!(!fm.remove_list_item("depends_on", "T7").unwrap());
        assert_eq!(fm.list_items("depends_on"), vec!["T9"]);
        assert!(fm.remove_list_item("depends_on", "T9").unwrap());
        assert!(fm.list_items("depends_on").is_empty());
    }

    #[test]
    fn remove_list_item_missing_errors() {
        let mut fm = Frontmatter::parse(SAMPLE).unwrap();
        assert!(fm.remove_list_item("depends_on", "T99").is_err());
        assert!(fm.remove_list_item("nope", "T7").is_err());
    }

    #[test]
    fn remove_drops_key() {
        let mut fm = Frontmatter::parse(SAMPLE).unwrap();
        fm.remove("depends_on");
        let out = fm.to_string();
        assert!(!out.contains("depends_on"));
        assert!(!out.contains("- T7"));
        assert!(out.contains("priority: 8"));
    }
}
