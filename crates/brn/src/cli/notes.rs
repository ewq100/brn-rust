//! Headless note adapter: explicit caller preconditions, exact input and typed outcomes.
use super::error::CliError;
use super::{
    expect_positionals, positional_uuid, required_positional, scan, sub_word, usage, CliFailure,
    Globals, Invocation, Output, Scanned, Tokens,
};
use brn_workflow::notes::{NoteErrorCode, NoteFailure, NoteStamp, NoteSubmission};
use serde_json::json;
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

pub enum WriteKind {
    Buffer,
    Save,
    Copy(PathBuf),
}

pub enum NoteCommand {
    Open {
        path: PathBuf,
        vault: PathBuf,
        operation: Option<Uuid>,
    },
    Show(Uuid),
    Write {
        kind: WriteKind,
        note: Uuid,
        expected: NoteStamp,
        generation: u64,
        text_file: PathBuf,
        operation: Option<Uuid>,
    },
    RecoveryList,
    RecoveryShow(Uuid),
    Reconcile(Uuid),
    AcceptCurrent {
        save_operation: Uuid,
        file_state: Uuid,
        operation: Option<Uuid>,
    },
    Compare(Uuid),
    Reload {
        note: Uuid,
        expected: NoteStamp,
        operation: Option<Uuid>,
    },
    Relink {
        note: Uuid,
        expected: NoteStamp,
        path: PathBuf,
        operation: Option<Uuid>,
    },
    Approve {
        note: Uuid,
        file_state: Uuid,
        operation: Option<Uuid>,
    },
}

impl NoteCommand {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Open { .. } => "notes.open",
            Self::Show(_) => "notes.show",
            Self::Write {
                kind: WriteKind::Buffer,
                ..
            } => "notes.buffer.save",
            Self::Write {
                kind: WriteKind::Save,
                ..
            } => "notes.save",
            Self::Write {
                kind: WriteKind::Copy(_),
                ..
            } => "notes.save-copy",
            Self::RecoveryList => "notes.recovery.list",
            Self::RecoveryShow(_) => "notes.recovery.show",
            Self::Reconcile(_) => "notes.recovery.reconcile",
            Self::AcceptCurrent { .. } => "notes.recovery.accept-current",
            Self::Compare(_) => "notes.compare",
            Self::Reload { .. } => "notes.reload",
            Self::Relink { .. } => "notes.relink",
            Self::Approve { .. } => "notes.approve-for-search",
        }
    }
}

pub(super) fn scan_command(
    tokens: Tokens<'_>,
    globals: &mut Globals,
    name: &mut Option<&'static str>,
) -> Result<Scanned, CliError> {
    let sub = sub_word(
        tokens,
        "notes",
        "open|show|buffer|save|recovery|compare|reload|relink|save-copy|approve-for-search",
    )?;
    let (label, options): (&str, &[(&str, bool)]) = match sub.as_str() {
        "open" => ("notes.open", &[("vault", true), ("operation", true)]),
        "show" => ("notes.show", &[]),
        "compare" => ("notes.compare", &[]),
        "buffer" => {
            let action = sub_word(tokens, "notes buffer", "save")?;
            if action != "save" {
                return Err(usage("unknown notes buffer subcommand"));
            }
            ("notes.buffer.save", WRITE_OPTIONS)
        }
        "save" => ("notes.save", WRITE_OPTIONS),
        "save-copy" => ("notes.save-copy", COPY_OPTIONS),
        "reload" => (
            "notes.reload",
            &[
                ("base-file-state", true),
                ("expected-generation", true),
                ("discard-local-edits", false),
                ("operation", true),
            ],
        ),
        "relink" => (
            "notes.relink",
            &[
                ("base-file-state", true),
                ("expected-generation", true),
                ("path", true),
                ("confirm-identity", false),
                ("operation", true),
            ],
        ),
        "approve-for-search" => (
            "notes.approve-for-search",
            &[("file-state", true), ("operation", true)],
        ),
        "recovery" => {
            let action = sub_word(
                tokens,
                "notes recovery",
                "list|show|reconcile|accept-current",
            )?;
            match action.as_str() {
                "list" => ("notes.recovery.list", &[]),
                "show" => ("notes.recovery.show", &[]),
                "reconcile" => ("notes.recovery.reconcile", &[("operation", true)]),
                "accept-current" => (
                    "notes.recovery.accept-current",
                    &[
                        ("save-operation", true),
                        ("file-state", true),
                        ("keep-recovery", false),
                        ("operation", true),
                    ],
                ),
                _ => return Err(usage("unknown notes recovery subcommand")),
            }
        }
        _ => return Err(usage("unknown notes subcommand")),
    };
    *name = Some(label);
    scan(tokens, globals, options)
}

