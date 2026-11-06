//! The checked subscription web route: per-chat **Allow web access** and the
//! agent's `web_search`, without workspace context.
use crate::{AppError, RecordData, Workspace, WorkspaceSettings, resolve_run_selection};
use brn_ai::{
    Provider, ReasoningEffort, Selection,
    subscription_search::{Request, SearchError, check_auth_file},
    threads::{RunWebSearch, WebSearchCancel as CancellationToken, WebSearchFuture},
};
use std::{path::PathBuf, sync::Arc};

/// Whether the agent may search the web in one chat. Unavailable states why
/// the selected route cannot search; it is never treated as the owner's Off.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WebAccessState {
    Allowed,
    Off,
    Unavailable(String),
}

/// The selected subscription route, when it supports search. BRN never
/// substitutes another provider, account or billing route.
fn route(
    settings: &WorkspaceSettings,
) -> Result<(Selection, ReasoningEffort, Option<PathBuf>), RouteError> {
    if !matches!(
        settings.provider.to_ascii_lowercase().as_str(),
        "codex" | "chatgpt"
    ) {
        return Err(RouteError::Search(SearchError::UnsupportedProvider));
    }
    // Chat's credentials-folder route can select a different account. Never
    // silently substitute the host's managed login for that saved selection,
    // and offer search only on the account path the adapter accepts.
    let Some(auth_file) = settings.auth_file.as_deref().map(PathBuf::from) else {
        return Err(RouteError::Search(SearchError::UnsupportedAuthPath));
    };
    check_auth_file(&auth_file).map_err(RouteError::Search)?;
    let (selection, effort) =
        resolve_run_selection(settings, None).map_err(|e| RouteError::Selection(e.to_string()))?;
    debug_assert_eq!(selection.provider, Provider::Chatgpt);
    Ok((selection, effort, Some(auth_file)))
}

enum RouteError {
    Search(SearchError),
    Selection(String),
}
impl RouteError {
    fn full(self) -> String {
        match self {
            Self::Search(error) => error.to_string(),
            Self::Selection(error) => error,
        }
    }
    /// The + menu's one-line reason.
    fn short(self) -> String {
        match self {
            Self::Search(SearchError::UnsupportedProvider) => {
                "Not available for this provider".into()
            }
            Self::Search(SearchError::UnsupportedAuthPath) => {
                "Needs the managed Codex login".into()
            }
            other => other.full(),
        }
    }
}

pub fn prepare(settings: &WorkspaceSettings, query: String) -> Result<Request, String> {
    if query.trim().is_empty() || query.len() > 16 * 1024 {
        return Err(SearchError::InvalidQuery.to_string());
    }
    let (selection, effort, auth_file) = route(settings).map_err(RouteError::full)?;
    Ok(Request {
        selection,
        effort,
        query,
        auth_file,
    })
}

struct SubscriptionWebSearch {
    selection: Selection,
    effort: ReasoningEffort,
    auth_file: Option<PathBuf>,
}
impl RunWebSearch for SubscriptionWebSearch {
    fn search(&self, query: String, cancel: CancellationToken) -> WebSearchFuture {
        let request = Request {
            selection: self.selection.clone(),
            effort: self.effort,
            query,
            auth_file: self.auth_file.clone(),
        };
        Box::pin(async move {
            if request.query.trim().is_empty() || request.query.len() > 16 * 1024 {
                return Err(SearchError::InvalidQuery);
            }
            brn_ai::subscription_search::search(request, cancel).await
        })
    }
}

impl Workspace {
    /// The chat's web access; `None` is a new chat, which starts Allowed.
    pub fn chat_web_access(&self, chat: Option<&str>) -> Result<WebAccessState, AppError> {
        let settings = self.settings()?;
        if let Err(reason) = route(&settings) {
            return Ok(WebAccessState::Unavailable(reason.short()));
        }
        let allowed = match chat {
            Some(chat) => self.store.chat_web_allowed(chat)?,
            None => true,
        };
        Ok(if allowed {
            WebAccessState::Allowed
        } else {
            WebAccessState::Off
        })
    }

