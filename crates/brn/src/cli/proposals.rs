//! Headless typed review and explicit approval over the shared worker.
use super::{
    error::CliError, expect_positionals, required_positional, scan, sub_word, usage, CliFailure,
    Globals, Scanned, Tokens,
};
use brn_workflow::{
    app_worker::AppCommand,
    proposal_apply::{
        validate_approval_request, validate_repair_request, validate_undo_request, ApprovalRequest,
        GroupApprovalRequest, RepairRequest, UndoRequest,
    },
    proposal_rewrite::RewriteRequest,
    proposals::{CommentRequest, DraftRequest, ProposalEdit, ProposalStamp},
};
use serde::de::DeserializeOwned;
use std::{
    fs::File, io::Read, os::unix::fs::OpenOptionsExt, path::PathBuf, sync::atomic::Ordering,
};
use uuid::Uuid;

pub enum ProposalCommand {
    Source(String),
    Asset(String),
    Create(PathBuf),
    List(Option<Uuid>),
    Show(Uuid),
    Edit(PathBuf),
    Rewrite(PathBuf),
    RewriteStatus(Uuid),
    RewriteResult(PathBuf),
    Comment(PathBuf),
    CommentUpdate(PathBuf),
    CommentRemove {
        expected: ProposalStamp,
        comment: Uuid,
    },
    Reject(ProposalStamp),
    Approve(ApprovalRequest),
    Reconcile(Uuid),
    ApproveGroup(PathBuf),
    Applies,
    UndoPreview(UndoRequest),
    Undo(UndoRequest),
    RestoreTrash(UndoRequest),
    RepairPreview(Uuid),
    Repair(PathBuf),
}

impl ProposalCommand {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Source(_) => "proposals.source",
            Self::Asset(_) => "proposals.asset",
            Self::Create(_) => "proposals.create",
            Self::List(_) => "proposals.list",
            Self::Show(_) => "proposals.show",
            Self::Edit(_) => "proposals.edit",
            Self::Rewrite(_) => "proposals.rewrite",
            Self::RewriteStatus(_) => "proposals.rewrite-status",
            Self::RewriteResult(_) => "proposals.rewrite-result",
            Self::Comment(_) => "proposals.comment",
            Self::CommentUpdate(_) => "proposals.comment-update",
            Self::CommentRemove { .. } => "proposals.comment-remove",
            Self::Reject(_) => "proposals.reject",
            Self::Approve(_) => "proposals.approve",
            Self::Reconcile(_) => "proposals.reconcile",
            Self::ApproveGroup(_) => "proposals.approve-group",
            Self::Applies => "proposals.applies",
            Self::UndoPreview(_) => "proposals.undo-preview",
            Self::Undo(_) => "proposals.undo",
            Self::RestoreTrash(_) => "proposals.restore-trash",
            Self::RepairPreview(_) => "proposals.repair-preview",
            Self::Repair(_) => "proposals.repair",
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
        "proposals",
        "source|asset|create|list|show|edit|rewrite|rewrite-status|rewrite-result|comment|comment-update|comment-remove|reject|approve|reconcile|approve-group|applies|undo-preview|undo|restore-trash|repair-preview|repair",
    )?;
    let (label, options): (_, &[(&str, bool)]) = match sub.as_str() {
        "source" => ("proposals.source", &[]),
        "asset" => ("proposals.asset", &[]),
        "create" => ("proposals.create", &[("file", true)]),
        "list" => ("proposals.list", &[("group", true)]),
        "show" => ("proposals.show", &[]),
        "edit" => ("proposals.edit", &[("file", true)]),
        "rewrite" => ("proposals.rewrite", &[("file", true)]),
        "rewrite-status" => ("proposals.rewrite-status", &[]),
        "rewrite-result" => ("proposals.rewrite-result", &[("file", true)]),
        "comment" => ("proposals.comment", &[("file", true)]),
        "comment-update" => ("proposals.comment-update", &[("file", true)]),
        "comment-remove" => (
            "proposals.comment-remove",
            &[("review-version", true), ("comment", true)],
        ),
        "reject" => ("proposals.reject", &[("review-version", true)]),
        "approve" => (
            "proposals.approve",
            &[("review-version", true), ("operation", true)],
        ),
        "reconcile" => ("proposals.reconcile", &[]),
        "approve-group" => ("proposals.approve-group", &[("file", true)]),
        "applies" => ("proposals.applies", &[]),
        "undo-preview" => (
            "proposals.undo-preview",
            &[("operation", true), ("member", true)],
        ),
        "undo" => ("proposals.undo", &[("operation", true)]),
        "restore-trash" => (
            "proposals.restore-trash",
            &[("member", true), ("operation", true)],
        ),
        "repair-preview" => ("proposals.repair-preview", &[]),
        "repair" => ("proposals.repair", &[("file", true)]),
        _ => return Err(usage("unknown proposals subcommand")),
    };
    *name = Some(label);
    scan(tokens, globals, options)
}

