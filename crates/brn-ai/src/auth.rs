use crate::error::{map_auth, map_provider, map_transport};
use crate::{
    AccountStatus, AiError, AiErrorKind, AiResult, LoginPrompt, ModelOption, Provider, Selection,
};
use rig::http_client::{DynHttpClient, HttpClientExt, NoBody, Request};
use rig::providers::{chatgpt, copilot, openai};
use std::fs::{File, Metadata, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub struct Auth {
    dir: PathBuf,
    chatgpt: Mutex<()>,
    copilot: Mutex<()>,
    http: DynHttpClient,
}

pub struct ProviderClient {
    pub(crate) inner: OwnedClient,
    selection: Selection,
}

pub(crate) enum OwnedClient {
    Chatgpt(Box<openai::OpenAI>),
    Copilot(copilot::Copilot),
}

impl ProviderClient {
    pub fn selection(&self) -> &Selection {
        &self.selection
    }
}

impl Provider {
    fn files(self) -> &'static [&'static str] {
        match self {
            Self::Chatgpt => &["chatgpt.json", "chatgpt-name.json"],
            Self::Copilot => &["github-token", "copilot.json", "copilot-name.json"],
        }
    }
    fn name_file(self) -> &'static str {
        match self {
            Self::Chatgpt => "chatgpt-name.json",
            Self::Copilot => "copilot-name.json",
        }
    }
}

impl Auth {
    #[cfg(test)]
    pub(crate) fn with_http(mut self, http: impl HttpClientExt + 'static) -> Self {
        self.http = DynHttpClient::new(http);
        self
    }

    pub fn open(credentials_dir: &Path) -> AiResult<Self> {
        if !credentials_dir.is_absolute() {
            return Err(unsafe_credentials());
        }
        validate_ancestors(credentials_dir)?;
        if !credentials_dir.try_exists().map_err(storage)? {
            std::fs::DirBuilder::new()
                .mode(0o700)
                .create(credentials_dir)
                .map_err(storage)?;
        }
        let auth = Self {
            dir: credentials_dir.to_owned(),
            chatgpt: Mutex::new(()),
            copilot: Mutex::new(()),
            http: DynHttpClient::new(rig::rig_reqwest::shared()),
        };
        auth.validate_dir()?;
        Ok(auth)
    }

    fn slot(&self, provider: Provider) -> &Mutex<()> {
        match provider {
            Provider::Chatgpt => &self.chatgpt,
            Provider::Copilot => &self.copilot,
        }
    }

    fn validate(&self, provider: Provider) -> AiResult<()> {
        self.validate_dir()?;
        for name in provider.files() {
            self.checked_file(name, false)?;
        }
        Ok(())
    }

    fn validate_dir(&self) -> AiResult<()> {
        validate_ancestors(&self.dir)?;
        let meta = std::fs::symlink_metadata(&self.dir).map_err(storage)?;
        validate_metadata(
            meta.is_file(),
            meta.is_dir(),
            meta.nlink(),
            meta.uid(),
            meta.mode(),
            true,
        )
    }

