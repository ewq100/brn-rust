//! Fixed packaged semantic guidance. Loading text never grants authority.
pub const BUNDLE_VERSION: &str = "brn-threads-guidance-12";
pub const BASE: &str = include_str!("../../../agent/BRN.md");
/// The research helper's packaged role guide (subagents phase 1).
pub const RESEARCH_SCOUT: &str = include_str!("../../../agent/roles/research-scout.md");

#[derive(Clone, Copy)]
pub struct Skill {
    pub id: &'static str,
    pub description: &'static str,
    pub text: &'static str,
}
pub const CATALOG: [Skill; 4] = [
    Skill {
        id: "intake",
        description: "Interpret supplied sources; honor useful information or faithful full-note import.",
        text: include_str!("../../../agent/skills/intake/SKILL.md"),
    },
    Skill {
        id: "maintain-notes",
        description: "Compile source-grounded ordinary knowledge, save reusable synthesis, or review affected pages.",
        text: include_str!("../../../agent/skills/maintain-notes/SKILL.md"),
    },
    Skill {
        id: "resolve-conflict",
        description: "Investigate disagreements and prepare the smallest owner decision.",
        text: include_str!("../../../agent/skills/resolve-conflict/SKILL.md"),
    },
    Skill {
        id: "prepare-reply",
        description: "Prepare a supported reply draft; never claim delivery.",
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
pub fn preamble(preloaded: &[&str]) -> String {
    let mut text = format!(
        "{BASE}\n\nInstruction bundle: {BUNDLE_VERSION}\n\nAvailable playbooks (load with read_skill):\n"
    );
    for skill in CATALOG.iter().filter(|s| !preloaded.contains(&s.id)) {
        text.push_str(&format!("- {}: {}\n", skill.id, skill.description));
    }
    for skill in CATALOG.iter().filter(|s| preloaded.contains(&s.id)) {
        text.push_str(&format!(
            "\nPlaybook {} (already loaded; no read_skill needed):\n\n{}\n",
            skill.id, skill.text
        ));
    }
    text
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
    fn fixed_catalog_has_valid_frontmatter_and_rejects_paths() {
        for skill in CATALOG {
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
