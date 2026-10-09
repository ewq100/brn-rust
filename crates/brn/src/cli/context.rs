//! Explicit saved profile lenses through the shared read-only application query.
use super::{
    error::CliError, expect_positionals, scan, sub_word, usage, CliFailure, Globals, Output,
    Scanned, Tokens,
};
use brn_workflow::{app_worker::AppEvent, knowledge::ProfileContextRequest};

pub(super) fn scan_command(
    tokens: Tokens<'_>,
    globals: &mut Globals,
    name: &mut Option<&'static str>,
) -> Result<Scanned, CliError> {
    if sub_word(tokens, "context", "inspect")? != "inspect" {
        return Err(usage("unknown context subcommand (expected inspect)"));
    }
    *name = Some("context.inspect");
    scan(tokens, globals, &[("file", true)])
}

pub(super) fn parse_command(scanned: &Scanned) -> Result<ProfileContextRequest, CliError> {
    expect_positionals(scanned, 0)?;
    let path = scanned
        .value("file")
        .ok_or_else(|| usage("missing --file"))?;
    let request: ProfileContextRequest =
        super::input::read_small_json_file(std::path::Path::new(path), "profile context request")?;
    request.validate().map_err(|error| usage(error.message))?;
    Ok(request)
}

pub(super) fn output(
    request: &ProfileContextRequest,
    event: AppEvent,
) -> Result<Output, CliFailure> {
    let AppEvent::ProfileContext(context) = event else {
        return Err(
            CliError::Workflow("unexpected application reply for context command".into()).into(),
        );
    };
    context
        .validate_for(request)
        .map_err(super::error::classify_workflow)?;
    Ok(Output {
        text: format!("{}\n", super::actions::record_text(&context)),
        data: serde_json::json!(*context),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Command, Invocation};
    use brn_workflow::{
        knowledge::{ProfileContext, ProfileLens},
        proposals::{ProposalSource, SourceVersion},
    };
    use uuid::Uuid;

    #[test]
    fn direct_invalid_requests_and_wrong_or_malformed_replies_have_no_workspace_effects() {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let vault = owner.path().join("vault");
        std::fs::create_dir(&vault).unwrap();
        let id = Uuid::new_v4();
        std::fs::write(
            vault.join("profile.md"),
            format!("---\nbrn_id: {id}\n---\nExact profile"),
        )
        .unwrap();
        // This test qualifies pure CLI reply/preflight validation. Construct its
        // exact typed evidence directly; Markdown Save is a macOS-only workflow.
        let text = std::fs::read_to_string(vault.join("profile.md")).unwrap();
        let request = ProfileContextRequest {
            profile: SourceVersion {
                path: "profile.md".into(),
                fingerprint: brn_store::files::FileFingerprint {
                    device: 1,
                    inode: 1,
                    len: text.len() as u64,
                    sha256: brn_intake::digest(text.as_bytes()),
                },
            },
            note_id: id,
            lens: ProfileLens::Person,
            action_offset: 0,
            relationship_offset: 0,
            limit: 25,
        };
        let context = ProfileContext {
            profile: ProposalSource {
                source: request.profile.clone(),
                text,
            },
            request: request.clone(),
            action_total: 0,
            actions: vec![],
            relationship_total: 0,
            relationships: vec![],
            references: vec![],
            issues: vec![],
            duplicates: vec![],
            complete: true,
        };
        assert!(output(
            &request,
            AppEvent::ProfileContext(Box::new(context.clone()))
        )
        .is_ok());
        assert!(output(&request, AppEvent::Refreshed(Default::default())).is_err());
        let mut wrong = context.clone();
        wrong.request.lens = ProfileLens::Project;
        assert!(output(&request, AppEvent::ProfileContext(Box::new(wrong))).is_err());
        let mut wrong = context;
        wrong.relationship_total = 1;
        assert!(output(&request, AppEvent::ProfileContext(Box::new(wrong))).is_err());
        let untouched = owner.path().join("untouched");
        std::fs::create_dir(&untouched).unwrap();
        let credentials = owner.path().join("untouched.credentials");
        for limit in [0, 201, usize::MAX] {
            let invocation = Invocation {
                json: true,
                data_dir: untouched.clone(),
                model_dir: None,
                vault: Some(vault.clone()),
                credentials_dir: Some(credentials.clone()),
                command: Command::Context(ProfileContextRequest {
                    limit,
                    ..request.clone()
                }),
            };
            assert!(super::super::execute(&invocation).is_err());
            assert_eq!(std::fs::read_dir(&untouched).unwrap().count(), 0);
            assert!(!credentials.exists());
        }
    }
}