    fn checked_file(&self, name: &str, tighten: bool) -> AiResult<Option<File>> {
        let path = self.dir.join(name);
        let meta = match std::fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(storage(e)),
        };
        validate_file(&meta, tighten)?;
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&path)
            .map_err(|_| unsafe_credentials())?;
        let opened = file.metadata().map_err(storage)?;
        validate_file(&opened, tighten)?;
        if opened.ino() != meta.ino() || opened.dev() != meta.dev() {
            return Err(unsafe_credentials());
        }
        if tighten {
            file.set_permissions(std::fs::Permissions::from_mode(0o600))
                .map_err(|_| unsafe_credentials())?;
            validate_file(&file.metadata().map_err(|_| unsafe_credentials())?, false)?;
        }
        Ok(Some(file))
    }

    fn read(&self, name: &str) -> AiResult<Option<Vec<u8>>> {
        let Some(file) = self.checked_file(name, false)? else {
            return Ok(None);
        };
        let mut bytes = Vec::new();
        file.take(1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(storage)?;
        if bytes.len() > 1024 * 1024 {
            return Err(record_error(name));
        }
        Ok(Some(bytes))
    }

    fn json(&self, name: &str) -> AiResult<Option<serde_json::Value>> {
        self.read(name)?
            .map(|bytes| {
                let value = serde_json::from_slice(&bytes).map_err(|_| record_error(name))?;
                if matches!(name, "chatgpt.json" | "copilot.json") {
                    validate_record(&value, name)?;
                }
                Ok(value)
            })
            .transpose()
    }

    fn tighten(&self, provider: Provider) -> AiResult<()> {
        self.validate_dir().map_err(|_| unsafe_credentials())?;
        // Check every known file even when an earlier one cannot be tightened.
        let mut result = Ok(());
        for name in provider.files() {
            if self.checked_file(name, true).is_err() {
                result = Err(unsafe_credentials());
            }
        }
        result
    }

    fn local_status(&self, provider: Provider) -> AiResult<AccountStatus> {
        let (connected, name) = match provider {
            Provider::Chatgpt => {
                let cache = self.json("chatgpt.json")?;
                let connected = cache.as_ref().is_some_and(|c| {
                    nonempty(c.get("access_token")).is_some()
                        || nonempty(c.get("refresh_token")).is_some()
                });
                let name = cache
                    .as_ref()
                    .and_then(|c| c.get("id_token"))
                    .and_then(|v| v.as_str())
                    .and_then(email_claim);
                (connected, name)
            }
            Provider::Copilot => {
                let github = self.github_token()?.is_some();
                let session = self.json("copilot.json")?;
                let connected = github
                    || session
                        .as_ref()
                        .is_some_and(|c| nonempty(c.get("token")).is_some());
                let name = if connected {
                    self.json(provider.name_file())?
                        .as_ref()
                        .and_then(|v| v.get("name"))
                        .and_then(|v| v.as_str())
                        .and_then(display_name)
                } else {
                    None
                };
                (connected, name)
            }
        };
        Ok(AccountStatus {
            provider,
            connected,
            name: if connected { name } else { None },
        })
    }

    pub async fn status(&self, provider: Provider) -> AiResult<AccountStatus> {
        let _guard = self.slot(provider).lock().await;
        self.validate(provider)?;
        self.local_status(provider)
    }

    async fn authenticate(
        &self,
        provider: Provider,
        login: Option<Arc<dyn Fn(LoginPrompt) + Send + Sync>>,
        cancel: &CancellationToken,
    ) -> AiResult<OwnedClient> {
        let allow_device_flow = login.is_some();
        match provider {
            Provider::Chatgpt => {
                self.json("chatgpt.json")?;
            }
            Provider::Copilot => {
                self.json("copilot.json")?;
                self.github_token()?;
            }
        }
        let handler = chatgpt::auth::DeviceCodeHandler::new(move |p| {
            if let Some(login) = &login {
                login(LoginPrompt {
                    verification_uri: p.verification_uri,
                    user_code: p.user_code,
                });
            }
        });
        let operation = async {
            match provider {
                Provider::Chatgpt => {
                    let authenticator = chatgpt::auth::Authenticator::new(
                        chatgpt::auth::AuthSource::OAuth,
                        Some(self.dir.join("chatgpt.json")),
                        handler,
                        allow_device_flow,
                    );
                    openai::OpenAIConfig::with_key(&chatgpt::DIALECT, "")
                        .connect(self.http.clone())
                        .authenticate(&authenticator)
                        .await
                        .map(|client| OwnedClient::Chatgpt(Box::new(client)))
                        .map_err(map_auth)
                }
                Provider::Copilot => {
                    let authenticator = copilot::auth::Authenticator::new(
                        copilot::auth::AuthSource::OAuth,
                        Some(self.dir.join("github-token")),
                        Some(self.dir.join("copilot.json")),
                        handler,
                        allow_device_flow,
                    );
                    copilot::CopilotConfig::new("")
                        .connect(self.http.clone())
                        .authenticate(&authenticator)
                        .await
                        .map(OwnedClient::Copilot)
                        .map_err(map_auth)
                }
            }
        };
        let client = cancellable(cancel, operation).await?;
        let credential = match &client {
            OwnedClient::Chatgpt(client) => &client.config().api_key,
            OwnedClient::Copilot(client) => &client.config().api_key,
        };
        if credential.expose().trim().is_empty() {
            return Err(AiError::new(AiErrorKind::ReconnectNeeded));
        }
        Ok(client)
    }

    pub async fn connect(
        &self,
        provider: Provider,
        login: Arc<dyn Fn(LoginPrompt) + Send + Sync>,
        cancel: CancellationToken,
    ) -> AiResult<AccountStatus> {
        let _guard = cancellable(&cancel, async { Ok(self.slot(provider).lock().await) }).await?;
        self.validate(provider)?;
        let finalizer = Finalizer::new(self, provider);
        let result = async {
            // Clear stale display metadata before a new account is resolved.
            self.delete_name(provider)?;
            let _client = self.authenticate(provider, Some(login), &cancel).await?;
            self.tighten(provider)?;
            let name = match provider {
                Provider::Chatgpt => self.local_status(provider)?.name,
                Provider::Copilot => cancellable(&cancel, self.github_name()).await?,
            };
            self.save_name(provider, name.as_deref())?;
            let mut status = self.local_status(provider)?;
            status.name = name;
            if !status.connected {
                return Err(AiError::new(AiErrorKind::ReconnectNeeded));
            }
            Ok(status)
        }
        .await;
        finalizer.finish(result)
    }

    pub async fn client(
        &self,
        selection: &Selection,
        cancel: CancellationToken,
    ) -> AiResult<ProviderClient> {
        selection.validate()?;
        let inner = self.resolve_client(selection.provider, &cancel).await?;
        Ok(ProviderClient {
            inner,
            selection: selection.clone(),
        })
    }

    async fn resolve_client(
        &self,
        provider: Provider,
        cancel: &CancellationToken,
    ) -> AiResult<OwnedClient> {
        let _guard = cancellable(cancel, async { Ok(self.slot(provider).lock().await) }).await?;
        self.validate(provider)?;
        let finalizer = Finalizer::new(self, provider);
        let result = self.authenticate(provider, None, cancel).await;
        finalizer.finish(result)
    }

    /// Caller must fence new target-provider work, cancel/join it and drop its
    /// owned clients first. This serializes cache deletion against auth/refresh.
    pub async fn disconnect(&self, provider: Provider) -> AiResult<()> {
        let _guard = self.slot(provider).lock().await;
        self.validate_dir()?;
        for name in provider.files() {
            self.check_removal(name)?;
        }
        for name in provider.files() {
            self.check_removal(name)?;
            match std::fs::remove_file(self.dir.join(name)) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(storage(e)),
            }
        }
        Ok(())
    }

    fn check_removal(&self, name: &str) -> AiResult<()> {
        match std::fs::symlink_metadata(self.dir.join(name)) {
            Ok(meta) => validate_removal(meta.is_file(), meta.file_type().is_symlink(), meta.uid()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(storage(e)),
        }
    }

    pub async fn models(
        &self,
        provider: Provider,
        cancel: CancellationToken,
    ) -> AiResult<Vec<ModelOption>> {
        if provider == Provider::Chatgpt {
            let _guard =
                cancellable(&cancel, async { Ok(self.slot(provider).lock().await) }).await?;
            self.validate(provider)?;
            return Ok(vec![ModelOption {
                id: "gpt-5.5".into(),
                live_qualified: false,
            }]);
        }
        let client = self.resolve_client(provider, &cancel).await?;
        let OwnedClient::Copilot(client) = client else {
            return Err(AiError::new(AiErrorKind::Other));
        };
        let list = cancellable(&cancel, async {
            client.list_models().await.map_err(map_provider)
        })
        .await?;
        if list.data.iter().any(|m| !crate::valid_model_id(&m.id)) {
            return Err(AiError::new(AiErrorKind::Other));
        }
        Ok(list
            .data
            .into_iter()
            .map(|m| ModelOption {
                id: m.id,
                live_qualified: false,
            })
            .collect())
    }

    fn github_token(&self) -> AiResult<Option<String>> {
        let Some(bytes) = self.read("github-token")? else {
            return Ok(None);
        };
        let token = std::str::from_utf8(&bytes)
            .map_err(|_| AiError::new(AiErrorKind::ReconnectNeeded))?
            .trim();
        if token.chars().any(char::is_control) {
            return Err(AiError::new(AiErrorKind::ReconnectNeeded));
        }
        Ok((!token.is_empty()).then(|| token.to_owned()))
    }

    async fn github_name(&self) -> AiResult<Option<String>> {
        let Some(token) = self.github_token()? else {
            return Ok(None);
        };
        let request = Request::builder()
            .method("GET")
            .uri("https://api.github.com/user")
            .header("authorization", format!("Bearer {}", token.trim()))
            .header("accept", "application/vnd.github+json")
            .header("user-agent", "brn")
            .body(NoBody)
            .map_err(|_| AiError::new(AiErrorKind::ReconnectNeeded))?;
        let response = self
            .http
            .send::<_, Vec<u8>>(request)
            .await
            .map_err(map_transport)?;
        let body = response.into_body().await.map_err(map_transport)?;
        let user: serde_json::Value =
            serde_json::from_slice(&body).map_err(|_| AiError::new(AiErrorKind::Other))?;
        Ok(user
            .get("login")
            .and_then(|v| v.as_str())
            .and_then(display_name))
    }

    fn delete_name(&self, provider: Provider) -> AiResult<()> {
        match std::fs::remove_file(self.dir.join(provider.name_file())) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(storage(e)),
        }
    }

    fn save_name(&self, provider: Provider, name: Option<&str>) -> AiResult<()> {
        let bytes = serde_json::to_vec(&serde_json::json!({ "name": name }))
            .map_err(|_| AiError::new(AiErrorKind::Storage))?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(self.dir.join(provider.name_file()))
            .map_err(storage)?;
        file.write_all(&bytes).map_err(storage)
    }
}

