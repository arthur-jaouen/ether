---
name: self-improve
description: Review recent project activity and improve Claude's memory, skills, and project knowledge. Run periodically to stay current.
disable-model-invocation: true
---

# Self-Improve

Review recent activity and improve your own knowledge base. Present all proposed changes for approval before writing.

## Step 1: Gather recent activity

- `cd /home/arthur/ether && git log --oneline --since="2 weeks ago" --no-merges --all`
- Read memory index: check for `MEMORY.md` in the project memory directory
- Read all memory files referenced in the index
- Read existing skills: `.claude/skills/*/SKILL.md`
- Read `CLAUDE.md`

## Step 2: Check memory health

For each existing memory file:
1. **Stale check**: references to files/functions that no longer exist?
2. **Accuracy check**: still reflects reality?
3. **Relevance check**: still useful?
4. **Gaps**: project context from recent git activity that should be remembered? (Only non-obvious things not derivable from code/git.)

## Step 3: Check skills health

For each skill:
1. **Convention drift**: do templates/formats match actual usage?
2. **Missing patterns**: recurring workflows that could be a new skill?
3. **Accuracy**: file paths and references still valid?

## Step 4: Check for new project knowledge

From recent git activity:
1. New patterns that future conversations should know about
2. Completed work that old memories might reference
3. New conventions visible in recent code

Only flag things **non-obvious** and **not derivable from code**.

## Step 5: Present findings

```
## Memory
- [UPDATE] <file> — <what changed and why>
- [DELETE] <file> — <why stale>
- [CREATE] <topic> — <what to remember>
- [OK] <file> — still accurate

## Skills
- [UPDATE] <skill> — <what drifted>
- [CREATE] <name> — <recurring pattern>
- [OK] <skill> — still accurate

## No action needed
- <things checked that are fine>
```

## Rules

- **Never write changes without user approval.**
- Keep memory minimal — don't save things derivable from code or git.
- Don't save ephemeral information.
- Preserve frontmatter format in memory files.
- Verify file paths and patterns exist before recommending.
- If nothing needs changing, say so.
