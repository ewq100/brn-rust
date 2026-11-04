//! Prepare a research request from the owner's current settings, without workspace context.
use crate::{WorkspaceSettings, resolve_run_selection};
use brn_ai::{
    Provider,
    subscription_search::{Request, SearchError},
};

pub fn prepare(settings: &WorkspaceSettings, query: String) -> Result<Request, String> {
    if query.trim().is_empty() || query.len() > 16 * 1024 {
        return Err(SearchError::InvalidQuery.to_string());
    }
    if !matches!(
        settings.provider.to_ascii_lowercase().as_str(),
        "codex" | "chatgpt"
    ) {
        return Err(SearchError::UnsupportedProvider.to_string());
    }
    // Chat's credentials-folder route can select a different account. Never
    // silently substitute the host's managed login for that saved selection.
    if settings.auth_file.is_none() {
        return Err(SearchError::UnsupportedAuthPath.to_string());
    }
    let (selection, effort) = resolve_run_selection(settings, None).map_err(|e| e.to_string())?;
    debug_assert_eq!(selection.provider, Provider::Chatgpt);
    Ok(Request {
        selection,
        effort,
        query,
        auth_file: settings.auth_file.clone().map(Into::into),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_empty_query_and_unsupported_provider_without_dispatch() {
        let root = tempfile::tempdir().unwrap();
        let workspace = crate::Workspace::open_without_default_credentials(root.path()).unwrap();
        let mut settings = workspace.settings().unwrap();
        assert!(prepare(&settings, "  ".into()).is_err());
        settings.provider = "copilot".into();
        settings.model = "gpt-4o".into();
        settings.effort = "medium".into();
        assert_eq!(
            prepare(&settings, "public query".into()).unwrap_err(),
            SearchError::UnsupportedProvider.to_string()
        );
    }

    #[test]
    fn rejects_credentials_folder_or_missing_account_without_host_fallback() {
        let root = tempfile::tempdir().unwrap();
        let workspace = crate::Workspace::open_without_default_credentials(root.path()).unwrap();
        let mut settings = workspace.settings().unwrap();
        settings.provider = "codex".into();
        settings.model = "gpt-6-sol".into();
        settings.effort = "medium".into();
        settings.auth_file = None;
        for credentials in [None, Some("/synthetic/account".into())] {
            settings.credentials_dir = credentials;
            assert_eq!(
                prepare(&settings, "public query".into()).unwrap_err(),
                SearchError::UnsupportedAuthPath.to_string()
            );
        }
    }
}
