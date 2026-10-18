//! Document choices use checked canonical settings; report labels use saved provenance.
use super::*;

fn converter_label(converter: DocumentConverter) -> &'static str {
    match converter {
        DocumentConverter::Xberg => "Xberg",
        DocumentConverter::XbergBrn => "Xberg + BRN layer",
    }
}

pub(super) fn report_details(report: &ImportReport) -> Vec<String> {
    let mut details = Vec::new();
    if let Some(conversion) = &report.conversion {
        details.push(format!(
            "{} · AI conversion: {}",
            converter_label(conversion.settings.converter),
            if conversion.settings.ai {
                "On (GPT-6 Luna, high)"
            } else {
                "Off"
            },
        ));
    }
    details.extend(report.issues.iter().map(ImportIssue::display));
    details
}

pub(super) fn report_summary(report: &ImportReport) -> String {
    match &report.conversion {
        Some(conversion) => format!(
            "{} · {} · AI conversion: {}",
            report.summary(),
            converter_label(conversion.settings.converter),
            if conversion.settings.ai { "On" } else { "Off" },
        ),
        None => report.summary().into(),
    }
}

impl ThreadsUi {
    pub(super) fn choose_document_conversion(
        &mut self,
        converter: Option<DocumentConverter>,
        ai: Option<bool>,
        cx: &mut Context<Self>,
    ) {
        // Read once here so a different host's change to the other choice is retained.
        let result = self.workspace.settings().and_then(|settings| {
            let mut documents = settings.document_conversion;
            if let Some(converter) = converter {
                documents.converter = converter;
            }
            if let Some(ai) = ai {
                documents.ai = ai;
            }
            self.workspace.choose_document_conversion(documents)
        });
        match result {
            Ok(outcome) => self.report_outcome(&outcome),
            Err(error) => self.fail(error.to_string()),
        }
        // Deferred settings remain unchanged until checked application succeeds.
        self.refresh();
        cx.notify();
    }

    pub(super) fn documents_settings(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let selected = &self.settings.document_conversion;
        div().id("documents-settings").flex().flex_col().gap_3().max_w(px(640.))
            .child(components::section_header("Converter", None, None))
            .child(metadata("Applies to future document imports and attachments. Existing notes are unchanged."))
            .child(div().flex().flex_wrap().gap_2()
                .child(button("documents-xberg").label("Xberg (default)").outline()
                    .selected(selected.converter == DocumentConverter::Xberg)
                    .on_click(cx.listener(|this, _, _, cx| this.choose_document_conversion(Some(DocumentConverter::Xberg), None, cx))))
                .child(button("documents-xberg-brn").label("Xberg + BRN layer").outline()
                    .selected(selected.converter == DocumentConverter::XbergBrn)
                    .on_click(cx.listener(|this, _, _, cx| this.choose_document_conversion(Some(DocumentConverter::XbergBrn), None, cx)))))
            .child(components::section_header("AI conversion", None, None))
            .child(metadata("Optional GPT-6 Luna refinement at high effort, with either converter. Turning it off does not disable the useful-information agent."))
            .child(div().flex().gap_2()
                .child(button("documents-ai-off").label("Off (default)").outline().selected(!selected.ai)
                    .on_click(cx.listener(|this, _, _, cx| this.choose_document_conversion(None, Some(false), cx))))
                .child(button("documents-ai-on").label("On").outline().selected(selected.ai)
                    .on_click(cx.listener(|this, _, _, cx| this.choose_document_conversion(None, Some(true), cx)))))
            .test_support().into_any_element()
    }
}
