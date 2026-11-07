use super::*;

#[test]
fn fallback_requires_empty_uncancelled_chat_model_refusal() {
    let cancel = CancellationToken::new();
    let mut answer = AiAnswer {
        text: String::new(),
        terminal: AiTerminal::Failed(AiError::new(AiErrorKind::ModelRefused)),
    };
    assert!(copilot_responses_retry(false, &answer, &cancel));
    assert!(!copilot_responses_retry(true, &answer, &cancel));
    answer.text = "partial output".into();
    assert!(!copilot_responses_retry(false, &answer, &cancel));
    answer.text.clear();
    for kind in [
        AiErrorKind::RateLimited,
        AiErrorKind::Network,
        AiErrorKind::ReconnectNeeded,
    ] {
        answer.terminal = AiTerminal::Failed(AiError::new(kind));
        assert!(!copilot_responses_retry(false, &answer, &cancel));
    }
    answer.terminal = AiTerminal::Failed(AiError::new(AiErrorKind::ModelRefused));
    cancel.cancel();
    assert!(!copilot_responses_retry(false, &answer, &cancel));
}

#[cfg(unix)]
mod account {
    use super::*;
    use rig::http_client::{self, HttpMiddleware};
    use rig::test_utils::{MockHttpResponse, SequencedHttpClient};
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    struct SwitchDuringAuth(PathBuf, Arc<AtomicBool>);
    impl HttpMiddleware for SwitchDuringAuth {
        fn before_request_headers<'a>(
            &'a self,
            _: http_client::middleware::Exchange<'a>,
            _: &'a mut http_client::HeaderMap,
        ) -> rig_core::wasm_compat::WasmBoxedFuture<'a, http_client::Result<()>> {
            std::fs::write(&self.0, "SYNTHETIC_OTHER_ACCOUNT").unwrap();
            self.1.store(true, Ordering::SeqCst);
            Box::pin(async { Ok(()) })
        }
    }

    #[tokio::test]
    async fn sign_in_change_during_auth_refuses_before_any_model_request() {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let dir = root.path().join("credentials");
        Auth::open(&dir).unwrap();
        let token = dir.join("github-token");
        std::fs::write(&token, "SYNTHETIC_START_ACCOUNT").unwrap();
        std::fs::set_permissions(&token, std::fs::Permissions::from_mode(0o600)).unwrap();
        let pin = Auth::open(&dir).unwrap().copilot_account().unwrap();
        let http = SequencedHttpClient::new([MockHttpResponse::success(
            r#"{"token":"SYNTHETIC_SESSION","expires_at":4102444800,"endpoints":{"api":"https://example.invalid"}}"#,
        )]);
        let switched = Arc::new(AtomicBool::new(false));
        let transport = DynHttpClient::new(http.clone())
            .with_middleware(SwitchDuringAuth(token.clone(), switched.clone()));
        let selection = Selection {
            provider: Provider::Copilot,
            model: "gpt-6-luna".into(),
        };
        let result = ProviderOperation {
            selection: &selection,
            auth_file: None,
            credentials_dir: Some(&dir),
            account: Some(&pin),
            cache_key: None,
        }
        .client(Some(transport), &CancellationToken::new())
        .await;
        assert!(
            switched.load(Ordering::SeqCst),
            "fixture must actually switch account during auth"
        );
        assert_eq!(
            std::fs::read_to_string(token).unwrap(),
            "SYNTHETIC_OTHER_ACCOUNT",
            "authentication must not overwrite a later sign-in"
        );
        let Err(error) = result else {
            panic!("changed account must refuse");
        };
        assert_eq!(error.kind, AiErrorKind::ReconnectNeeded);
        assert_eq!(
            error.detail.as_ref().unwrap().code.as_deref(),
            Some("account_changed")
        );
        assert_eq!(http.requests().len(), 1);
        assert_eq!(
            http.requests()[0].uri,
            "https://api.github.com/copilot_internal/v2/token"
        );
    }
}
