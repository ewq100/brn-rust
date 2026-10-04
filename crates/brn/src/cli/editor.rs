//! Simple Markdown editing over the shared application worker.
use super::{
    error::CliError, expect_positionals, required_positional, scan, sub_word, usage, CliFailure,
    Globals, Scanned, Tokens,
};
use brn_workflow::{
    app_worker::AppCommand,
    editor::{EditRequest, EditStamp, ReloadRequest, SaveRequest},
};
use std::{path::PathBuf, sync::atomic::Ordering};
use uuid::Uuid;

pub enum EditorCommand {
    Open(String),
    Recover {
        path: String,
        expected: EditStamp,
        generation: u64,
        file: PathBuf,
    },
    Save {
        path: String,
        expected: EditStamp,
        generation: u64,
        file: PathBuf,
        operation: Uuid,
        destination: Option<String>,
    },
    Reload {
        path: String,
        expected: EditStamp,
        observed_file: PathBuf,
        discard: bool,
    },
    List,
    Reconcile(Uuid),
}

impl EditorCommand {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Open(_) => "edit.open",
            Self::Recover { .. } => "edit.recover",
            Self::Save { .. } => "edit.save",
            Self::Reload { .. } => "edit.reload",
            Self::List => "edit.list",
            Self::Reconcile(_) => "edit.reconcile",
        }
    }
}

pub(super) fn scan_command(
    tokens: Tokens<'_>,
    globals: &mut Globals,
    name: &mut Option<&'static str>,
) -> Result<Scanned, CliError> {
    let sub = sub_word(tokens, "edit", "open|recover|save|reload|list|reconcile")?;
    let (label, options): (&str, &[(&str, bool)]) = match sub.as_str() {
        "open" => ("edit.open", &[]),
        "recover" => ("edit.recover", EDIT_OPTIONS),
        "save" => ("edit.save", SAVE_OPTIONS),
        "reload" => (
            "edit.reload",
            &[
                ("baseline", true),
                ("expected-generation", true),
                ("observed-file", true),
                ("discard", false),
            ],
        ),
        "list" => ("edit.list", &[]),
        "reconcile" => ("edit.reconcile", &[]),
        _ => return Err(usage("unknown edit subcommand")),
    };
    *name = Some(label);
    scan(tokens, globals, options)
}

const EDIT_OPTIONS: &[(&str, bool)] = &[
    ("baseline", true),
    ("expected-generation", true),
    ("generation", true),
    ("file", true),
];
const SAVE_OPTIONS: &[(&str, bool)] = &[
    ("baseline", true),
    ("expected-generation", true),
    ("generation", true),
    ("file", true),
    ("operation", true),
    ("copy", true),
];

fn path(raw: &str) -> Result<String, CliError> {
    brn_workflow::vault::VaultPath::parse(raw).map_err(|error| usage(error.to_string()))?;
    Ok(raw.to_owned())
}

pub(super) fn parse_command(name: &str, s: &Scanned) -> Result<EditorCommand, CliError> {
    expect_positionals(s, usize::from(name != "edit.list"))?;
    match name {
        "edit.open" => Ok(EditorCommand::Open(path(required_positional(s, "PATH")?)?)),
        "edit.list" => Ok(EditorCommand::List),
        "edit.reload" => Ok(EditorCommand::Reload {
            path: path(required_positional(s, "PATH")?)?,
            expected: EditStamp {
                baseline: s.require_uuid("baseline")?,
                generation: s.require_generation("expected-generation")?,
            },
            observed_file: PathBuf::from(
                s.value("observed-file")
                    .ok_or_else(|| usage("missing --observed-file"))?,
            ),
            discard: s.flag("discard"),
        }),
        "edit.reconcile" => Ok(EditorCommand::Reconcile(super::positional_uuid(
            s,
            0,
            "OPERATION",
        )?)),
        "edit.recover" | "edit.save" => {
            let path = path(required_positional(s, "PATH")?)?;
            let expected = EditStamp {
                baseline: s.require_uuid("baseline")?,
                generation: s.require_generation("expected-generation")?,
            };
            let generation = s.require_generation("generation")?;
            if generation < expected.generation {
                return Err(usage("--generation must be at least --expected-generation"));
            }
            let file = PathBuf::from(s.value("file").ok_or_else(|| usage("missing --file"))?);
            if name == "edit.save" {
                Ok(EditorCommand::Save {
                    path,
                    expected,
                    generation,
                    file,
                    operation: s.require_uuid("operation")?,
                    destination: s.value("copy").map(self::path).transpose()?,
                })
            } else {
                Ok(EditorCommand::Recover {
                    path,
                    expected,
                    generation,
                    file,
                })
            }
        }
        _ => unreachable!("scanned editor command"),
    }
}

/// Prepare exact input before opening any workspace. Save's admission id is
/// the same UUID as its durable request, including on explicit replay.
pub(super) fn prepare(command: &EditorCommand) -> Result<(Uuid, AppCommand), CliFailure> {
    let id = match command {
        EditorCommand::Save { operation, .. } => *operation,
        _ => Uuid::new_v4(),
    };
    let request = match command {
        EditorCommand::Open(path) => AppCommand::OpenEditor(path.clone()),
        EditorCommand::List => AppCommand::Editors,
        EditorCommand::Reconcile(operation) => AppCommand::ReconcileEditor(*operation),
        EditorCommand::Reload {
            path,
            expected,
            observed_file,
            discard,
        } => AppCommand::ReloadEditor(ReloadRequest {
            path: path.clone(),
            expected: *expected,
            observed: serde_json::from_str(&super::input::read_text_file(
                observed_file,
                "observed fingerprint",
            )?)
            .map_err(|_| {
                usage("--observed-file must contain a JSON file fingerprint from edit open")
            })?,
            discard: *discard,
        }),
        EditorCommand::Recover {
            path,
            expected,
            generation,
            file,
        } => AppCommand::RecoverEditor(EditRequest {
            path: path.clone(),
            expected: *expected,
            generation: *generation,
            text: super::input::read_text_file(file, "editor")?,
        }),
        EditorCommand::Save {
            path,
            expected,
            generation,
            file,
            operation,
            destination,
        } => AppCommand::SaveEditor(SaveRequest {
            operation_id: *operation,
            edit: EditRequest {
                path: path.clone(),
                expected: *expected,
                generation: *generation,
                text: super::input::read_text_file(file, "editor")?,
            },
            destination: destination.clone(),
        }),
    };
    if crate::CANCEL.load(Ordering::SeqCst) {
        return Err(CliError::Interrupted(
            "interrupted during input preparation; the command was not run".into(),
        )
        .into());
    }
    Ok((id, request))
}