pub(super) fn parse_command(name: &str, s: &Scanned) -> Result<ProposalCommand, CliError> {
    let positional = matches!(
        name,
        "proposals.source"
            | "proposals.asset"
            | "proposals.show"
            | "proposals.rewrite-status"
            | "proposals.comment-remove"
            | "proposals.reject"
            | "proposals.approve"
            | "proposals.reconcile"
            | "proposals.undo-preview"
            | "proposals.undo"
            | "proposals.restore-trash"
            | "proposals.repair-preview"
    );
    expect_positionals(s, usize::from(positional))?;
    let file = || {
        s.value("file")
            .map(PathBuf::from)
            .ok_or_else(|| usage("missing --file"))
    };
    let id = || {
        Uuid::parse_str(required_positional(s, "PROPOSAL_ID")?)
            .map_err(|_| usage("invalid proposal UUID"))
    };
    let stamp = || {
        let expected = ProposalStamp {
            id: id()?,
            version: s.require_generation("review-version")?,
        };
        if expected.id.is_nil() || expected.version == 0 {
            return Err(usage("proposal ID and review version must be nonzero"));
        }
        Ok(expected)
    };
    match name {
        "proposals.source" => {
            let path = required_positional(s, "PATH")?;
            brn_workflow::vault::EvidencePath::parse(path).map_err(|e| usage(e.to_string()))?;
            Ok(ProposalCommand::Source(path.to_owned()))
        }
        "proposals.asset" => {
            let path = required_positional(s, "PATH")?;
            brn_workflow::proposals::validate_asset_path(path)
                .map_err(|error| usage(error.message))?;
            Ok(ProposalCommand::Asset(path.to_owned()))
        }
        "proposals.create" => Ok(ProposalCommand::Create(file()?)),
        "proposals.list" => Ok(ProposalCommand::List(s.uuid("group")?)),
        "proposals.show" => Ok(ProposalCommand::Show(id()?)),
        "proposals.edit" => Ok(ProposalCommand::Edit(file()?)),
        "proposals.rewrite" => Ok(ProposalCommand::Rewrite(file()?)),
        "proposals.rewrite-status" => {
            let id = Uuid::parse_str(required_positional(s, "JOB_UUID")?)
                .map_err(|_| usage("invalid Rewrite job UUID"))?;
            if id.is_nil() {
                return Err(usage("Rewrite job UUID must be nonzero"));
            }
            Ok(ProposalCommand::RewriteStatus(id))
        }
        "proposals.rewrite-result" => Ok(ProposalCommand::RewriteResult(file()?)),
        "proposals.comment" => Ok(ProposalCommand::Comment(file()?)),
        "proposals.comment-update" => Ok(ProposalCommand::CommentUpdate(file()?)),
        "proposals.comment-remove" => Ok(ProposalCommand::CommentRemove {
            expected: stamp()?,
            comment: s.require_uuid("comment")?,
        }),
        "proposals.reject" => Ok(ProposalCommand::Reject(stamp()?)),
        "proposals.approve" => {
            let operation_id = s.require_uuid("operation")?;
            if operation_id.is_nil() {
                return Err(usage("approval operation UUID must be nonzero"));
            }
            Ok(ProposalCommand::Approve(ApprovalRequest {
                operation_id,
                expected: stamp()?,
            }))
        }
        "proposals.reconcile" | "proposals.repair-preview" => {
            let operation_id = Uuid::parse_str(required_positional(s, "OPERATION_UUID")?)
                .map_err(|_| usage("invalid approval operation UUID"))?;
            if operation_id.is_nil() {
                return Err(usage("approval operation UUID must be nonzero"));
            }
            Ok(if name == "proposals.reconcile" {
                ProposalCommand::Reconcile(operation_id)
            } else {
                ProposalCommand::RepairPreview(operation_id)
            })
        }
        "proposals.repair" => Ok(ProposalCommand::Repair(file()?)),
        "proposals.approve-group" => Ok(ProposalCommand::ApproveGroup(file()?)),
        "proposals.applies" => Ok(ProposalCommand::Applies),
        "proposals.undo-preview" | "proposals.undo" | "proposals.restore-trash" => {
            let target_operation_id =
                Uuid::parse_str(required_positional(s, "TARGET_OPERATION_UUID")?)
                    .map_err(|_| usage("invalid Undo source operation UUID"))?;
            let trash_member = s
                .value("member")
                .map(|value| {
                    value
                        .parse::<usize>()
                        .map_err(|_| usage("invalid zero-based --member index"))
                })
                .transpose()?;
            if name == "proposals.restore-trash" && trash_member.is_none() {
                return Err(usage("restore-trash requires --member"));
            }
            let request = UndoRequest {
                operation_id: s.require_uuid("operation")?,
                target_operation_id,
                trash_member,
            };
            validate_undo_request(&request).map_err(|error| usage(error.message))?;
            Ok(match name {
                "proposals.undo-preview" => ProposalCommand::UndoPreview(request),
                "proposals.undo" => ProposalCommand::Undo(request),
                _ => ProposalCommand::RestoreTrash(request),
            })
        }
        _ => unreachable!("scanned proposal command"),
    }
}

