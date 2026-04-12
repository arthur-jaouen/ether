//! ROADMAP.md parsing and section-matching helpers shared across subcommands
//! (groom coverage + task `--context` linking).

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

/// A parsed ROADMAP heading plus the lines that follow it up to the next
/// heading of equal or shallower depth.
#[derive(Debug, Clone)]
pub struct Section {
    /// Heading text without leading hashes.
    pub title: String,
    /// Body text between this heading and the next peer/parent heading.
    pub body: String,
    /// Lowercased keywords extracted from the title (>=4 chars, alphanumeric).
    pub keywords: Vec<String>,
}

/// Parse a ROADMAP.md file into level-2 and level-3 sections. A missing file
/// is treated as an empty ROADMAP rather than an error so downstream callers
/// (e.g. `groom`, `task --context`) can degrade gracefully.
pub fn parse(path: &Path) -> Result<Vec<Section>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw =
        fs::read_to_string(path).with_context(|| format!("reading roadmap {}", path.display()))?;

    let mut sections: Vec<Section> = Vec::new();
    let mut cur: Option<(String, Vec<String>)> = None;

    let finish = |sections: &mut Vec<Section>, title: String, body: Vec<String>| {
        let keywords = extract_keywords(&title);
        if keywords.is_empty() {
            return;
        }
        let body = body.join("\n").trim_end().to_string();
        sections.push(Section {
            title,
            body,
            keywords,
        });
    };

    for line in raw.lines() {
        let trimmed = line.trim_start();
        let heading = trimmed
            .strip_prefix("### ")
            .or_else(|| trimmed.strip_prefix("## "));
        if let Some(title) = heading {
            if let Some((t, b)) = cur.take() {
                finish(&mut sections, t, b);
            }
            cur = Some((title.trim().to_string(), Vec::new()));
        } else if let Some((_, body)) = cur.as_mut() {
            body.push(line.to_string());
        }
    }
    if let Some((t, b)) = cur {
        finish(&mut sections, t, b);
    }
    Ok(sections)
}

/// Extract match keywords from a heading or title. Short words and common
/// glue are dropped so section→task matching has signal.
pub fn extract_keywords(text: &str) -> Vec<String> {
    const STOP: &[&str] = &[
        "the",
        "and",
        "for",
        "with",
        "from",
        "into",
        "that",
        "this",
        "than",
        "then",
        "phase",
        "goal",
        "when",
        "what",
        "will",
        "over",
        "also",
        "only",
        "non-goals",
        "non",
    ];
    text.split(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
        .filter_map(|w| {
            let w = w.trim_matches('-').to_ascii_lowercase();
            if w.len() < 4 || STOP.contains(&w.as_str()) {
                None
            } else {
                Some(w)
            }
        })
        .collect()
}

/// Does a task haystack (title + body, lowercased) match this section? Uses
/// the same two-keyword-minimum threshold as `groom` coverage so a task linked
/// to a section and a section flagged as covered by a task agree.
pub fn section_matches(section: &Section, task_haystack_lower: &str) -> bool {
    let hits = section
        .keywords
        .iter()
        .filter(|k| task_haystack_lower.contains(k.as_str()))
        .count();
    let threshold = if section.keywords.len() >= 2 { 2 } else { 1 };
    hits >= threshold
}

/// Find the section with the highest keyword overlap with the given task
/// title + body. Ties are broken by document order (first match wins).
pub fn best_match_for_task<'a>(
    task_title: &str,
    task_body: &str,
    sections: &'a [Section],
) -> Option<&'a Section> {
    let haystack = format!("{task_title} {task_body}").to_ascii_lowercase();
    let mut best: Option<(usize, &Section)> = None;
    for s in sections {
        if !section_matches(s, &haystack) {
            continue;
        }
        let hits = s
            .keywords
            .iter()
            .filter(|k| haystack.contains(k.as_str()))
            .count();
        if best.map(|(bs, _)| hits > bs).unwrap_or(true) {
            best = Some((hits, s));
        }
    }
    best.map(|(_, s)| s)
}

/// Find a section by exact (case-insensitive) title match. Used for explicit
/// `roadmap_section:` frontmatter tags on task files.
pub fn find_by_title<'a>(title: &str, sections: &'a [Section]) -> Option<&'a Section> {
    sections
        .iter()
        .find(|s| s.title.eq_ignore_ascii_case(title.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROADMAP: &str = "# ROADMAP\n\n## Phase 1 — Core storage\n\nSparse set foundations.\n\n### Component registration\n\nTypeId-based deterministic IDs.\n\n## Tooling\n\nether-forge backlog CLI.\n";

    fn write(raw: &str) -> tempfile::NamedTempFile {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(raw.as_bytes()).unwrap();
        f
    }

    #[test]
    fn parse_captures_headings_and_bodies() {
        let f = write(ROADMAP);
        let sections = parse(f.path()).unwrap();
        let titles: Vec<_> = sections.iter().map(|s| s.title.as_str()).collect();
        assert_eq!(
            titles,
            vec![
                "Phase 1 — Core storage",
                "Component registration",
                "Tooling",
            ]
        );
        assert!(sections[0].body.contains("Sparse set foundations"));
        // Body of Phase 1 stops before the next heading.
        assert!(!sections[0].body.contains("TypeId"));
        assert!(sections[1].body.contains("TypeId"));
        assert!(sections[2].body.contains("ether-forge"));
    }

    #[test]
    fn parse_missing_file_is_empty() {
        let path = std::path::PathBuf::from("/definitely/does/not/exist/ROADMAP.md");
        assert!(parse(&path).unwrap().is_empty());
    }

    #[test]
    fn extract_keywords_drops_stop_words_and_short_tokens() {
        let kw = extract_keywords("Phase 1 — Core storage and queries");
        assert!(kw.contains(&"core".to_string()));
        assert!(kw.contains(&"storage".to_string()));
        assert!(kw.contains(&"queries".to_string()));
        assert!(!kw.contains(&"phase".to_string()));
        assert!(!kw.contains(&"and".to_string()));
    }

    #[test]
    fn best_match_prefers_strongest_overlap() {
        let f = write(ROADMAP);
        let sections = parse(f.path()).unwrap();
        // Task about component registration should prefer its section.
        let s = best_match_for_task(
            "Deterministic component registration",
            "typeid based component ids",
            &sections,
        )
        .unwrap();
        assert_eq!(s.title, "Component registration");
    }

    #[test]
    fn best_match_returns_none_when_no_overlap() {
        let f = write(ROADMAP);
        let sections = parse(f.path()).unwrap();
        assert!(best_match_for_task("unrelated thing", "", &sections).is_none());
    }

    #[test]
    fn find_by_title_is_case_insensitive() {
        let f = write(ROADMAP);
        let sections = parse(f.path()).unwrap();
        let s = find_by_title("tooling", &sections).unwrap();
        assert_eq!(s.title, "Tooling");
    }
}