const WRITE_OPTIONS: &[(&str, bool)] = &[
    ("base-file-state", true),
    ("expected-generation", true),
    ("generation", true),
    ("text-file", true),
    ("operation", true),
];
const COPY_OPTIONS: &[(&str, bool)] = &[
    ("base-file-state", true),
    ("expected-generation", true),
    ("generation", true),
    ("text-file", true),
    ("operation", true),
    ("path", true),
];

fn value_path(s: &Scanned, name: &str) -> Result<PathBuf, CliError> {
    s.value(name)
        .map(PathBuf::from)
        .ok_or_else(|| usage(format!("missing --{name}")))
}

fn relative_path(s: &Scanned) -> Result<PathBuf, CliError> {
    let path = value_path(s, "path")?;
    validate_relative(&path)?;
    Ok(path)
}

fn validate_relative(path: &Path) -> Result<(), CliError> {
    if path
        .to_str()
        .is_none_or(|p| p.split('/').any(|c| c.is_empty() || c == "." || c == ".."))
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        || path.extension().and_then(|e| e.to_str()) != Some("md")
    {
        return Err(usage(
            "note path must be a contained vault-relative .md path",
        ));
    }
    Ok(())
}

fn stamp(s: &Scanned) -> Result<NoteStamp, CliError> {
    Ok(NoteStamp {
        file_state: s.require_uuid("base-file-state")?,
        generation: s.require_generation("expected-generation")?,
    })
}

fn require_flag(s: &Scanned, flag: &str) -> Result<(), CliError> {
    if !s.flag(flag) {
        return Err(usage(format!("missing --{flag}")));
    }
    Ok(())
}

pub(super) fn parse_command(name: &str, s: &Scanned) -> Result<NoteCommand, CliError> {
    use NoteCommand::*;
    let count = match name {
        "notes.recovery.list" | "notes.recovery.reconcile" | "notes.recovery.accept-current" => 0,
        _ => 1,
    };
    expect_positionals(s, count)?;
    let operation = s.uuid("operation")?;
    let note = || positional_uuid(s, 0, "NOTE_ID");
    Ok(match name {
        "notes.open" => {
            let path = PathBuf::from(required_positional(s, "PATH")?);
            let vault = value_path(s, "vault")?;
            if !path.is_absolute() || !vault.is_absolute() {
                return Err(usage("notes open PATH and --vault must be absolute paths"));
            }
            let relative = path
                .strip_prefix(&vault)
                .map_err(|_| usage("PATH must be contained in --vault"))?;
            validate_relative(relative)?;
            Open {
                path,
                vault,
                operation,
            }
        }
        "notes.show" => Show(note()?),
        "notes.compare" => Compare(note()?),
        "notes.recovery.list" => RecoveryList,
        "notes.recovery.show" => RecoveryShow(note()?),
        "notes.recovery.reconcile" => Reconcile(s.require_uuid("operation")?),
        "notes.recovery.accept-current" => {
            require_flag(s, "keep-recovery")?;
            AcceptCurrent {
                save_operation: s.require_uuid("save-operation")?,
                file_state: s.require_uuid("file-state")?,
                operation,
            }
        }
        "notes.reload" => {
            require_flag(s, "discard-local-edits")?;
            Reload {
                note: note()?,
                expected: stamp(s)?,
                operation,
            }
        }
        "notes.relink" => {
            require_flag(s, "confirm-identity")?;
            Relink {
                note: note()?,
                expected: stamp(s)?,
                path: relative_path(s)?,
                operation,
            }
        }
        "notes.approve-for-search" => Approve {
            note: note()?,
            file_state: s.require_uuid("file-state")?,
            operation,
        },
        "notes.buffer.save" | "notes.save" | "notes.save-copy" => Write {
            kind: match name {
                "notes.buffer.save" => WriteKind::Buffer,
                "notes.save" => WriteKind::Save,
                _ => WriteKind::Copy(relative_path(s)?),
            },
            note: note()?,
            expected: stamp(s)?,
            generation: s.require_generation("generation")?,
            text_file: value_path(s, "text-file")?,
            operation,
        },
        _ => unreachable!("validated note command"),
    })
}

