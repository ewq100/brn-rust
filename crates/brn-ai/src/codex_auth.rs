//! Read the explicitly selected normal Codex subscription account in memory.
//! No credential copy, account discovery, API-key fallback, or persistence.
use crate::auth::{OwnedClient, validate_ancestors, validate_file};
use crate::error::map_auth;
use crate::{AiError, AiErrorKind, AiResult, Provider, ProviderClient, Selection};
use rig::http_client::DynHttpClient;
use rig::providers::{chatgpt, openai};
use std::{
    fs::OpenOptions,
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};
use tokio_util::sync::CancellationToken;

pub async fn codex_client(
    auth_file: &Path,
    selection: &Selection,
    cancel: CancellationToken,
) -> AiResult<ProviderClient> {
    selection.validate()?;
    if selection.provider != Provider::Chatgpt || !auth_file.is_absolute() {
        return Err(AiError::new(AiErrorKind::ToolRejected));
    }
    let parent = auth_file
        .parent()
        .ok_or_else(|| AiError::new(AiErrorKind::UnsafeCredentials))?;
    validate_ancestors(parent)?;
    let meta = std::fs::symlink_metadata(auth_file)
        .map_err(|_| AiError::new(AiErrorKind::ReconnectNeeded))?;
    validate_file(&meta, false)?;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(auth_file)
        .map_err(|_| AiError::new(AiErrorKind::UnsafeCredentials))?;
    let opened = file
        .metadata()
        .map_err(|_| AiError::new(AiErrorKind::UnsafeCredentials))?;
    validate_file(&opened, false)?;
    if meta.ino() != opened.ino() || meta.dev() != opened.dev() {
        return Err(AiError::new(AiErrorKind::UnsafeCredentials));
    }
    let mut bytes = Vec::new();
    file.take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| AiError::new(AiErrorKind::Storage))?;
    if bytes.len() > 1024 * 1024 {
        return Err(AiError::new(AiErrorKind::ReconnectNeeded));
    }
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| AiError::new(AiErrorKind::ReconnectNeeded))?;
    // Only the owner-selected subscription route. Refuse API-key accounts.
    if value
        .get("auth_mode")
        .and_then(|v| v.as_str())
        .is_some_and(|mode| mode != "chatgpt")
    {
        return Err(AiError::new(AiErrorKind::ReconnectNeeded));
    }
    let tokens = value
        .get("tokens")
        .ok_or_else(|| AiError::new(AiErrorKind::ReconnectNeeded))?;
    let access_token = tokens
        .get("access_token")
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| AiError::new(AiErrorKind::ReconnectNeeded))?
        .to_owned();
    let account_id = tokens
        .get("account_id")
        .and_then(|v| v.as_str())
        .map(str::to_owned);
    let auth = chatgpt::auth::Authenticator::new(
        chatgpt::auth::AuthSource::AccessToken {
            access_token,
            account_id,
        },
        None,
        chatgpt::auth::DeviceCodeHandler::new(|_| {}),
        false,
    );
    let operation = openai::OpenAIConfig::with_key(&chatgpt::DIALECT, "")
        .connect(DynHttpClient::new(rig::rig_reqwest::shared()))
        .authenticate(&auth);
    let inner = tokio::select! {
        biased;
        _ = cancel.cancelled() => return Err(AiError::new(AiErrorKind::Other)),
        client = operation => client.map_err(map_auth)?,
    };
    Ok(ProviderClient {
        inner: OwnedClient::Chatgpt(Box::new(inner)),
        selection: selection.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    fn selection() -> Selection {
        Selection {
            provider: Provider::Chatgpt,
            model: "gpt-6.1-sol".into(),
        }
    }
    #[tokio::test]
    async fn malformed_and_api_key_files_refuse_without_fallback_or_secret_errors() {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let file = root.path().join("auth.json");
        for contents in [
            "SYNTHETIC_SECRET malformed",
            r#"{"auth_mode":"apikey","OPENAI_API_KEY":"SYNTHETIC_SECRET"}"#,
            r#"{"auth_mode":"chatgpt","tokens":{"access_token":""}}"#,
        ] {
            std::fs::write(&file, contents).unwrap();
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
            let error = codex_client(&file, &selection(), CancellationToken::new())
                .await
                .err()
                .unwrap();
            assert_eq!(error.kind, AiErrorKind::ReconnectNeeded);
            assert!(!format!("{error:?}").contains("SYNTHETIC_SECRET"));
        }
    }
    #[tokio::test]
    async fn unsafe_path_or_provider_is_refused_before_authentication() {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let target = root.path().join("auth.json");
        std::fs::write(&target, "{}").unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600)).unwrap();
        let link = root.path().join("link.json");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert_eq!(
            codex_client(&link, &selection(), CancellationToken::new())
                .await
                .err()
                .unwrap()
                .kind,
            AiErrorKind::UnsafeCredentials
        );
        assert_eq!(
            codex_client(
                Path::new("relative"),
                &selection(),
                CancellationToken::new()
            )
            .await
            .err()
            .unwrap()
            .kind,
            AiErrorKind::ToolRejected
        );
        let mut other = selection();
        other.provider = Provider::Copilot;
        assert_eq!(
            codex_client(&target, &other, CancellationToken::new())
                .await
                .err()
                .unwrap()
                .kind,
            AiErrorKind::ToolRejected
        );
    }
}