pub(super) fn input<T: DeserializeOwned>(path: &PathBuf) -> Result<T, CliError> {
    // Keep the existing envelope cap for escaped text and canonical asset
    // base64. Separate domain/payload limits are checked after decoding.
    const MAX_JSON_BYTES: usize = brn_workflow::proposals::MAX_PROPOSAL_BYTES * 8;
    let io = |e: std::io::Error| CliError::Workflow(e.to_string());
    // Reject non-regular inputs from the opened descriptor without blocking on
    // a writerless FIFO during pre-admission preparation. Regular files ignore
    // O_NONBLOCK and retain the exact bounded input contract.
    let mut file = File::options()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
        .map_err(io)?;
    let meta = file.metadata().map_err(io)?;
    if !meta.is_file() || meta.len() > MAX_JSON_BYTES as u64 {
        return Err(usage(
            "proposal input requires a regular JSON file up to 64 MiB",
        ));
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((MAX_JSON_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(io)?;
    if bytes.len() > MAX_JSON_BYTES {
        return Err(usage("proposal JSON exceeds 64 MiB"));
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| usage("proposal input does not match its typed JSON schema"))
}

fn prepare_input(command: &ProposalCommand) -> Result<(Uuid, AppCommand), CliFailure> {
    let mut operation = match command {
        ProposalCommand::Approve(request) => request.operation_id,
        ProposalCommand::UndoPreview(request)
        | ProposalCommand::Undo(request)
        | ProposalCommand::RestoreTrash(request) => request.operation_id,
        ProposalCommand::Reconcile(id) => *id,
        _ => Uuid::new_v4(),
    };
    let command = match command {
        ProposalCommand::Source(path) => {
            brn_workflow::vault::EvidencePath::parse(path).map_err(|e| usage(e.to_string()))?;
            AppCommand::ProposalEvidenceSource(path.clone())
        }
        ProposalCommand::Asset(path) => {
            brn_workflow::proposals::validate_asset_path(path)
                .map_err(super::error::classify_workflow)?;
            AppCommand::ProposalAsset(path.clone())
        }
        ProposalCommand::Create(file) => {
            let request: DraftRequest = input(file)?;
            request
                .validate()
                .map_err(super::error::classify_workflow)?;
            AppCommand::CreateProposal(request)
        }
        ProposalCommand::List(group) => AppCommand::Proposals(*group),
        ProposalCommand::Show(id) => AppCommand::Proposal(*id),
        ProposalCommand::Edit(file) => AppCommand::EditProposal(input::<ProposalEdit>(file)?),
        ProposalCommand::Rewrite(file) => {
            let request: RewriteRequest = input(file)?;
            request
                .validate()
                .map_err(super::error::classify_workflow)?;
            operation = request.id;
            AppCommand::StartProposalRewrite(request)
        }
        ProposalCommand::RewriteStatus(id) => AppCommand::ProposalRewrite(*id),
        ProposalCommand::RewriteResult(file) => {
            AppCommand::RewriteProposal(input::<ProposalEdit>(file)?)
        }
        ProposalCommand::Comment(file) => {
            AppCommand::AddProposalComment(input::<CommentRequest>(file)?)
        }
        ProposalCommand::CommentUpdate(file) => {
            AppCommand::UpdateProposalComment(input::<CommentRequest>(file)?)
        }
        ProposalCommand::CommentRemove { expected, comment } => AppCommand::RemoveProposalComment {
            expected: *expected,
            comment: *comment,
        },
        ProposalCommand::Reject(expected) => AppCommand::RejectProposal(*expected),
        ProposalCommand::Approve(request) => {
            validate_approval_request(request).map_err(super::error::classify_workflow)?;
            AppCommand::ApproveProposal(request.clone())
        }
        ProposalCommand::Reconcile(id) => AppCommand::ReconcileProposal(*id),
        ProposalCommand::ApproveGroup(file) => {
            let request: GroupApprovalRequest = input(file)?;
            request
                .validate()
                .map_err(super::error::classify_workflow)?;
            AppCommand::ApproveProposalGroup(request)
        }
        ProposalCommand::Applies => AppCommand::ProposalApplies,
        ProposalCommand::UndoPreview(request) => {
            validate_undo_request(request).map_err(super::error::classify_workflow)?;
            AppCommand::PreviewProposalUndo(request.clone())
        }
        ProposalCommand::Undo(request) | ProposalCommand::RestoreTrash(request) => {
            validate_undo_request(request).map_err(super::error::classify_workflow)?;
            AppCommand::UndoProposal(request.clone())
        }
        ProposalCommand::RepairPreview(id) => AppCommand::PreviewProposalRepair(*id),
        ProposalCommand::Repair(file) => {
            let request: RepairRequest = input(file)?;
            validate_repair_request(&request).map_err(super::error::classify_workflow)?;
            operation = request.id;
            AppCommand::RepairProposal(request)
        }
    };
    Ok((operation, command))
}

pub(super) fn prepare(command: &ProposalCommand) -> Result<(Uuid, AppCommand), CliFailure> {
    let prepared = prepare_input(command)?;
    if crate::CANCEL.load(Ordering::SeqCst) {
        return Err(
            CliError::Interrupted("interrupted during proposal input preparation".into()).into(),
        );
    }
    Ok(prepared)
}

#[cfg(test)]
mod tests {
    use super::*;
    use brn_workflow::proposal_apply::RepairDirection;
    use brn_workflow::{proposal_rewrite::ReasoningEffort, Provider, Selection};

    #[test]
    fn asset_parser_and_direct_preparation_share_the_workflow_path_boundary() {
        let directory = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let args = [
            "proposals",
            "asset",
            "assets/proof.bin",
            "--data-dir",
            directory.path().to_str().unwrap(),
        ]
        .map(str::to_owned);
        let parsed = crate::cli::parse(&args)
            .unwrap_or_else(|failure| panic!("{}", failure.error.message()));
        let crate::cli::Outcome::Run(invocation) = parsed else {
            panic!("asset invocation")
        };
        let crate::cli::Command::Proposals(command) = invocation.command else {
            panic!("proposal command")
        };
        assert_eq!(command.name(), "proposals.asset");
        let (id, request) = prepare_input(&command).unwrap();
        assert!(!id.is_nil());
        assert!(matches!(request, AppCommand::ProposalAsset(path) if path == "assets/proof.bin"));
        for path in [
            "../outside.bin",
            ".hidden.bin",
            "note.MD",
            "archive/old.bin",
            "a//b.bin",
        ] {
            assert!(prepare_input(&ProposalCommand::Asset(path.into())).is_err());
        }
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[test]
    fn rewrite_submission_preserves_exact_job_uuid_and_generation() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("rewrite.json");
        let request = RewriteRequest {
            id: Uuid::new_v4(),
            expected: ProposalStamp {
                id: Uuid::new_v4(),
                version: 7,
            },
            selection: Selection {
                provider: Provider::Copilot,
                model: "gpt-5.3-codex".into(),
            },
            effort: ReasoningEffort::High,
            generation: 19,
        };
        std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
        let (id, command) = prepare_input(&ProposalCommand::Rewrite(path)).unwrap();
        assert_eq!(id, request.id);
        assert!(
            matches!(command, AppCommand::StartProposalRewrite(submitted) if submitted == request)
        );
    }

    #[test]
    fn repair_submission_correlates_with_the_exact_attempt_uuid() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("repair.json");
        let request = RepairRequest {
            id: Uuid::new_v4(),
            operation_id: Uuid::new_v4(),
            expected: [17; 32],
            direction: RepairDirection::Restore,
        };
        std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
        let (id, command) = prepare_input(&ProposalCommand::Repair(path)).unwrap();
        assert_eq!(id, request.id);
        assert!(matches!(command, AppCommand::RepairProposal(submitted) if submitted == request));
    }
}
