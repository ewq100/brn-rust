//! Hand-authored, already-converted synthetic material; not a converter demonstration.
use brn_threads_app::*;
use brn_threads_intake::{ConversionResult, Coverage, CoverageStatus, ImportIntent};
use rig::test_utils::{MockCompletionModel, MockStreamEvent};
use serde_json::json;

pub const ORIGINAL: &str = "# Synthetic draft\n\nHello, reader. This explanation is unclear.\n";
pub const REVISED: &str =
    "# Synthetic draft\n\nHello, reader. This explanation is clear and concise.\n";
pub const BRIEF: &str = "Write for a new reader; preserve the greeting and explain clearly.";
pub const QUOTE: &str = "Preserve the greeting.";

pub struct Fixture {
    pub workspace: Workspace,
    pub note: Record,
    pub reference: Record,
    // Last: SQLite/search handles must drop before their owned disposable directory.
    pub root: tempfile::TempDir,
}
impl Fixture {
    pub fn new() -> AppResult<Self> {
        let root = tempfile::Builder::new()
            .prefix("brn-writing-host-")
            .tempdir_in(std::env::temp_dir().canonicalize()?)?;
        let mut workspace = Workspace::open(root.path().join("workspace"))?;
        let mut settings = workspace.settings()?;
        settings.provider = "Codex".into();
        settings.model = "gpt-6.1-sol".into();
        settings.effort = "medium".into();
        settings.auth_file = None;
        settings.credentials_dir = None;
        settings.helper = None;
        workspace.configure(settings)?;
        let note = workspace.new_note("Synthetic writing document", ORIGINAL)?;
        let conversion = ConversionResult {
            intent: ImportIntent::FullNote,
            markdown: format!("# Synthetic requirements\n\n{QUOTE}\nKeep the explanation clear.\n"),
            assets: vec![],
            source_sha256: [84; 32],
            source_reference: "synthetic/b4-requirements.txt".into(),
            converter: "hand-authored synthetic completed conversion".into(),
            sources: vec![],
            coverage: Coverage {
                status: CoverageStatus::Partial,
                checked_items: vec![],
                gaps: vec![],
            },
        };
        let ApplyOutcome::Applied(receipt) = workspace.import_full(
            "b4-synthetic-reference",
            "Synthetic retained requirements (partial original)",
            &conversion,
        )?
        else {
            return Err(AppError::Invalid("Synthetic import did not apply".into()));
        };
        let reference =
            workspace.full_import_note_in_scope(&receipt, &conversion.source_sha256, None)?;
        workspace.set_writing_setup(
            "b4-synthetic-setup",
            None,
            &WritingSetup {
                note: note.id.clone(),
                chat: None,
                brief: Some(BRIEF.into()),
                references: vec![WritingReference {
                    note: reference.id.clone(),
                    purpose: WritingPurpose::Requirements,
                }],
            },
        )?;
        workspace.ensure_writing_context("b4-open-context", &note.id, None)?;
        Ok(Self {
            workspace,
            note,
            reference,
            root,
        })
    }
}

/// A scripted provider uses the real read/prepare tools; it cannot manufacture
/// the runtime's ReadLedger, candidate evidence, authorization or commit result.
pub fn model(snapshot: &WritingSnapshot, reference: &Record) -> AppResult<MockCompletionModel> {
    let RecordData::Note(note) = &reference.data else {
        return Err(AppError::Invalid(
            "Synthetic reference is not a Note".into(),
        ));
    };
    let start = note
        .markdown
        .find(QUOTE)
        .ok_or_else(|| AppError::Invalid("Synthetic quotation missing".into()))?;
    Ok(MockCompletionModel::from_stream_turns([
        vec![
            MockStreamEvent::tool_call(
                "b4-read",
                "read_note",
                json!({"id":reference.id,"version":reference.version,"start":0,"max_bytes":131072,"section":false}),
            ),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
        vec![
            MockStreamEvent::tool_call(
                "b4-propose",
                "prepare_changes",
                json!({"request_key":"b4-scripted-proposal","request":snapshot.change_request(REVISED)?,"writing":{"quotations":[{"id":reference.id,"version":reference.version,"start":start,"end":start+QUOTE.len(),"quote":QUOTE}],"requirements_reported":[{"claim":"Scripted model reports the greeting retained; this is not independent verification","quotations":[0]}]}}),
            ),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
        vec![
            MockStreamEvent::text(
                "Synthetic proposal prepared. Review the exact change; original import coverage remains partial.",
            ),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
    ]))
}