struct Finalizer<'a> {
    auth: &'a Auth,
    provider: Provider,
    armed: bool,
}
impl<'a> Finalizer<'a> {
    fn new(auth: &'a Auth, provider: Provider) -> Self {
        Self {
            auth,
            provider,
            armed: true,
        }
    }

    fn finish<T>(mut self, result: AiResult<T>) -> AiResult<T> {
        let tightened = self.auth.tighten(self.provider);
        self.armed = false;
        tightened?;
        result
    }
}
impl Drop for Finalizer<'_> {
    fn drop(&mut self) {
        // Also runs if the caller drops an auth future instead of cancelling it.
        if self.armed {
            let _ = self.auth.tighten(self.provider);
        }
    }
}

async fn cancellable<T>(
    cancel: &CancellationToken,
    operation: impl Future<Output = AiResult<T>>,
) -> AiResult<T> {
    tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(AiError::new(AiErrorKind::Other)),
        result = operation => result,
    }
}

fn unsafe_credentials() -> AiError {
    AiError::new(AiErrorKind::UnsafeCredentials)
}
fn storage(_: std::io::Error) -> AiError {
    AiError::new(AiErrorKind::Storage)
}
fn record_error(name: &str) -> AiError {
    AiError::new(if name.ends_with("-name.json") {
        AiErrorKind::Storage
    } else {
        AiErrorKind::ReconnectNeeded
    })
}
fn current_uid() -> u32 {
    // SAFETY: geteuid has no preconditions.
    unsafe { libc::geteuid() }
}

fn validate_metadata(
    file: bool,
    dir: bool,
    links: u64,
    uid: u32,
    mode: u32,
    folder: bool,
) -> AiResult<()> {
    if uid != current_uid()
        || if folder {
            !dir || mode & 0o7777 != 0o700
        } else {
            !file || links != 1 || mode & 0o7777 != 0o600
        }
    {
        Err(unsafe_credentials())
    } else {
        Ok(())
    }
}

fn validate_file(meta: &Metadata, tighten: bool) -> AiResult<()> {
    if tighten {
        if !meta.is_file() || meta.nlink() != 1 || meta.uid() != current_uid() {
            return Err(unsafe_credentials());
        }
        Ok(())
    } else {
        validate_metadata(
            meta.is_file(),
            meta.is_dir(),
            meta.nlink(),
            meta.uid(),
            meta.mode(),
            false,
        )
    }
}

fn validate_removal(file: bool, symlink: bool, uid: u32) -> AiResult<()> {
    // Unlink the owned directory entry, not its content or any symlink target.
    if uid != current_uid() || !(file || symlink) {
        return Err(unsafe_credentials());
    }
    Ok(())
}

