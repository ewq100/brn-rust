//! Read-only, scoped saved-note relationship pages through AppWorker.
use super::{
    error::{classify_workflow, CliError},
    expect_positionals, parse_scope, scan, sub_word, usage, CliFailure, Globals, Scanned, Tokens,
};
use brn_workflow::{app_worker::AppCommand, knowledge::RelationshipRequest};

pub(super) fn scan_command(
    tokens: Tokens<'_>,
    globals: &mut Globals,
    name: &mut Option<&'static str>,
) -> Result<Scanned, CliError> {
    if sub_word(tokens, "relationships", "list")? != "list" {
        return Err(usage("unknown relationships subcommand"));
    }
    *name = Some("relationships.list");
    scan(
        tokens,
        globals,
        &[("scope", true), ("offset", true), ("limit", true)],
    )
}

pub(super) fn parse_command(scanned: &Scanned) -> Result<RelationshipRequest, CliError> {
    expect_positionals(scanned, 0)?;
    let integer = |name: &str, default: usize| {
        scanned.value(name).map_or(Ok(default), |value| {
            value
                .parse::<usize>()
                .map_err(|_| usage(format!("invalid --{name} nonnegative integer: {value}")))
        })
    };
    let request = RelationshipRequest {
        scope: parse_scope(scanned.value("scope"))?,
        offset: integer("offset", 0)?,
        limit: integer("limit", 50)?,
    };
    request.validate().map_err(|error| usage(error.message))?;
    Ok(request)
}

pub(super) fn prepare(request: &RelationshipRequest) -> Result<AppCommand, CliFailure> {
    request.validate().map_err(classify_workflow)?;
    Ok(AppCommand::Relationships(request.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Command, Invocation};
    use brn_workflow::library::KnowledgeScope;

    #[test]
    fn direct_invalid_page_requests_refuse_before_storage_or_credentials_open() {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = owner.path().join("data");
        std::fs::create_dir(&data).unwrap();
        let credentials = owner.path().join("task.credentials");
        for limit in [0, 201, usize::MAX] {
            let invocation = Invocation {
                json: true,
                data_dir: data.clone(),
                model_dir: None,
                vault: None,
                credentials_dir: Some(credentials.clone()),
                command: Command::Relationships(RelationshipRequest {
                    scope: KnowledgeScope::All,
                    offset: 0,
                    limit,
                }),
            };
            assert!(super::super::execute(&invocation).is_err());
            assert_eq!(std::fs::read_dir(&data).unwrap().count(), 0);
            assert!(!credentials.exists());
        }
    }
}