pub fn note_cli_failure(failure: NoteFailure) -> CliFailure {
    let context = json!({
        "operation_id": failure.operation_id,
        "note_id": failure.note_id,
        "phase": failure.phase,
        "filesystem_outcome": failure.filesystem_outcome,
        "recovery_available": failure.recovery_available,
    });
    let message = failure.message;
    let error = match failure.code {
        NoteErrorCode::StateChanged => CliError::NoteStateChanged(message),
        NoteErrorCode::Conflict => CliError::NoteConflict(message),
        NoteErrorCode::Missing => CliError::NoteMissing(message),
        NoteErrorCode::Unsupported => CliError::NoteUnsupported(message),
        NoteErrorCode::SaveUncertain => CliError::NoteSaveUncertain(message),
        NoteErrorCode::Io => CliError::NoteIo(message),
        NoteErrorCode::Storage => CliError::NoteStorage(message),
        NoteErrorCode::VaultBusy => CliError::VaultBusy(message),
        NoteErrorCode::VaultUnavailable => CliError::VaultUnavailable(message),
        NoteErrorCode::OperationConflict => CliError::OperationConflict(message),
        NoteErrorCode::WorkspaceBusy => CliError::WorkspaceBusy(message),
    };
    CliFailure {
        error,
        context: Some(context),
    }
}

pub fn run(invocation: &Invocation, command: &NoteCommand) -> Result<Output, CliFailure> {
    use NoteCommand::*;
    let text = match command {
        Write { text_file, .. } => Some(super::input::read_text_file(text_file, "note")?),
        _ => None,
    };
    let mut workspace = super::input::prepared_workspace(invocation)?;
    let op = |operation: &Option<Uuid>| operation.unwrap_or_else(Uuid::new_v4);
    let data = match command {
        Open {
            path,
            vault,
            operation,
        } => {
            let operation = op(operation);
            view_output(
                workspace
                    .open_note(
                        operation,
                        vault,
                        path.strip_prefix(vault).expect("validated relative path"),
                    )
                    .map_err(note_cli_failure)?,
                operation,
            )
        }
        Show(id) => json!(workspace.note(*id).map_err(note_cli_failure)?),
        Compare(id) => json!(workspace.compare_note(*id).map_err(note_cli_failure)?),
        RecoveryList => json!(workspace.note_recoveries().map_err(note_cli_failure)?),
        RecoveryShow(id) => json!(workspace
            .note_recovery(*id)
            .map_err(note_cli_failure)?
            .ok_or_else(|| note_cli_failure(NoteFailure {
                code: NoteErrorCode::Missing,
                message: "note recovery is absent".into(),
                operation_id: None,
                note_id: Some(*id),
                phase: None,
                filesystem_outcome: brn_workflow::notes::FileOutcome::NotApplied,
                recovery_available: false,
            }))?),
        Reconcile(operation) => json!(workspace
            .reconcile_note_save(*operation)
            .map_err(note_cli_failure)?),
        AcceptCurrent {
            save_operation,
            file_state,
            operation,
        } => {
            let operation = op(operation);
            let mut data = view_output(
                workspace
                    .accept_note_disk_state(operation, *save_operation, *file_state)
                    .map_err(note_cli_failure)?,
                operation,
            );
            data["save_operation_id"] = json!(save_operation);
            data
        }
        Reload {
            note,
            expected,
            operation,
        } => {
            let operation = op(operation);
            view_output(
                workspace
                    .reload_note(operation, *note, *expected, true)
                    .map_err(note_cli_failure)?,
                operation,
            )
        }
        Relink {
            note,
            expected,
            path,
            operation,
        } => {
            let operation = op(operation);
            view_output(
                workspace
                    .relink_note(operation, *note, *expected, path, true)
                    .map_err(note_cli_failure)?,
                operation,
            )
        }
        Approve {
            note,
            file_state,
            operation,
        } => json!(workspace
            .approve_note_snapshot(op(operation), *note, *file_state)
            .map_err(note_cli_failure)?),
        Write {
            kind,
            note,
            expected,
            generation,
            operation,
            ..
        } => {
            let request = NoteSubmission {
                operation_id: op(operation),
                note_id: *note,
                expected: *expected,
                generation: *generation,
                text: text.expect("prepared exact text"),
            };
            match kind {
                WriteKind::Buffer => json!(workspace
                    .save_note_buffer(request)
                    .map_err(note_cli_failure)?),
                WriteKind::Save => json!(workspace.save_note(request).map_err(note_cli_failure)?),
                WriteKind::Copy(path) => json!(workspace
                    .save_note_copy(request, path)
                    .map_err(note_cli_failure)?),
            }
        }
    };
    Ok(Output {
        text: format!(
            "{}: {}\n",
            command.name(),
            serde_json::to_string_pretty(&data).expect("note output serializes")
        ),
        data,
    })
}