fn validate_ancestors(path: &Path) -> AiResult<()> {
    let mut current = PathBuf::new();
    for component in path.components() {
        if matches!(component, Component::ParentDir | Component::CurDir) {
            return Err(unsafe_credentials());
        }
        current.push(component);
        match std::fs::symlink_metadata(&current) {
            Ok(meta) if !meta.is_dir() || meta.file_type().is_symlink() => {
                return Err(unsafe_credentials());
            }
            Ok(_) => {
                if current.join(".git").exists() {
                    return Err(unsafe_credentials());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && current == path => {}
            Err(e) => return Err(storage(e)),
        }
    }
    Ok(())
}

fn nonempty(value: Option<&serde_json::Value>) -> Option<&str> {
    value
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
}

fn validate_record(value: &serde_json::Value, name: &str) -> AiResult<()> {
    let invalid = || AiError::new(AiErrorKind::ReconnectNeeded);
    let object = value.as_object().ok_or_else(invalid)?;
    let fields: &[&str] = if name == "chatgpt.json" {
        &["access_token", "refresh_token", "id_token", "account_id"]
    } else {
        &["token", "bootstrap_token_fingerprint"]
    };
    for field in fields {
        if let Some(v) = object.get(*field).filter(|v| !v.is_null())
            && v.as_str().is_none_or(|s| s.trim().is_empty())
        {
            return Err(invalid());
        }
    }
    if let Some(expiry) = object.get("expires_at").filter(|v| !v.is_null())
        && expiry.as_i64().is_none_or(|n| n < 0)
    {
        return Err(invalid());
    }
    if name == "copilot.json"
        && let Some(endpoints) = object.get("endpoints").filter(|v| !v.is_null())
    {
        if !endpoints.is_object() {
            return Err(invalid());
        }
        if let Some(api) = endpoints.get("api").filter(|v| !v.is_null())
            && !api.is_string()
        {
            return Err(invalid());
        }
    }
    Ok(())
}

fn display_name(name: &str) -> Option<String> {
    (!name.is_empty() && name.len() <= 320 && !name.chars().any(char::is_control))
        .then(|| name.to_owned())
}

fn email_claim(token: &str) -> Option<String> {
    use base64::Engine;
    let payload = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    claims.get("email")?.as_str().and_then(display_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rig::http_client::StatusCode;
    use rig::test_utils::{MockHttpResponse, SequencedHttpClient};
    use std::os::unix::fs::{PermissionsExt, symlink};

    fn test_root() -> tempfile::TempDir {
        tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
    }

    fn fixture() -> (tempfile::TempDir, Auth) {
        let root = test_root();
        let auth = Auth::open(&root.path().join("credentials"))
            .unwrap()
            .with_http(SequencedHttpClient::new([]));
        (root, auth)
    }

    fn write(auth: &Auth, name: &str, bytes: &[u8]) {
        let path = auth.dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }

    fn jwt(claims: serde_json::Value) -> String {
        use base64::Engine;
        format!(
            "e30.{}.synthetic",
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(serde_json::to_vec(&claims).unwrap())
        )
    }

    fn chat_cache(auth: &Auth, expired: bool) {
        write(
            auth,
            "chatgpt.json",
            serde_json::to_vec(&serde_json::json!({
                "access_token": "SYNTHETIC_ACCESS",
                "expires_at": if expired { 1 } else { 4102444800_i64 },
                "account_id": "synthetic-account",
                "id_token": jwt(serde_json::json!({"email": "synthetic@example.invalid"}))
            }))
            .unwrap()
            .as_slice(),
        );
    }

    fn selection(provider: Provider) -> Selection {
        Selection {
            provider,
            model: "gpt-5.5".into(),
        }
    }

    #[test]
    fn unsafe_folder_modes_and_symlinks_are_rejected() {
        let root = test_root();
        let dir = root.path().join("credentials");
        std::fs::create_dir(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(Auth::open(&dir).is_err());
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(Auth::open(&dir).is_ok());
        let link = root.path().join("link");
        symlink(&dir, &link).unwrap();
        assert!(Auth::open(&link).is_err());
    }

    #[test]
    fn cache_symlinks_hardlinks_owner_and_modes_are_rejected() {
        let (root, auth) = fixture();
        let path = auth.dir.join("chatgpt.json");
        std::fs::write(&path, b"{}").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(auth.validate(Provider::Chatgpt).is_err());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let link = root.path().join("hardlink");
        std::fs::hard_link(&path, &link).unwrap();
        assert!(auth.validate(Provider::Chatgpt).is_err());
        std::fs::remove_file(&path).unwrap();
        symlink(&link, &path).unwrap();
        assert!(auth.validate(Provider::Chatgpt).is_err());
        assert!(validate_metadata(true, false, 1, current_uid() + 1, 0o600, false).is_err());
        assert!(validate_metadata(false, true, 2, current_uid() + 1, 0o700, true).is_err());
    }

    #[tokio::test]
    async fn status_is_offline_and_identity_is_display_only() {
        let (_root, mut auth) = fixture();
        let http = SequencedHttpClient::new([]);
        auth.http = DynHttpClient::new(http.clone());
        assert!(!auth.status(Provider::Chatgpt).await.unwrap().connected);
        chat_cache(&auth, true);
        let status = auth.status(Provider::Chatgpt).await.unwrap();
        assert!(status.connected);
        assert_eq!(status.name.as_deref(), Some("synthetic@example.invalid"));
        assert!(http.requests().is_empty());
        write(&auth, "chatgpt.json", b"{}");
        let status = auth.status(Provider::Chatgpt).await.unwrap();
        assert!(!status.connected);
        assert_eq!(status.name, None);
        write(&auth, "chatgpt.json", b"{malformed SYNTHETIC_SECRET");
        let error = auth.status(Provider::Chatgpt).await.unwrap_err();
        assert_eq!(error.kind, AiErrorKind::ReconnectNeeded);
        assert!(!format!("{error:?} {error}").contains("SYNTHETIC_SECRET"));
    }

    #[tokio::test]
    async fn missing_expired_and_malformed_caches_never_start_device_login() {
        for cache in [
            None,
            Some(b"{}".as_slice()),
            Some(b"bad SYNTHETIC_SECRET".as_slice()),
        ] {
            let (_root, mut auth) = fixture();
            let http = SequencedHttpClient::new([]);
            auth.http = DynHttpClient::new(http.clone());
            if let Some(cache) = cache {
                write(&auth, "chatgpt.json", cache);
            }
            let result = auth
                .client(&selection(Provider::Chatgpt), CancellationToken::new())
                .await;
            assert_eq!(result.err().unwrap().kind, AiErrorKind::ReconnectNeeded);
            assert!(http.requests().is_empty());
        }
        let (_root, mut auth) = fixture();
        let http = SequencedHttpClient::new([]);
        auth.http = DynHttpClient::new(http.clone());
        chat_cache(&auth, true);
        assert_eq!(
            auth.client(&selection(Provider::Chatgpt), CancellationToken::new())
                .await
                .err()
                .unwrap()
                .kind,
            AiErrorKind::ReconnectNeeded
        );
        assert!(http.requests().is_empty());
    }

    #[tokio::test]
    async fn disconnect_deletes_only_target_provider_and_owned_client_releases_lock() {
        let (_root, auth) = fixture();
        chat_cache(&auth, false);
        write(&auth, "github-token", b"SYNTHETIC_GITHUB");
        write(&auth, "copilot.json", b"{}");
        write(&auth, "copilot-name.json", br#"{"name":"synthetic"}"#);
        write(&auth, "chatgpt-name.json", br#"{"name":"synthetic"}"#);
        let client = auth
            .client(&selection(Provider::Chatgpt), CancellationToken::new())
            .await
            .unwrap();
        let pending_stream = async move {
            let _client = client;
            std::future::pending::<()>().await;
        };
        tokio::pin!(pending_stream);
        assert!(futures::poll!(&mut pending_stream).is_pending());
        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            auth.disconnect(Provider::Chatgpt),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(!auth.dir.join("chatgpt.json").exists());
        assert!(!auth.dir.join("chatgpt-name.json").exists());
        assert!(auth.dir.join("github-token").exists());
        assert!(auth.dir.join("copilot.json").exists());
        assert!(auth.dir.join("copilot-name.json").exists());
        auth.disconnect(Provider::Copilot).await.unwrap();
        assert!(!auth.dir.join("github-token").exists());
        assert!(!auth.dir.join("copilot.json").exists());
        assert!(!auth.dir.join("copilot-name.json").exists());
    }

    #[tokio::test]
    async fn stale_copilot_cache_does_not_block_chatgpt_or_explicit_disconnect() {
        let (_root, auth) = fixture();
        chat_cache(&auth, false);
        write(&auth, "chatgpt-name.json", br#"{"name":"synthetic"}"#);
        write(&auth, "github-token", b"SYNTHETIC_PARTIAL_LOGIN");
        write(&auth, "copilot.json", b"malformed partial login");
        write(&auth, "copilot-name.json", b"malformed display name");
        write(&auth, "unrecognized", b"leave unchanged");
        let stale = auth.dir.join("github-token");
        std::fs::set_permissions(&stale, std::fs::Permissions::from_mode(0o644)).unwrap();
        let chat_before = std::fs::read(auth.dir.join("chatgpt.json")).unwrap();
        let reopened = Auth::open(&auth.dir)
            .unwrap()
            .with_http(SequencedHttpClient::new([]));
        assert!(reopened.status(Provider::Chatgpt).await.unwrap().connected);
        reopened
            .client(&selection(Provider::Chatgpt), CancellationToken::new())
            .await
            .unwrap();
        reopened
            .models(Provider::Chatgpt, CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(
            reopened.status(Provider::Copilot).await.unwrap_err().kind,
            AiErrorKind::UnsafeCredentials
        );
        assert_eq!(
            reopened
                .client(&selection(Provider::Copilot), CancellationToken::new())
                .await
                .err()
                .unwrap()
                .kind,
            AiErrorKind::UnsafeCredentials
        );
        assert_eq!(std::fs::metadata(&stale).unwrap().mode() & 0o7777, 0o644);
        reopened.disconnect(Provider::Copilot).await.unwrap();
        for name in Provider::Copilot.files() {
            assert!(!reopened.dir.join(name).try_exists().unwrap());
        }
        assert_eq!(
            std::fs::read(auth.dir.join("chatgpt.json")).unwrap(),
            chat_before
        );
        assert_eq!(
            std::fs::read(auth.dir.join("chatgpt-name.json")).unwrap(),
            br#"{"name":"synthetic"}"#
        );
        assert_eq!(
            std::fs::read(auth.dir.join("unrecognized")).unwrap(),
            b"leave unchanged"
        );
        reopened.disconnect(Provider::Copilot).await.unwrap();
    }

    #[tokio::test]
    async fn disconnect_unlinks_owned_symlinks_and_hardlinks_without_touching_targets() {
        for dangling in [false, true] {
            let (root, auth) = fixture();
            let target = root.path().join("outside");
            if !dangling {
                std::fs::write(&target, b"SYNTHETIC_OUTSIDE").unwrap();
                std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o644)).unwrap();
            }
            symlink(&target, auth.dir.join("github-token")).unwrap();
            let hardlink_target = root.path().join("hardlink-target");
            std::fs::write(&hardlink_target, b"SYNTHETIC_HARDLINK").unwrap();
            std::fs::set_permissions(&hardlink_target, std::fs::Permissions::from_mode(0o644))
                .unwrap();
            std::fs::hard_link(&hardlink_target, auth.dir.join("copilot.json")).unwrap();
            assert!(auth.validate(Provider::Copilot).is_err());
            auth.disconnect(Provider::Copilot).await.unwrap();
            assert!(std::fs::symlink_metadata(auth.dir.join("github-token")).is_err());
            assert!(std::fs::symlink_metadata(auth.dir.join("copilot.json")).is_err());
            if !dangling {
                assert_eq!(std::fs::read(&target).unwrap(), b"SYNTHETIC_OUTSIDE");
                assert_eq!(std::fs::metadata(&target).unwrap().mode() & 0o7777, 0o644);
            } else {
                assert!(!target.exists());
            }
            assert_eq!(
                std::fs::read(&hardlink_target).unwrap(),
                b"SYNTHETIC_HARDLINK"
            );
            assert_eq!(
                std::fs::metadata(&hardlink_target).unwrap().mode() & 0o7777,
                0o644
            );
        }
    }

    #[tokio::test]
    async fn disconnect_refuses_directories_and_unsafe_folders_before_deleting() {
        let (_root, auth) = fixture();
        write(&auth, "github-token", b"SYNTHETIC_GITHUB");
        write(&auth, "copilot.json", b"{}");
        let directory = auth.dir.join("copilot-name.json");
        std::fs::create_dir(&directory).unwrap();
        assert_eq!(
            auth.disconnect(Provider::Copilot).await.unwrap_err().kind,
            AiErrorKind::UnsafeCredentials
        );
        assert!(auth.dir.join("github-token").exists());
        assert!(auth.dir.join("copilot.json").exists());
        assert!(directory.is_dir());
        std::fs::remove_dir(directory).unwrap();
        std::fs::set_permissions(&auth.dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(
            auth.disconnect(Provider::Copilot).await.unwrap_err().kind,
            AiErrorKind::UnsafeCredentials
        );
        assert!(auth.dir.join("github-token").exists());
        assert!(auth.dir.join("copilot.json").exists());
    }

    #[test]
    fn disconnect_metadata_policy_refuses_foreign_entries_and_non_files() {
        for (file, symlink) in [(true, false), (false, true)] {
            assert!(validate_removal(file, symlink, current_uid()).is_ok());
            assert_eq!(
                validate_removal(file, symlink, current_uid() + 1)
                    .unwrap_err()
                    .kind,
                AiErrorKind::UnsafeCredentials
            );
        }
        assert!(validate_removal(false, false, current_uid()).is_err());
        assert!(validate_metadata(false, true, 2, current_uid() + 1, 0o700, true).is_err());
    }

    fn copilot_login_responses() -> Vec<MockHttpResponse> {
        vec![
            MockHttpResponse::success(
                r#"{"device_code":"SYNTHETIC_DEVICE","user_code":"SYNTHETIC_CODE","verification_uri":"https://example.invalid/device","interval":1,"expires_in":900}"#,
            ),
            MockHttpResponse::success(r#"{"access_token":"SYNTHETIC_GITHUB"}"#),
            MockHttpResponse::success(
                r#"{"token":"SYNTHETIC_SESSION","expires_at":4102444800,"endpoints":{"api":"https://example.invalid"}}"#,
            ),
            MockHttpResponse::success(r#"{"login":"synthetic-user"}"#),
        ]
    }

    #[tokio::test]
    async fn real_copilot_connect_caches_name_and_tightens_all_files() {
        let (_root, mut auth) = fixture();
        let http = SequencedHttpClient::new(copilot_login_responses());
        auth.http = DynHttpClient::new(http.clone());
        let prompts = Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured = prompts.clone();
        let status = auth
            .connect(
                Provider::Copilot,
                Arc::new(move |p| {
                    captured.lock().unwrap().push(p);
                }),
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert!(status.connected);
        assert_eq!(status.name.as_deref(), Some("synthetic-user"));
        assert_eq!(prompts.lock().unwrap()[0].user_code, "SYNTHETIC_CODE");
        assert_eq!(http.remaining_responses(), 0);
        let requests = http.requests();
        assert_eq!(requests[3].uri, "https://api.github.com/user");
        assert_eq!(
            requests[3].headers["authorization"],
            "Bearer SYNTHETIC_GITHUB"
        );
        for name in Provider::Copilot.files() {
            assert_eq!(
                std::fs::metadata(auth.dir.join(name))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o7777,
                0o600
            );
        }
        assert_eq!(
            auth.status(Provider::Copilot).await.unwrap().name,
            status.name
        );
    }

    #[tokio::test]
    async fn login_error_tightens_partial_cache_and_does_not_retain_http_body() {
        let (_root, mut auth) = fixture();
        let mut responses = copilot_login_responses();
        responses.truncate(2);
        responses.push(MockHttpResponse::error(
            StatusCode::TOO_MANY_REQUESTS,
            r#"{"error":{"resets_in_seconds":42},"access_token":"SYNTHETIC_SECRET"}"#,
        ));
        auth.http = DynHttpClient::new(SequencedHttpClient::new(responses));
        let error = auth
            .connect(
                Provider::Copilot,
                Arc::new(|_| {}),
                CancellationToken::new(),
            )
            .await
            .unwrap_err();
        assert_eq!(error.kind, AiErrorKind::RateLimited);
        assert_eq!(error.retry_after_seconds, Some(42));
        assert!(!format!("{error:?} {error}").contains("SYNTHETIC_SECRET"));
        assert_eq!(
            std::fs::metadata(auth.dir.join("github-token"))
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o600
        );
    }

    #[tokio::test]
    async fn code_expiry_has_retry_copy_without_automatic_retry() {
        let (_root, mut auth) = fixture();
        let mut responses = copilot_login_responses();
        responses.truncate(1);
        responses.push(MockHttpResponse::success(
            r#"{"error":"expired_token","error_description":"SYNTHETIC_SECRET"}"#,
        ));
        let http = SequencedHttpClient::new(responses);
        auth.http = DynHttpClient::new(http.clone());
        let error = auth
            .connect(
                Provider::Copilot,
                Arc::new(|_| {}),
                CancellationToken::new(),
            )
            .await
            .unwrap_err();
        assert_eq!(error.kind, AiErrorKind::CodeExpired);
        assert!(error.to_string().contains("Retry"));
        assert_eq!(http.requests().len(), 2);
        assert!(!format!("{error:?} {error}").contains("SYNTHETIC_SECRET"));
    }

    #[tokio::test]
    async fn chatgpt_models_are_fixed_and_copilot_discovery_failure_is_not_fallback() {
        let (_root, mut auth) = fixture();
        let options = auth
            .models(Provider::Chatgpt, CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].id, "gpt-5.5");
        assert!(!options[0].live_qualified);
        write(&auth, "github-token", b"SYNTHETIC_GITHUB");
        let http = SequencedHttpClient::new([
            copilot_login_responses()[2].clone(),
            MockHttpResponse::error(StatusCode::FORBIDDEN, "SYNTHETIC_SECRET"),
        ]);
        auth.http = DynHttpClient::new(http.clone());
        let error = auth
            .models(Provider::Copilot, CancellationToken::new())
            .await
            .unwrap_err();
        assert_eq!(error.kind, AiErrorKind::ReconnectNeeded);
        assert_eq!(http.requests().len(), 2);
        assert!(http.requests()[1].uri.ends_with("/models"));
    }

    #[derive(Clone)]
    struct Gate {
        suffix: &'static str,
        entered: Arc<tokio::sync::Notify>,
        release: CancellationToken,
    }

    impl rig::http_client::HttpMiddleware for Gate {
        fn before_request_headers<'a>(
            &'a self,
            _: &'a rig::http_client::Method,
            uri: &'a rig::http_client::Uri,
            _: &'a mut rig::http_client::HeaderMap,
        ) -> rig_core::wasm_compat::WasmBoxedFuture<'a, rig::http_client::Result<()>> {
            Box::pin(async move {
                if uri.path().ends_with(self.suffix) {
                    self.entered.notify_one();
                    self.release.cancelled().await;
                }
                Ok(())
            })
        }
    }

    fn gate(suffix: &'static str) -> Gate {
        Gate {
            suffix,
            entered: Arc::new(tokio::sync::Notify::new()),
            release: CancellationToken::new(),
        }
    }

    #[tokio::test]
    async fn cancellation_tightens_partial_login_and_disconnect_cannot_recreate_it() {
        let (_root, mut auth) = fixture();
        let gate = gate("/copilot_internal/v2/token");
        let http = SequencedHttpClient::new(copilot_login_responses());
        auth.http = DynHttpClient::new(http.clone()).with_middleware(gate.clone());
        let cancel = CancellationToken::new();
        let operation = auth.connect(Provider::Copilot, Arc::new(|_| {}), cancel.clone());
        tokio::pin!(operation);
        tokio::select! {
            _ = gate.entered.notified() => {},
            result = &mut operation => panic!("unexpected early result: {result:?}"),
        }
        assert!(auth.dir.join("github-token").exists());
        cancel.cancel();
        assert_eq!(operation.await.unwrap_err().kind, AiErrorKind::Other);
        assert_eq!(
            std::fs::metadata(auth.dir.join("github-token"))
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o600
        );
        auth.disconnect(Provider::Copilot).await.unwrap();
        gate.release.cancel();
        tokio::task::yield_now().await;
        assert!(!auth.dir.join("github-token").exists());
        assert!(!auth.dir.join("copilot.json").exists());
        assert_eq!(http.requests().len(), 2);
    }

    #[tokio::test]
    async fn other_provider_login_and_status_progress_while_chatgpt_login_waits() {
        let (_root, mut auth) = fixture();
        let gate = gate("/api/accounts/deviceauth/usercode");
        auth.http = DynHttpClient::new(SequencedHttpClient::new(copilot_login_responses()))
            .with_middleware(gate.clone());
        let cancel = CancellationToken::new();
        let chatgpt = auth.connect(Provider::Chatgpt, Arc::new(|_| {}), cancel.clone());
        tokio::pin!(chatgpt);
        tokio::select! {
            _ = gate.entered.notified() => {},
            result = &mut chatgpt => panic!("unexpected early result: {result:?}"),
        }
        assert!(!auth.status(Provider::Copilot).await.unwrap().connected);
        assert!(
            auth.connect(
                Provider::Copilot,
                Arc::new(|_| {}),
                CancellationToken::new()
            )
            .await
            .unwrap()
            .connected
        );
        cancel.cancel();
        assert!(chatgpt.await.is_err());
        assert!(auth.status(Provider::Copilot).await.unwrap().connected);
    }

    #[tokio::test]
    async fn models_network_wait_does_not_hold_auth_lock() {
        let (_root, mut auth) = fixture();
        write(&auth, "github-token", b"SYNTHETIC_GITHUB");
        let gate = gate("/models");
        auth.http = DynHttpClient::new(SequencedHttpClient::new([
            copilot_login_responses()[2].clone(),
            MockHttpResponse::success(r#"{"data":[{"id":"gpt-5.5"}]}"#),
        ]))
        .with_middleware(gate.clone());
        let cancel = CancellationToken::new();
        let models = auth.models(Provider::Copilot, cancel.clone());
        tokio::pin!(models);
        tokio::select! {
            _ = gate.entered.notified() => {},
            result = &mut models => panic!("unexpected early result: {result:?}"),
        }
        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            auth.status(Provider::Copilot),
        )
        .await
        .unwrap()
        .unwrap();
        cancel.cancel();
        assert!(models.await.is_err());
    }

    #[tokio::test]
    async fn refresh_is_serialized_and_disconnect_waits_for_its_finalization() {
        let (_root, mut auth) = fixture();
        chat_cache(&auth, true);
        let mut cache = auth.json("chatgpt.json").unwrap().unwrap();
        cache["refresh_token"] = "SYNTHETIC_REFRESH".into();
        write(&auth, "chatgpt.json", &serde_json::to_vec(&cache).unwrap());
        let gate = gate("/oauth/token");
        let token = jwt(serde_json::json!({"exp":4102444800_i64}));
        auth.http = DynHttpClient::new(SequencedHttpClient::new([MockHttpResponse::success(
            serde_json::json!({"access_token":token}).to_string(),
        )]))
        .with_middleware(gate.clone());
        let chosen = selection(Provider::Chatgpt);
        let first = auth.client(&chosen, CancellationToken::new());
        tokio::pin!(first);
        tokio::select! {
            _ = gate.entered.notified() => {},
            _ = &mut first => panic!("refresh unexpectedly finished"),
        }
        let disconnect = auth.disconnect(Provider::Chatgpt);
        tokio::pin!(disconnect);
        assert!(futures::poll!(&mut disconnect).is_pending());
        gate.release.cancel();
        let (client, removed) = tokio::join!(first, disconnect);
        drop(client.unwrap());
        removed.unwrap();
        assert!(!auth.dir.join("chatgpt.json").exists());
        tokio::task::yield_now().await;
        assert!(!auth.dir.join("chatgpt.json").exists());
    }

    #[tokio::test]
    async fn unsafe_replacement_during_login_never_reports_success() {
        let (root, mut auth) = fixture();
        let gate = gate("/copilot_internal/v2/token");
        auth.http = DynHttpClient::new(SequencedHttpClient::new(copilot_login_responses()))
            .with_middleware(gate.clone());
        let cancel = CancellationToken::new();
        let operation = auth.connect(Provider::Copilot, Arc::new(|_| {}), cancel.clone());
        tokio::pin!(operation);
        tokio::select! {
            _ = gate.entered.notified() => {},
            result = &mut operation => panic!("unexpected early result: {result:?}"),
        }
        let target = root.path().join("target");
        std::fs::write(&target, b"synthetic").unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o644)).unwrap();
        std::fs::remove_file(auth.dir.join("github-token")).unwrap();
        symlink(&target, auth.dir.join("github-token")).unwrap();
        cancel.cancel();
        assert_eq!(
            operation.await.unwrap_err().kind,
            AiErrorKind::UnsafeCredentials
        );
        assert_eq!(
            std::fs::metadata(target).unwrap().permissions().mode() & 0o7777,
            0o644
        );
    }

    #[tokio::test]
    async fn empty_token_and_invalid_expiry_are_reconnect_not_success_or_panic() {
        for cache in [
            r#"{"access_token":"","expires_at":4102444800}"#,
            r#"{"access_token":"SYNTHETIC","expires_at":-9223372036854775808}"#,
            r#"{"access_token":42,"expires_at":4102444800}"#,
        ] {
            let (_root, mut auth) = fixture();
            auth.http = DynHttpClient::new(SequencedHttpClient::new([]));
            write(&auth, "chatgpt.json", cache.as_bytes());
            let result = auth
                .client(&selection(Provider::Chatgpt), CancellationToken::new())
                .await;
            assert_eq!(result.err().unwrap().kind, AiErrorKind::ReconnectNeeded);
        }
    }

    #[tokio::test]
    async fn empty_oauth_reply_does_not_create_a_success_shaped_client() {
        let (_root, mut auth) = fixture();
        chat_cache(&auth, true);
        let mut cache = auth.json("chatgpt.json").unwrap().unwrap();
        cache["refresh_token"] = "SYNTHETIC_REFRESH".into();
        write(&auth, "chatgpt.json", &serde_json::to_vec(&cache).unwrap());
        auth.http = DynHttpClient::new(SequencedHttpClient::new([MockHttpResponse::success(
            r#"{"access_token":""}"#,
        )]));
        assert_eq!(
            auth.client(&selection(Provider::Chatgpt), CancellationToken::new())
                .await
                .err()
                .unwrap()
                .kind,
            AiErrorKind::ReconnectNeeded
        );
    }

    #[tokio::test]
    async fn real_chatgpt_connect_and_refresh_tighten_caches_without_default_printing() {
        let (_root, mut auth) = fixture();
        let access = jwt(serde_json::json!({"exp":4102444800_i64}));
        let identity = jwt(serde_json::json!({"email":"synthetic@example.invalid"}));
        let response = serde_json::json!({
            "access_token":access,"refresh_token":"SYNTHETIC_REFRESH","id_token":identity
        })
        .to_string();
        let http = SequencedHttpClient::new([
            MockHttpResponse::success(
                r#"{"device_auth_id":"SYNTHETIC_DEVICE","user_code":"SYNTHETIC_CODE","interval":1}"#,
            ),
            MockHttpResponse::success(
                r#"{"authorization_code":"SYNTHETIC_AUTH","code_verifier":"SYNTHETIC_VERIFIER"}"#,
            ),
            MockHttpResponse::success(response.clone()),
            MockHttpResponse::success(response),
        ]);
        auth.http = DynHttpClient::new(http.clone());
        let prompts = Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen = prompts.clone();
        let status = auth
            .connect(
                Provider::Chatgpt,
                Arc::new(move |p| {
                    seen.lock().unwrap().push(p);
                }),
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_eq!(status.name.as_deref(), Some("synthetic@example.invalid"));
        assert_eq!(
            prompts.lock().unwrap()[0].verification_uri,
            "https://auth.openai.com/codex/device"
        );
        assert_eq!(prompts.lock().unwrap()[0].user_code, "SYNTHETIC_CODE");
        let mut cache = auth.json("chatgpt.json").unwrap().unwrap();
        cache["expires_at"] = 1.into();
        write(&auth, "chatgpt.json", &serde_json::to_vec(&cache).unwrap());
        let chosen = selection(Provider::Chatgpt);
        let client = auth
            .client(&chosen, CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(client.selection(), &chosen);
        assert_eq!(http.requests().len(), 4);
        assert!(
            String::from_utf8_lossy(&http.requests()[3].body).contains("grant_type=refresh_token")
        );
        assert_eq!(http.remaining_responses(), 0);
        for name in Provider::Chatgpt.files() {
            assert_eq!(
                std::fs::metadata(auth.dir.join(name))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o7777,
                0o600
            );
        }
    }

    #[tokio::test]
    async fn dropping_login_future_also_tightens_partial_cache() {
        let (_root, mut auth) = fixture();
        let gate = gate("/copilot_internal/v2/token");
        auth.http = DynHttpClient::new(SequencedHttpClient::new(copilot_login_responses()))
            .with_middleware(gate.clone());
        {
            let operation = auth.connect(
                Provider::Copilot,
                Arc::new(|_| {}),
                CancellationToken::new(),
            );
            tokio::pin!(operation);
            tokio::select! {
                _ = gate.entered.notified() => {},
                result = &mut operation => panic!("unexpected early result: {result:?}"),
            }
        }
        assert_eq!(
            std::fs::metadata(auth.dir.join("github-token"))
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o600
        );
        auth.disconnect(Provider::Copilot).await.unwrap();
    }

    #[tokio::test]
    async fn cancelled_lock_waiter_never_touches_cache_or_transport() {
        let (_root, mut auth) = fixture();
        chat_cache(&auth, false);
        let before = std::fs::read(auth.dir.join("chatgpt.json")).unwrap();
        let http = SequencedHttpClient::new([]);
        auth.http = DynHttpClient::new(http.clone());
        let _guard = auth.chatgpt.lock().await;
        let cancel = CancellationToken::new();
        cancel.cancel();
        assert!(
            auth.client(&selection(Provider::Chatgpt), cancel)
                .await
                .is_err()
        );
        assert_eq!(
            std::fs::read(auth.dir.join("chatgpt.json")).unwrap(),
            before
        );
        assert!(http.requests().is_empty());
    }

    #[tokio::test]
    async fn missing_identity_is_explicit_and_permission_drift_is_refused() {
        let (_root, auth) = fixture();
        write(
            &auth,
            "chatgpt.json",
            br#"{"access_token":"SYNTHETIC_ACCESS","expires_at":4102444800}"#,
        );
        let status = auth.status(Provider::Chatgpt).await.unwrap();
        assert!(status.connected);
        assert_eq!(status.name, None);
        std::fs::set_permissions(
            auth.dir.join("chatgpt.json"),
            std::fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        assert_eq!(
            auth.status(Provider::Chatgpt).await.unwrap_err().kind,
            AiErrorKind::UnsafeCredentials
        );
        auth.disconnect(Provider::Chatgpt).await.unwrap();
        assert!(!auth.dir.join("chatgpt.json").exists());
    }

    #[tokio::test]
    async fn copilot_discovery_returns_only_the_actual_authenticated_list() {
        let (_root, mut auth) = fixture();
        write(&auth, "github-token", b"SYNTHETIC_GITHUB");
        let http = SequencedHttpClient::new([
            copilot_login_responses()[2].clone(),
            MockHttpResponse::success(r#"{"data":[{"id":"synthetic-model"},{"id":"gpt-5.5"}]}"#),
        ]);
        auth.http = DynHttpClient::new(http.clone());
        let options = auth
            .models(Provider::Copilot, CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(
            options.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
            vec!["synthetic-model", "gpt-5.5"]
        );
        assert!(options.iter().all(|m| !m.live_qualified));
        assert_eq!(
            http.requests()[1].headers["authorization"],
            "Bearer SYNTHETIC_SESSION"
        );
    }

    #[test]
    fn selection_and_provider_wire_values_are_explicit_and_validated() {
        assert_eq!(
            serde_json::to_value(selection(Provider::Chatgpt)).unwrap(),
            serde_json::json!({"provider":"chatgpt","model":"gpt-5.5"})
        );
        assert!(serde_json::from_str::<Provider>(r#""unknown""#).is_err());
        for model in ["", " gpt-5.5", "gpt-5.5\n", "SYNTHETIC_SECRET🚫"] {
            assert!(
                Selection {
                    provider: Provider::Copilot,
                    model: model.into()
                }
                .validate()
                .is_err()
            );
        }
        assert!(
            Selection {
                provider: Provider::Chatgpt,
                model: "gpt-5.4".into()
            }
            .validate()
            .is_err()
        );
        assert!(selection(Provider::Chatgpt).validate().is_ok());
        assert!(
            Selection {
                provider: Provider::Copilot,
                model: "explicit-model".into()
            }
            .validate()
            .is_ok()
        );
    }

    #[test]
    fn in_repository_and_symlinked_ancestor_paths_are_rejected() {
        let here = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(Auth::open(&here.join("not-created-credentials")).is_err());
        assert!(!here.join("not-created-credentials").exists());
        let root = test_root();
        let dir = root.path().join("credentials");
        std::fs::create_dir(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        let link = root.path().join("ancestor");
        symlink(root.path(), &link).unwrap();
        assert!(Auth::open(&link.join("credentials")).is_err());
    }

    #[tokio::test]
    async fn malformed_display_metadata_is_storage_not_expired_login() {
        let (_root, auth) = fixture();
        write(&auth, "github-token", b"SYNTHETIC_GITHUB");
        write(&auth, "copilot-name.json", b"{bad SYNTHETIC_SECRET");
        let error = auth.status(Provider::Copilot).await.unwrap_err();
        assert_eq!(error.kind, AiErrorKind::Storage);
        assert!(!format!("{error:?} {error}").contains("SYNTHETIC_SECRET"));
    }

    #[tokio::test]
    async fn malformed_github_token_is_reconnect_not_connected() {
        let (_root, auth) = fixture();
        write(&auth, "github-token", &[0xff, 0xfe]);
        let error = auth.status(Provider::Copilot).await.unwrap_err();
        assert_eq!(error.kind, AiErrorKind::ReconnectNeeded);
    }
}
