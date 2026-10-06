//! Exact single-image Source metadata and its separately approved annotation.
//! These pure checks grant neither filesystem freshness nor semantic truth.
use super::{inbox_source, proposals::SourceVersion};
use crate::{MAX_NOTE_BYTES, Result, hash, invalid};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MAX_VISUAL_TEXT_BYTES: usize = 8 * 1024;
pub const MAX_VISUAL_SECTION_BYTES: usize = 20 * 1024;
const SECTION_START: &str =
    "\n## Visual interpretation\n\n<!-- brn:visual-interpretation:1:start -->\n";
const SECTION_END: &str = "\n<!-- brn:visual-interpretation:1:end -->\n";
const PENDING: &str = "Pending explicit visual interpretation. Source and asset approval does not approve an interpretation.";
const TENTATIVE: &str =
    "Tentative model interpretation; approved wording does not establish verified source facts.";
const DESCRIPTION: &str = "\n\n**Tentative description:**\n";
const UNCERTAINTY: &str = "\n\n**Uncertainty:**\n";

/// Path-neutral original/image metadata. The asset basename is reconstructed
/// from the complete original digest, never adopted from a package path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxSourceVisual {
    pub part_name: String,
    pub relationship_id: String,
    pub asset_name: String,
    pub byte_len: u64,
    pub sha256: [u8; 32],
    pub width: u32,
    pub height: u32,
    pub alt_text: Option<String>,
    pub title: Option<String>,
    pub image_start: usize,
    pub image_end: usize,
    pub converted_byte_len: u64,
    pub converted_sha256: [u8; 32],
}

pub fn asset_name(original_sha256: &[u8; 32]) -> String {
    let digest: String = original_sha256.iter().map(|b| format!("{b:02x}")).collect();
    format!("brn-inbox-image-{digest}-1.png")
}

impl InboxSourceVisual {
    pub fn validate(&self, original_sha256: &[u8; 32]) -> Result<()> {
        if self.asset_name != asset_name(original_sha256)
            || self.part_name.is_empty()
            || self.part_name.len() > 2048
            || !self.part_name.is_ascii()
            || self.part_name.contains(['\\', ':', '%', '?', '#'])
            || self
                .part_name
                .chars()
                .any(|c| c.is_control() || c.is_whitespace())
            || self
                .part_name
                .split('/')
                .any(|part| matches!(part, "" | "." | ".."))
            || self.relationship_id.is_empty()
            || self.relationship_id.len() > 512
            || self
                .relationship_id
                .chars()
                .any(|c| c.is_control() || c.is_whitespace())
            || !(8..=1024 * 1024).contains(&self.byte_len)
            || !(1..=4096).contains(&self.width)
            || !(1..=4096).contains(&self.height)
            || u64::from(self.width) * u64::from(self.height) > 4_194_304
            || self.image_start >= self.image_end
            || self.image_end as u64 > self.converted_byte_len
            || self.converted_byte_len > MAX_NOTE_BYTES as u64
            || self
                .alt_text
                .iter()
                .chain(self.title.iter())
                .any(|value| value.len() > 2048)
        {
            return Err(invalid(
                "Inbox visual needs one exact bounded PNG occurrence",
            ));
        }
        Ok(())
    }

    pub fn asset_path(&self, source_path: &str, original_sha256: &[u8; 32]) -> Result<String> {
        self.validate(original_sha256)?;
        super::proposals::validate_path(source_path)?;
        let path = source_path.rsplit_once('/').map_or_else(
            || self.asset_name.clone(),
            |(parent, _)| format!("{parent}/{}", self.asset_name),
        );
        super::proposals::validate_asset_path(&path)?;
        Ok(path)
    }

    pub fn validate_converted(&self, original_sha256: &[u8; 32], body: &str) -> Result<()> {
        self.validate(original_sha256)?;
        let markup = inbox_source::docx_inline_png_markdown(
            &self.asset_name,
            self.alt_text.as_deref(),
            self.title.as_deref(),
        )?;
        if body.len() as u64 != self.converted_byte_len
            || hash(body.as_bytes()) != self.converted_sha256
            || body.get(self.image_start..self.image_end) != Some(markup.as_str())
        {
            return Err(invalid(
                "Inbox visual differs from its exact converted occurrence",
            ));
        }
        Ok(())
    }