fn view_output(view: brn_workflow::notes::NoteView, operation: Uuid) -> serde_json::Value {
    let mut data = json!(view);
    data["operation_id"] = json!(operation);
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use brn_workflow::notes::{FileOutcome, SavePhase};

    #[test]
    fn all_typed_failures_preserve_context_without_classifying_wording() {
        let operation = Uuid::new_v4();
        let note = Uuid::new_v4();
        for (code, expected) in [
            (NoteErrorCode::StateChanged, "NOTE_STATE_CHANGED"),
            (NoteErrorCode::Conflict, "NOTE_CONFLICT"),
            (NoteErrorCode::Missing, "NOTE_MISSING"),
            (NoteErrorCode::Unsupported, "NOTE_UNSUPPORTED"),
            (NoteErrorCode::SaveUncertain, "NOTE_SAVE_UNCERTAIN"),
            (NoteErrorCode::Io, "NOTE_IO_ERROR"),
            (NoteErrorCode::Storage, "NOTE_STORAGE_ERROR"),
            (NoteErrorCode::VaultBusy, "VAULT_BUSY"),
            (NoteErrorCode::VaultUnavailable, "VAULT_UNAVAILABLE"),
            (NoteErrorCode::OperationConflict, "OPERATION_CONFLICT"),
            (NoteErrorCode::WorkspaceBusy, "WORKSPACE_BUSY"),
        ] {
            for phase in [
                None,
                Some(SavePhase::Intent),
                Some(SavePhase::Prepared),
                Some(SavePhase::Exchanged),
                Some(SavePhase::Verified),
                Some(SavePhase::Complete),
            ] {
                for outcome in [
                    FileOutcome::NotApplied,
                    FileOutcome::Applied,
                    FileOutcome::Unknown,
                ] {
                    for recovery in [true, false] {
                        let mapped = note_cli_failure(NoteFailure {
                            code,
                            message: "unrelated wording: missing conflict busy".into(),
                            operation_id: Some(operation),
                            note_id: Some(note),
                            phase,
                            filesystem_outcome: outcome,
                            recovery_available: recovery,
                        });
                        assert_eq!(mapped.error.code(), expected);
                        assert_eq!(mapped.error.exit_code(), 1);
                        assert_eq!(
                            mapped.error.message(),
                            "unrelated wording: missing conflict busy"
                        );
                        let envelope = super::super::envelope_err(
                            Some("notes.save"),
                            &mapped.error,
                            mapped.context.as_ref(),
                        );
                        let envelope: serde_json::Value = serde_json::from_str(&envelope).unwrap();
                        assert_eq!(
                            envelope["error"]["context"],
                            json!({
                                "operation_id": operation, "note_id": note, "phase": phase,
                                "filesystem_outcome": outcome, "recovery_available": recovery,
                            })
                        );
                        assert!(envelope.get("data").is_none());
                    }
                }
            }
        }
    }
}
