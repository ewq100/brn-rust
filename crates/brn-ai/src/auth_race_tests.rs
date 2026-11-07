//! Synthetic account mutations while authentication owns its short-lived store.
use super::*;
use chatgpt::auth::TokenStore;
use rig::test_utils::{MockHttpResponse, SequencedHttpClient};
use std::sync::atomic::{AtomicBool, Ordering};

fn setup() -> (tempfile::TempDir, Auth) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let auth = Auth::open(&root.path().join("credentials")).unwrap();
    write_record_in(&auth.dir, "github-token", b"SYNTHETIC_START_ACCOUNT").unwrap();
    (root, auth)
}

struct ChangeDuringAuth {
    token: PathBuf,
    replace: bool,
    changed: Arc<AtomicBool>,
}
impl HttpMiddleware for ChangeDuringAuth {
    fn before_request_headers<'a>(
        &'a self,
        _: rig::http_client::middleware::Exchange<'a>,
        _: &'a mut rig::http_client::HeaderMap,
    ) -> rig_core::wasm_compat::WasmBoxedFuture<'a, rig::http_client::Result<()>> {
        if self.replace {
            std::fs::write(&self.token, "SYNTHETIC_OTHER_ACCOUNT").unwrap();
        } else {
            std::fs::remove_file(&self.token).unwrap();
        }
        self.changed.store(true, Ordering::SeqCst);
        Box::pin(async { Ok(()) })
    }
}
fn account_changed_error(error: AiError) {
    assert_eq!(error.kind, AiErrorKind::ReconnectNeeded);
    assert_eq!(
        error.detail.as_ref().unwrap().code.as_deref(),
        Some("account_changed")
    );
}

#[tokio::test]
async fn sign_in_change_during_auth_refuses_and_preserves_the_newer_state() {
    for replace in [true, false] {
        let (_root, auth) = setup();
        let token = auth.dir.join("github-token");
        let http = SequencedHttpClient::new([MockHttpResponse::success(
            r#"{"token":"SYNTHETIC_SESSION","expires_at":4102444800,"endpoints":{"api":"https://example.invalid"}}"#,
        )]);
        let changed = Arc::new(AtomicBool::new(false));
        let transport = DynHttpClient::new(http.clone()).with_middleware(ChangeDuringAuth {
            token: token.clone(),
            replace,
            changed: changed.clone(),
        });
        let selection = Selection {
            provider: Provider::Copilot,
            model: "gpt-6-luna".into(),
        };
        let result = auth
            .with_http(transport)
            .client(&selection, CancellationToken::new())
            .await;
        assert!(
            changed.load(Ordering::SeqCst),
            "fixture must actually change account during auth"
        );
        if replace {
            assert_eq!(
                std::fs::read_to_string(&token).unwrap(),
                "SYNTHETIC_OTHER_ACCOUNT"
            );
        } else {
            assert!(
                !token.exists(),
                "auth must not restore a signed-out account"
            );
        }
        let Err(error) = result else {
            panic!("changed account must refuse");
        };
        account_changed_error(error);
        assert!(
            !token.with_file_name("copilot.json").exists(),
            "no stale session save"
        );
        assert_eq!(http.requests().len(), 1);
        assert_eq!(
            http.requests()[0].uri,
            "https://api.github.com/copilot_internal/v2/token"
        );
    }
}

#[tokio::test]
async fn stale_store_cannot_reload_save_or_delete_a_new_account_and_cache() {
    let (_root, auth) = setup();
    let store = LegacyTokenStore::new(auth.dir.clone());
    let old = store.load("copilot").await.unwrap().unwrap();
    write_record_in(&auth.dir, "github-token", b"SYNTHETIC_OTHER_ACCOUNT").unwrap();
    let cache = br#"{"token":"SYNTHETIC_OTHER_SESSION","expires_at":4102444800,"endpoints":null}"#;
    write_record_in(&auth.dir, "copilot.json", cache).unwrap();
    for error in [
        store.load("copilot").await.unwrap_err(),
        store.save("copilot", &old).await.unwrap_err(),
        store.delete("copilot").await.unwrap_err(),
    ] {
        account_changed_error(*error.downcast::<AiError>().unwrap());
    }
    assert_eq!(
        std::fs::read(auth.dir.join("github-token")).unwrap(),
        b"SYNTHETIC_OTHER_ACCOUNT"
    );
    assert_eq!(std::fs::read(auth.dir.join("copilot.json")).unwrap(), cache);
    // A fresh operation may use the newly selected account normally.
    let fresh = LegacyTokenStore::new(auth.dir.clone());
    let current = fresh.load("copilot").await.unwrap().unwrap();
    fresh.save("copilot", &current).await.unwrap();
}