    /// A Source may change only inside its designated annotation section.
    /// Its converted wording and actual image reference remain exact evidence.
    pub fn validate_source_body<'a>(
        &self,
        original_sha256: &[u8; 32],
        body: &'a str,
    ) -> Result<&'a str> {
        let end = usize::try_from(self.converted_byte_len)
            .map_err(|_| invalid("Inbox visual body size is invalid"))?;
        let converted = body
            .get(..end)
            .ok_or_else(|| invalid("Inbox visual body is incomplete"))?;
        self.validate_converted(original_sha256, converted)?;
        let section = &body[end..];
        if section.len() < SECTION_START.len() + SECTION_END.len()
            || section.len() > MAX_VISUAL_SECTION_BYTES
            || !section.starts_with(SECTION_START)
            || !section.ends_with(SECTION_END)
            || section[SECTION_START.len()..section.len() - SECTION_END.len()]
                .contains(['\0', '\r'])
            || section.matches(SECTION_START).count() != 1
            || section.matches(SECTION_END).count() != 1
        {
            return Err(invalid(
                "Inbox visual needs its complete designated annotation section",
            ));
        }
        Ok(converted)
    }
}

pub fn pending_section() -> String {
    format!("{SECTION_START}{PENDING}{SECTION_END}")
}

pub fn validate_capture_asset(
    source_path: &str,
    source_text: &str,
    asset: &SourceVersion,
) -> Result<()> {
    let provenance = inbox_source::read_provenance(source_text)?
        .ok_or_else(|| invalid("Visual analysis needs retained original provenance"))?;
    let visual = provenance
        .visual
        .as_ref()
        .ok_or_else(|| invalid("Visual analysis needs a bound PNG occurrence"))?;
    if asset.path != visual.asset_path(source_path, &provenance.original_sha256)?
        || asset.fingerprint.len != visual.byte_len
        || asset.fingerprint.sha256 != visual.sha256
        || asset.fingerprint.device == 0
        || asset.fingerprint.inode == 0
    {
        return Err(invalid(
            "Visual asset differs from its exact Source manifest",
        ));
    }
    Ok(())
}

