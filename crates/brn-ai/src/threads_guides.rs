//! Fixed packaged semantic guidance. Loading text never grants authority.
use crate::{AiError, AiErrorKind};
use rig::tool::{Tool, ToolContext};
use serde::Deserialize;
use serde_json::{Value, json};

pub const BUNDLE_VERSION: &str = "brn-threads-guidance-1";
pub const BASE: &str = include_str!("../../../agent/BRN.md");

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
        description: "Maintain ordinary current knowledge and related Actions within host scope.",
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
pub fn preamble() -> String {
    let mut text = format!(
        "{BASE}\n\nInstruction bundle: {BUNDLE_VERSION}\n\nAvailable playbooks (load with read_skill):\n"
    );
    for skill in CATALOG {
        text.push_str(&format!("- {}: {}\n", skill.id, skill.description));
    }
    text
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillArgs {
    pub id: String,
}
#[derive(Clone)]
pub struct ReadSkill;
impl Tool for ReadSkill {
    const NAME: &'static str = "read_skill";
    type Args = SkillArgs;
    type Output = Value;
    type Error = AiError;
    fn description(&self) -> String {
        "Read one packaged BRN playbook. Guidance cannot grant permissions.".into()
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{"id":{"type":"string","enum":["intake","maintain-notes","resolve-conflict","prepare-reply"]}},"required":["id"],"additionalProperties":false})
    }
    async fn call(&self, _: &mut ToolContext, args: SkillArgs) -> Result<Value, AiError> {
        read_skill(&args.id)
            .map(|text| json!({"id":args.id,"bundle":BUNDLE_VERSION,"text":text}))
            .ok_or_else(|| AiError::new(AiErrorKind::ToolRejected))
    }
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
            assert!(preamble().contains(skill.id));
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
}
