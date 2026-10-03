//! Headless typed review and explicit approval over the shared worker.
use super::{
    error::CliError, expect_positionals, required_positional, scan, sub_word, usage, CliFailure,
    Globals, Scanned, Tokens,
};
use brn_workflow::{
    app_worker::AppCommand,
    proposal_apply::{validate_approval_request, ApprovalRequest, GroupApprovalRequest},
    proposals::{CommentRequest, DraftRequest, ProposalEdit, ProposalStamp},
};
use serde::de::DeserializeOwned;
use std::{
    fs::File, io::Read, os::unix::fs::OpenOptionsExt, path::PathBuf, sync::atomic::Ordering,
};
use uuid::Uuid;

pub enum ProposalCommand {
    Create(PathBuf),
    List(Option<Uuid>),
    Show(Uuid),
    Edit(PathBuf),
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
}

impl ProposalCommand {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Create(_) => "proposals.create",
            Self::List(_) => "proposals.list",
            Self::Show(_) => "proposals.show",
            Self::Edit(_) => "proposals.edit",
            Self::RewriteResult(_) => "proposals.rewrite-result",
            Self::Comment(_) => "proposals.comment",
            Self::CommentUpdate(_) => "proposals.comment-update",
            Self::CommentRemove { .. } => "proposals.comment-remove",
            Self::Reject(_) => "proposals.reject",
            Self::Approve(_) => "proposals.approve",
            Self::Reconcile(_) => "proposals.reconcile",
            Self::ApproveGroup(_) => "proposals.approve-group",
            Self::Applies => "proposals.applies",
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
        "create|list|show|edit|rewrite-result|comment|comment-update|comment-remove|reject|approve|reconcile|approve-group|applies",
    )?;
    let (label, options): (_, &[(&str, bool)]) = match sub.as_str() {
        "create" => ("proposals.create", &[("file", true)]),
        "list" => ("proposals.list", &[("group", true)]),
        "show" => ("proposals.show", &[]),
        "edit" => ("proposals.edit", &[("file", true)]),
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
        _ => return Err(usage("unknown proposals subcommand")),
    };
    *name = Some(label);
    scan(tokens, globals, options)
}

pub(super) fn parse_command(name: &str, s: &Scanned) -> Result<ProposalCommand, CliError> {
    let positional = matches!(
        name,
        "proposals.show"
            | "proposals.comment-remove"
            | "proposals.reject"
            | "proposals.approve"
            | "proposals.reconcile"
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
        "proposals.create" => Ok(ProposalCommand::Create(file()?)),
        "proposals.list" => Ok(ProposalCommand::List(s.uuid("group")?)),
        "proposals.show" => Ok(ProposalCommand::Show(id()?)),
        "proposals.edit" => Ok(ProposalCommand::Edit(file()?)),
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
        "proposals.reconcile" => {
            let operation_id = Uuid::parse_str(required_positional(s, "OPERATION_UUID")?)
                .map_err(|_| usage("invalid approval operation UUID"))?;
            if operation_id.is_nil() {
                return Err(usage("approval operation UUID must be nonzero"));
            }
            Ok(ProposalCommand::Reconcile(operation_id))
        }
        "proposals.approve-group" => Ok(ProposalCommand::ApproveGroup(file()?)),
        "proposals.applies" => Ok(ProposalCommand::Applies),
        _ => unreachable!("scanned proposal command"),
    }
}

fn input<T: DeserializeOwned>(path: &PathBuf) -> Result<T, CliError> {
    // JSON escaping can expand bounded proposal text by six times. The encoded
    // envelope has its own hard cap; domain limits are checked after decoding.
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

pub(super) fn prepare(command: &ProposalCommand) -> Result<(Uuid, AppCommand), CliFailure> {
    let operation = match command {
        ProposalCommand::Approve(request) => request.operation_id,
        ProposalCommand::Reconcile(id) => *id,
        _ => Uuid::new_v4(),
    };
    let command = match command {
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
    };
    if crate::CANCEL.load(Ordering::SeqCst) {
        return Err(
            CliError::Interrupted("interrupted during proposal input preparation".into()).into(),
        );
    }
    Ok((operation, command))
}