fn escaped_literal(text: &str) -> String {
    let mut out = String::new();
    for character in text.chars() {
        match character {
            '\n' => out.push_str("<br>"),
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\\' | '`' | '*' | '_' | '{' | '}' | '[' | ']' | '(' | ')' | '#' | '+' | '-' | '.'
            | '!' | '|' | '~' => {
                out.push('\\');
                out.push(character);
            }
            _ => out.push(character),
        }
    }
    out
}

fn validate_text(value: &str) -> Result<()> {
    if value.trim().is_empty()
        || value.len() > MAX_VISUAL_TEXT_BYTES
        || value
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\t'))
    {
        return Err(invalid(
            "Visual interpretation needs bounded description and uncertainty",
        ));
    }
    Ok(())
}

/// An existing proposal-family Replace can annotate only this saved Source.
/// Complete captured text keeps terminal history independent of the live queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxVisualAnnotationBinding {
    pub analysis_id: Uuid,
    pub note_id: Uuid,
    pub source: SourceVersion,
    pub source_text: String,
    pub asset: SourceVersion,
    pub description: String,
    pub uncertainty: String,
}

impl InboxVisualAnnotationBinding {
    pub fn validate_capture(&self, job: &super::inbox_actions::InboxActionJob) -> Result<()> {
        self.validate()?;
        job.validate()?;
        if job.capture.purpose != super::inbox_actions::InboxAnalysisPurpose::VisualInterpretation
            || job.capture.id != self.analysis_id
            || job.capture.note_id()? != self.note_id
            || job.capture.source != self.source
            || job.capture.source_text != self.source_text
            || job.capture.visual_asset.as_ref() != Some(&self.asset)
        {
            return Err(invalid(
                "Visual proposal differs from its exact analysis capture",
            ));
        }
        Ok(())
    }
    pub fn validate(&self) -> Result<()> {
        if self.analysis_id.is_nil()
            || self.note_id.is_nil()
            || self.source_text.len() > super::inbox_actions::MAX_INBOX_ACTION_SOURCE_BYTES
            || self.source.fingerprint.len != self.source_text.len() as u64
            || self.source.fingerprint.sha256 != hash(self.source_text.as_bytes())
            || crate::note_identity::read(&self.source_text)? != Some(self.note_id)
            || !crate::note_metadata::classify(&self.source_text)?.source
        {
            return Err(invalid(
                "Visual annotation needs its exact managed Source evidence",
            ));
        }
        super::proposals::validate_path(&self.source.path)?;
        let provenance = inbox_source::read_provenance(&self.source_text)?
            .ok_or_else(|| invalid("Visual annotation needs retained original provenance"))?;
        let visual = provenance
            .visual
            .as_ref()
            .ok_or_else(|| invalid("Visual annotation needs a bound PNG occurrence"))?;
        visual.validate_source_body(
            &provenance.original_sha256,
            &self.source_text[crate::note_identity::body_start(&self.source_text)?..],
        )?;
        validate_capture_asset(&self.source.path, &self.source_text, &self.asset)?;
        validate_text(&self.description)?;
        validate_text(&self.uncertainty)?;
        Ok(())
    }

    pub fn candidate_text(&self) -> Result<String> {
        self.validate()?;
        let provenance =
            inbox_source::read_provenance(&self.source_text)?.expect("validated provenance");
        let visual = provenance.visual.as_ref().expect("validated visual");
        let end = crate::note_identity::body_start(&self.source_text)?
            .checked_add(
                usize::try_from(visual.converted_byte_len)
                    .map_err(|_| invalid("Visual annotation body size is invalid"))?,
            )
            .ok_or_else(|| invalid("Visual annotation body size is invalid"))?;
        let section = format!(
            "{SECTION_START}{TENTATIVE}{DESCRIPTION}{}{UNCERTAINTY}{}{SECTION_END}",
            escaped_literal(&self.description),
            escaped_literal(&self.uncertainty),
        );
        if section.len() > MAX_VISUAL_SECTION_BYTES
            || end
                .checked_add(section.len())
                .is_none_or(|length| length > MAX_NOTE_BYTES)
        {
            return Err(invalid(
                "Visual annotation exceeds the complete Source bound",
            ));
        }
        Ok(self.source_text[..end].to_owned() + &section)
    }

    pub fn validate_replace(
        &self,
        path: &str,
        before: &crate::files::FileFingerprint,
        before_text: &str,
        candidate: &str,
    ) -> Result<()> {
        self.validate()?;
        let original = self.candidate_text()?;
        let fixed_end = original.len()
            - original
                .rsplit_once(SECTION_START)
                .ok_or_else(|| invalid("Visual annotation section is missing"))?
                .1
                .len();
        let section = candidate
            .get(fixed_end..)
            .ok_or_else(|| invalid("Visual annotation is incomplete"))?;
        let wording = section
            .strip_prefix(&format!("{TENTATIVE}{DESCRIPTION}"))
            .and_then(|s| s.strip_suffix(SECTION_END))
            .ok_or_else(|| {
                invalid("Visual annotation must retain tentative labels and uncertainty")
            })?;
        let (description, uncertainty) = wording
            .split_once(UNCERTAINTY)
            .ok_or_else(|| invalid("Visual annotation must retain separate uncertainty"))?;
        if path != self.source.path
            || before != &self.source.fingerprint
            || before_text != self.source_text
            || candidate.get(..fixed_end) != original.get(..fixed_end)
            || candidate.len() > MAX_NOTE_BYTES
            || section.len() + SECTION_START.len() > MAX_VISUAL_SECTION_BYTES
            || description.trim().is_empty()
            || uncertainty.trim().is_empty()
            || section.contains(['\0', '\r'])
            || section.contains(SECTION_START)
            || wording.contains(SECTION_END)
            || uncertainty.contains(UNCERTAINTY)
            || wording.contains(DESCRIPTION)
        {
            return Err(invalid(
                "Visual annotation must retain its exact target and protected literal Source",
            ));
        }
        Ok(())
    }
}

pub(super) fn check_binding(
    conn: &rusqlite::Connection,
    binding: &InboxVisualAnnotationBinding,
) -> Result<()> {
    let job = super::inbox_actions::reserved(conn, binding.analysis_id)?
        .ok_or_else(|| invalid("Visual annotation analysis capture is unavailable"))?;
    binding.validate_capture(&job)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::{FileFingerprint, VaultIdentity};
    use crate::work::{
        inbox::{InboxCapture, InboxCopy, InboxItem, InboxKind},
        inbox_processing::InboxConversionFormat,
        inbox_source::InboxSourceBinding,
        proposals::NoteChange,
    };
    use std::sync::atomic::AtomicBool;

    fn fixture() -> (InboxSourceBinding, String, Vec<u8>) {
        let bytes = include_bytes!("../../../brn-ai/src/fixtures/capability.png").to_vec();
        let facts = inbox_source::validate_png_image(&bytes, &AtomicBool::new(false)).unwrap();
        let original_sha256 = hash(b"complete synthetic DOCX original");
        let asset_name = asset_name(&original_sha256);
        let alt = "Illustration õ [not an instruction]";
        let title = "Exact original title";
        let image =
            inbox_source::docx_inline_png_markdown(&asset_name, Some(alt), Some(title)).unwrap();
        let before = "Exact before õ\n\n";
        let converted = format!("{before}{image}\n\nExact caption and after\n");
        let binding = InboxSourceBinding {
            visual: Some(InboxSourceVisual {
                part_name: "word/media/image1.png".into(),
                relationship_id: "image1".into(),
                asset_name,
                byte_len: bytes.len() as u64,
                sha256: hash(&bytes),
                width: facts.width,
                height: facts.height,
                alt_text: Some(alt.into()),
                title: Some(title.into()),
                image_start: before.len(),
                image_end: before.len() + image.len(),
                converted_byte_len: converted.len() as u64,
                converted_sha256: hash(converted.as_bytes()),
            }),
            batch_id: Uuid::from_u128(1),
            index: 0,
            original: InboxItem {
                capture: InboxCapture {
                    id: Uuid::from_u128(2),
                    kind: InboxKind::Binary,
                    title: "Synthetic document".into(),
                    original_name: Some("document.docx".into()),
                    copy: InboxCopy {
                        directory: "/synthetic/inbox".into(),
                        directory_device: 1,
                        directory_inode: 2,
                        file_device: 1,
                        file_inode: 3,
                        byte_len: b"complete synthetic DOCX original".len() as u64,
                        sha256: original_sha256,
                    },
                },
                received_at_ms: 1,
            },
            format: InboxConversionFormat::DocxInlinePngV1,
            byte_len: converted.len() as u64,
            sha256: hash(converted.as_bytes()),
            note_id: Uuid::from_u128(3),
        };
        (binding, converted, bytes)
    }

    fn fingerprint(text: &str) -> FileFingerprint {
        FileFingerprint {
            device: 1,
            inode: 20,
            len: text.len() as u64,
            sha256: hash(text.as_bytes()),
        }
    }

    #[test]
    fn source_requires_exact_contained_asset_pair_and_complete_png_bytes() {
        let (binding, converted, bytes) = fixture();
        let text = binding.markdown(&converted).unwrap();
        let source_path = "sources/doc.md";
        let visual = binding.visual.as_ref().unwrap();
        let path = visual
            .asset_path(source_path, &binding.original.capture.copy.sha256)
            .unwrap();
        assert!(path.starts_with("sources/brn-inbox-image-"));
        let parent = VaultIdentity {
            device: 1,
            inode: 10,
        };
        let changes = vec![
            NoteChange::Create {
                path: source_path.into(),
                parent: parent.clone(),
                text: text.clone(),
            },
            NoteChange::CreateAsset {
                path: path.clone(),
                parent,
                bytes: bytes.clone(),
            },
        ];
        binding.validate_members(&changes).unwrap();
        assert!(binding.validate_members(&changes[..1]).is_err());
        assert!(
            binding
                .validate_members(&[changes[1].clone(), changes[0].clone()])
                .is_err()
        );
        let mut substituted = bytes.clone();
        substituted[20] ^= 1;
        assert!(
            binding
                .validate_asset(source_path, &path, &substituted)
                .is_err()
        );
        assert!(
            binding
                .validate_asset(source_path, &format!("other/{}", visual.asset_name), &bytes)
                .is_err()
        );
        assert!(
            binding
                .validate_asset(
                    source_path,
                    &format!("sources/../{}", visual.asset_name),
                    &bytes
                )
                .is_err()
        );
        let mut forged = binding.clone();
        forged.visual.as_mut().unwrap().width += 1;
        assert!(forged.validate_asset(source_path, &path, &bytes).is_err());
        assert!(text.ends_with(&pending_section()));
        assert!(
            inbox_source::read_provenance(&text)
                .unwrap()
                .unwrap()
                .visual
                .is_some()
        );
    }

    #[test]
    fn annotation_preserves_complete_literal_metadata_and_occurrence_outside_its_section() {
        let (binding, converted, bytes) = fixture();
        let source_text = binding.markdown(&converted).unwrap();
        let visual = binding.visual.as_ref().unwrap();
        let source_path = "sources/doc.md";
        let annotation = InboxVisualAnnotationBinding {
            analysis_id: Uuid::from_u128(4),
            note_id: binding.note_id,
            source: SourceVersion {
                path: source_path.into(),
                fingerprint: fingerprint(&source_text),
            },
            source_text: source_text.clone(),
            asset: SourceVersion {
                path: visual
                    .asset_path(source_path, &binding.original.capture.copy.sha256)
                    .unwrap(),
                fingerprint: FileFingerprint {
                    device: 1,
                    inode: 21,
                    len: bytes.len() as u64,
                    sha256: hash(&bytes),
                },
            },
            description: "A tentative shape.\n<script>instructions</script> [link](bad)".into(),
            uncertainty: "The depicted object's intent is unclear.".into(),
        };
        let candidate = annotation.candidate_text().unwrap();
        annotation
            .validate_replace(
                source_path,
                &annotation.source.fingerprint,
                &source_text,
                &candidate,
            )
            .unwrap();
        let end = crate::note_identity::body_start(&source_text).unwrap() + converted.len();
        assert_eq!(&candidate[..end], &source_text[..end]);
        assert_eq!(
            inbox_source::read_provenance(&candidate).unwrap(),
            inbox_source::read_provenance(&source_text).unwrap()
        );
        assert!(candidate.contains(TENTATIVE));
        assert!(candidate.contains("&lt;script&gt;instructions&lt;/script&gt;"));
        assert!(candidate.contains("\\[link\\]\\(bad\\)"));
        assert!(candidate.contains("**Uncertainty:**"));
        assert_eq!(annotation.source_text, source_text);
        let edited =
            candidate.replacen("A tentative shape", "An owner-reviewed tentative shape", 1);
        annotation
            .validate_replace(
                source_path,
                &annotation.source.fingerprint,
                &source_text,
                &edited,
            )
            .unwrap();
        let missing_uncertainty = edited.replacen("**Uncertainty:**", "**Verified:**", 1);
        assert!(
            annotation
                .validate_replace(
                    source_path,
                    &annotation.source.fingerprint,
                    &source_text,
                    &missing_uncertainty
                )
                .is_err()
        );
        let modified = candidate.replacen("Exact caption", "Invented caption", 1);
        assert!(
            annotation
                .validate_replace(
                    source_path,
                    &annotation.source.fingerprint,
                    &source_text,
                    &modified
                )
                .is_err()
        );
        assert!(
            annotation
                .validate_replace(
                    "other.md",
                    &annotation.source.fingerprint,
                    &source_text,
                    &candidate
                )
                .is_err()
        );
        let mut stale = annotation.clone();
        stale.asset.fingerprint.inode += 1;
        assert_ne!(stale.asset, annotation.asset); // Physical freshness is checked by Workflow, not this pure metadata guard.
        stale.asset.fingerprint.sha256[0] ^= 1;
        assert!(stale.candidate_text().is_err());
        let mut changed = annotation;
        changed.source_text = source_text.replacen("Exact before", "Changed before", 1);
        changed.source.fingerprint = fingerprint(&changed.source_text);
        assert!(changed.candidate_text().is_err());
    }

    #[test]
    fn forged_occurrence_and_profile_metadata_do_not_become_visual_evidence() {
        let (binding, converted, _) = fixture();
        let mut forged = binding.clone();
        forged.visual.as_mut().unwrap().image_start += 1;
        assert!(forged.markdown(&converted).is_err());
        forged = binding.clone();
        forged.visual.as_mut().unwrap().part_name = "../image.png".into();
        assert!(forged.markdown(&converted).is_err());
        forged = binding.clone();
        forged.format = InboxConversionFormat::DocxTextV1;
        assert!(forged.markdown(&converted).is_err());
        forged = binding.clone();
        forged.visual = None;
        assert!(forged.markdown(&converted).is_err());
        let text = binding.markdown(&converted).unwrap();
        let extra = text + "unbound trailing wording\n";
        assert!(inbox_source::read_provenance(&extra).is_err());
        // The start/end markers can share one newline in malformed untrusted
        // input. This must refuse rather than slice with an inverted range.
        let overlapped = format!("{converted}{SECTION_START}{}", &SECTION_END[1..]);
        assert!(
            binding
                .visual
                .as_ref()
                .unwrap()
                .validate_source_body(&binding.original.capture.copy.sha256, &overlapped,)
                .is_err()
        );
    }
}
