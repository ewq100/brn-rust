//! Presentation of the existing checked host evidence, not a bibliography or reading policy.
use super::setup::import_label;
use super::{WritingView, metadata, subheading};
use brn_threads_app::{WritingCandidateReview, WritingRead, WritingSnapshot};
use gpui_kit::{TestSupportExt, div, prelude::*};

fn retained_read_label(read: Option<&WritingRead>) -> String {
    match read {
        None => "Not read".into(),
        Some(r) if r.complete && r.covered_retained_bytes == r.total_retained_bytes => format!(
            "Read in full · {} retained bytes at v{}",
            r.total_retained_bytes, r.version
        ),
        Some(r) => format!(
            "Partially read · {} of {} retained bytes at v{}",
            r.covered_retained_bytes, r.total_retained_bytes, r.version
        ),
    }
}
impl<T: 'static> WritingView<'_, T> {
    /// Called only after review_body's existing CW2 checked-context gate.
    pub(crate) fn sources_used(
        &self,
        evidence: &WritingCandidateReview,
        snapshot: Option<&WritingSnapshot>,
    ) -> gpui_kit::AnyElement {
        let mut panel = div().id("writing-sources-used").flex().flex_col().gap_1()
            .child(subheading("Sources used"))
            .child(metadata("Host-recorded reads for this immutable proposal. Attached/available is not read. Reading all retained text does not establish coverage of the original file."));
        let current = self.open_note_id().and_then(|note| {
            self.workspace
                .writing_setup(note, self.displayed_scope)
                .ok()
        });
        for (index, source) in evidence.sources.iter().enumerate() {
            let used = source
                .retained_read
                .as_ref()
                .is_some_and(|r| r.covered_retained_bytes > 0 || r.complete);
            let mut status = format!(
                "{} · v{} · {}",
                source.title,
                source.version,
                if used {
                    "Actually used (host-recorded)"
                } else {
                    "Available at request; not read"
                }
            );
            if source.current_version != source.version {
                status.push_str(&format!(" · Changed since to v{}", source.current_version));
            }
            if let Some(current) = &current {
                let was_attached = snapshot
                    .and_then(|s| s.setup.as_ref())
                    .is_some_and(|s| s.references.iter().any(|r| r.note == source.id));
                if was_attached
                    && !current
                        .references
                        .iter()
                        .any(|r| r.reference.note == source.id)
                {
                    status.push_str(" · Removed from current references; saved evidence unchanged");
                }
            }
            panel = panel.child(
                div()
                    .id(format!("writing-source-used-{index}"))
                    .flex()
                    .flex_col()
                    .aria_label(status.clone())
                    .child(status)
                    .child(
                        div()
                            .id(format!("writing-source-import-{index}"))
                            .aria_label(import_label(source.import_coverage.as_ref()))
                            .child(metadata(import_label(source.import_coverage.as_ref())))
                            .test_support(),
                    )
                    .child(
                        div()
                            .id(format!("writing-source-reading-{index}"))
                            .aria_label(retained_read_label(source.retained_read.as_ref()))
                            .child(metadata(retained_read_label(source.retained_read.as_ref())))
                            .test_support(),
                    )
                    .test_support(),
            );
        }
        panel = panel.child(div().id("writing-requirements-reading").aria_label(if evidence.complete_over_retained_text { "Requirements reading: complete over retained text" } else { "Requirements reading: partial or no exhaustive check requested" })
            .child(if evidence.complete_over_retained_text { "Requirements reading: complete over retained text" } else { "Requirements reading: partial or no exhaustive check requested" }).test_support())
            .child(metadata("Requirements reported as checked are model claims, not proof that all requirements were verified. Exact quotations establish provenance, not support for every generated claim."));
        for (index, quote) in evidence.host_verified_quotations.iter().enumerate() {
            let source_title = evidence
                .sources
                .iter()
                .find(|source| source.id == quote.id && source.version == quote.version)
                .map_or("Checked source title unavailable", |source| {
                    source.title.as_str()
                });
            let provenance = format!(
                "Host-verified quotation · {source_title} v{} · exact UTF-8 bytes {}..{}",
                quote.version, quote.start, quote.end
            );
            panel = panel.child(
                div()
                    .id(format!("writing-source-quotation-{index}"))
                    .aria_label(provenance.clone())
                    .flex()
                    .flex_col()
                    .child(metadata(provenance))
                    .child(quote.quote.clone())
                    .test_support(),
            );
        }
        for (index, claim) in evidence.requirements_reported.iter().enumerate() {
            panel = panel.child(div().id(format!("writing-requirement-claim-{index}")).flex().flex_col()
                .child(format!("Model-reported claim: {}", claim.claim))
                .child(metadata(format!("Host-verified quotation indices: {:?}. This is not independent verification of the claim.", claim.quotations))).test_support());
        }
        panel.test_support().into_any_element()
    }
}
