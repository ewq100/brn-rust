//! Fixed packaged semantic guidance. Loading text never grants authority.
pub const BUNDLE_VERSION: &str = "brn-threads-guidance-12";
pub const BASE: &str = include_str!("../../../agent/BRN.md");
/// The research helper's packaged role guide (subagents phase 1).
pub const RESEARCH_SCOUT: &str = include_str!("../../../agent/roles/research-scout.md");

#[derive(Clone, Copy)]
pub struct Skill {
    pub id: &'static str,
    pub text: &'static str,
}
pub const CATALOG: [Skill; 4] = [
    Skill {
        id: "intake",
        text: include_str!("../../../agent/skills/intake/SKILL.md"),
    },
    Skill {
        id: "maintain-notes",
        text: include_str!("../../../agent/skills/maintain-notes/SKILL.md"),
    },
    Skill {
        id: "resolve-conflict",
        text: include_str!("../../../agent/skills/resolve-conflict/SKILL.md"),
    },
    Skill {
        id: "prepare-reply",
        text: include_str!("../../../agent/skills/prepare-reply/SKILL.md"),
    },
];

pub fn read_skill(id: &str) -> Option<&'static str> {
    CATALOG
        .iter()
        .find(|skill| skill.id == id)
        .map(|skill| skill.text)
}
/// Playbooks included in the instructions from the start: a run with
/// supplied sources always needs intake and maintain-notes.
pub fn preloaded(sources: bool) -> &'static [&'static str] {
    if sources {
        &["intake", "maintain-notes"]
    } else {
        &[]
    }
}
/// Test consumers exercise the same frozen production renderer; no second
/// packaged-only preamble is retained in the shipped runtime.
#[cfg(test)]
pub fn preamble(preloaded: &[&str]) -> String {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let store = brn_threads_core::Store::open(root.path().join("data")).unwrap();
    crate::thread_skills::Catalogue::freeze(&store, &[], None, !preloaded.is_empty()).preamble()
}

/// A helper sub-run's complete instructions: its role guide only, so the
/// parent's write and authority guidance never reaches it.
pub fn scout_preamble() -> String {
    format!("{RESEARCH_SCOUT}\n\nInstruction bundle: {BUNDLE_VERSION}\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runtime_descriptions_are_parsed_from_packaged_frontmatter() {
        for skill in CATALOG {
            let metadata = skill_metadata(skill.text).unwrap();
            assert!(
                preamble(&[]).contains(metadata.description),
                "{} metadata was replaced",
                skill.id
            );
        }
    }

    #[test]
    fn fixed_catalog_has_valid_frontmatter_and_rejects_paths() {
        for skill in CATALOG {
            let metadata = skill_metadata(skill.text).unwrap();
            assert_eq!(metadata.name, skill.id);
            assert!(skill.text.starts_with("---\n"));
            let header = skill.text.split("---\n").nth(1).unwrap();
            assert!(
                header
                    .lines()
                    .any(|line| line == format!("name: {}", skill.id))
            );
            assert!(
                header
                    .lines()
                    .any(|line| line.starts_with("description: ") && line.len() > 20)
            );
            assert!(preamble(&[]).contains(skill.id));
            assert!(preamble(preloaded(true)).contains(skill.id));
        }
        for invalid in [
            "../AGENTS.md",
            "intake/SKILL.md",
            "https://example.com",
            "",
            "Intake",
        ] {
            assert!(read_skill(invalid).is_none());
        }
    }

    #[test]
    fn guidance_says_when_to_withdraw_an_obsolete_proposal() {
        assert!(preamble(&[]).contains("Instruction bundle: brn-threads-guidance-12"));
        assert!(scout_preamble().contains("brn-threads-guidance-12"));
        assert!(BASE.contains("withdraw it with `withdraw_proposal`"));
        assert!(
            read_skill("resolve-conflict")
                .unwrap()
                .contains("withdraw your own pending proposal that contradicts the decision")
        );
        // The read-only helper's role guide never offers it.
        assert!(!RESEARCH_SCOUT.contains("withdraw_proposal"));
    }
}

/// The C0 single-line subset, deliberately not a YAML parser. Shared by owner
/// installation and packaged-metadata inspection; it grants no runtime tools.
#[derive(Debug, PartialEq, Eq)]
pub struct SkillMetadata<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub body: &'a str,
}
pub fn valid_skill_name(name: &str) -> bool {
    (1..=64).contains(&name.len())
        && name.as_bytes()[0].is_ascii_lowercase()
        && name
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}
pub fn skill_metadata(text: &str) -> Result<SkillMetadata<'_>, &'static str> {
    if text.len() > 64 * 1024 || text.contains('\0') {
        return Err("SKILL.md must be UTF-8 text of at most 64 KiB without NUL");
    }
    let mut lines = text.split_inclusive('\n');
    let first = lines.next().ok_or("missing frontmatter")?;
    if metadata_line(first) != "---" {
        return Err("frontmatter must start with ---");
    }
    let mut offset = first.len();
    let (mut name, mut description) = (None, None);
    for line in lines {
        offset += line.len();
        if offset > 8 * 1024 {
            return Err("frontmatter exceeds 8 KiB");
        }
        let line = metadata_line(line);
        if line == "---" {
            let name = name.ok_or("name required")?;
            let description = description.ok_or("description required")?;
            let body = &text[offset..];
            if body.trim().is_empty() {
                return Err("instruction body required");
            }
            return Ok(SkillMetadata {
                name,
                description,
                body,
            });
        }
        let (key, value) = line
            .split_once(": ")
            .ok_or("only literal key: value lines supported")?;
        if value.is_empty()
            || value.trim() != value
            || value
                .chars()
                .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
            || value.starts_with([
                '-', '?', ':', ',', '[', ']', '{', '}', '#', '&', '*', '!', '|', '>', '\'', '"',
                '%', '@', '`',
            ])
        {
            return Err("empty, control-bearing or YAML-shaped metadata is unsupported");
        }
        match key {
            "name" if name.is_none() && valid_skill_name(value) => name = Some(value),
            "description" if description.is_none() && value.len() <= 1024 => {
                description = Some(value)
            }
            _ => return Err("invalid, duplicate, unknown or unsupported metadata field"),
        }
    }
    Err("frontmatter closing --- required")
}

fn metadata_line(line: &str) -> &str {
    let line = line.strip_suffix('\n').unwrap_or(line);
    line.strip_suffix('\r').unwrap_or(line)
}

/// Stable D3 package identity over sorted, length-framed UTF-8 files. Shared
/// by packaged runtime and descriptor-validated installed snapshots.
pub(crate) fn package_digest(files: &std::collections::BTreeMap<String, String>) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"brn-instruction-package-v1\0");
    for (path, text) in files {
        h.update((path.len() as u64).to_be_bytes());
        h.update(path.as_bytes());
        h.update((text.len() as u64).to_be_bytes());
        h.update(text.as_bytes());
    }
    format!("{:x}", h.finalize())
}