    /// Save the owner's choice for an existing chat. It applies to later runs;
    /// a run already working keeps the access it started with.
    pub fn set_chat_web_allowed(&mut self, chat: &str, allowed: bool) -> Result<(), AppError> {
        let Some(record) = self.store.record(chat)? else {
            return Err(AppError::Invalid("That chat no longer exists.".into()));
        };
        if !matches!(record.data, RecordData::Thread(_)) || record.archived {
            return Err(AppError::Invalid("That chat no longer exists.".into()));
        }
        Ok(self.store.save_chat_web_allowed(chat, allowed)?)
    }

    /// The checked searcher for one chat run, only when access is Allowed.
    pub(crate) fn web_searcher(
        &self,
        chat: &str,
        settings: &WorkspaceSettings,
    ) -> Option<Arc<dyn RunWebSearch>> {
        let (selection, effort, auth_file) = route(settings).ok()?;
        if !self.store.chat_web_allowed(chat).ok()? {
            return None;
        }
        Some(Arc::new(SubscriptionWebSearch {
            selection,
            effort,
            auth_file,
        }))
    }
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

    #[test]
    fn chat_web_access_is_per_chat_and_unavailable_routes_never_search() {
        let root = tempfile::tempdir().unwrap();
        let mut workspace =
            crate::Workspace::open_without_default_credentials(root.path()).unwrap();
        let alpha = workspace.new_thread("Alpha").unwrap().id;
        let bravo = workspace.new_thread("Bravo").unwrap().id;
        let mut settings = workspace.settings().unwrap();
        settings.provider = "codex".into();
        settings.model = "gpt-6-sol".into();
        settings.effort = "medium".into();
        settings.auth_file = Some(
            root.path()
                .join("synthetic-auth.json")
                .display()
                .to_string(),
        );
        workspace.configure(settings.clone()).unwrap();
        // Search runs only through the managed login's auth.json, so another
        // account file is Unavailable before the tool is offered.
        assert_eq!(
            workspace.chat_web_access(Some(&alpha)).unwrap(),
            WebAccessState::Unavailable("Needs the managed Codex login".into())
        );
        assert!(workspace.web_searcher(&alpha, &settings).is_none());
        settings.auth_file =
            brn_ai::subscription_search::managed_auth_file().map(|path| path.display().to_string());
        workspace.configure(settings.clone()).unwrap();

        assert_eq!(
            workspace.chat_web_access(None).unwrap(),
            WebAccessState::Allowed
        );
        assert_eq!(
            workspace.chat_web_access(Some(&alpha)).unwrap(),
            WebAccessState::Allowed
        );
        workspace.set_chat_web_allowed(&alpha, false).unwrap();
        assert_eq!(
            workspace.chat_web_access(Some(&alpha)).unwrap(),
            WebAccessState::Off
        );
        assert_eq!(
            workspace.chat_web_access(Some(&bravo)).unwrap(),
            WebAccessState::Allowed
        );
        let settings = workspace.settings().unwrap();
        assert!(workspace.web_searcher(&alpha, &settings).is_none());
        assert!(workspace.web_searcher(&bravo, &settings).is_some());
        assert!(workspace.set_chat_web_allowed("missing", true).is_err());

        let mut copilot = settings;
        copilot.provider = "copilot".into();
        copilot.model = "gpt-4o".into();
        workspace.configure(copilot.clone()).unwrap();
        assert_eq!(
            workspace.chat_web_access(Some(&bravo)).unwrap(),
            WebAccessState::Unavailable("Not available for this provider".into())
        );
        assert!(workspace.web_searcher(&bravo, &copilot).is_none());
        // The owner's saved choice survives an unavailable route.
        copilot.provider = "codex".into();
        copilot.model = "gpt-6-sol".into();
        workspace.configure(copilot).unwrap();
        assert_eq!(
            workspace.chat_web_access(Some(&alpha)).unwrap(),
            WebAccessState::Off
        );
    }
}
