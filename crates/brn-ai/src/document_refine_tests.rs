use super::*;
use brn_threads_intake::{Coverage, CoverageStatus, RetainedAsset};
use rig::test_utils::{MockHttpResponse, SequencedHttpClient};
use serde_json::{Value, json};
use std::os::unix::fs::PermissionsExt;
fn capture(name: &str, value: impl serde::Serialize) {
    if let Some(path) = std::env::var_os("BRN_TASK2_CAPTURES") {
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(
            PathBuf::from(path).join(format!("{name}.json")),
            serde_json::to_vec_pretty(&value).unwrap(),
        )
        .unwrap();
    }
}
fn root() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}
fn helpers() -> DocumentHelpers {
    let base = PathBuf::from("/private/tmp/brn-x2-standard-task1-20261018/targets/artifacts");
    DocumentHelpers {
        stock: base.join("brn-xberg-helper"),
        brn: base.join("brn-xberg-helper-brn"),
    }
}
fn cases() -> Vec<(DocumentKind, Vec<u8>)> {
    vec![
        (
            DocumentKind::Pdf,
            include_bytes!("../../brn-threads-intake/tests/fixtures/harbor-study.pdf").to_vec(),
        ),
        (
            DocumentKind::Docx,
            include_bytes!("../../brn-intake/tests/fixtures/harbor.docx").to_vec(),
        ),
        (
            DocumentKind::Pptx,
            include_bytes!("../../brn-intake/tests/fixtures/quay.pptx").to_vec(),
        ),
        (
            DocumentKind::Xlsx,
            include_bytes!("../../brn-intake/tests/fixtures/forecast.xlsx").to_vec(),
        ),
        (
            DocumentKind::Eml,
            include_bytes!("../../brn-intake/tests/fixtures/single.eml").to_vec(),
        ),
        (DocumentKind::Html, b"<p>Synthetic HTML</p>".to_vec()),
        (DocumentKind::Text, b"Exact text\r\n".to_vec()),
        (DocumentKind::Markdown, b"# Exact Markdown\n".to_vec()),
    ]
}
fn job(
    root: &tempfile::TempDir,
    converter: DocumentConverter,
    ai: bool,
    kind: DocumentKind,
    bytes: Vec<u8>,
) -> DocumentConversionJob {
    DocumentConversionJob {
        converter: Converter {
            helper_path: "/never-legacy".into(),
            pdf_tools: None,
            temp_root: root.path().join("scratch"),
            limits: brn_intake::IntakeLimits {
                wall_time_ms: 20000,
                ..Default::default()
            },
        },
        helpers: helpers(),
        settings: DocumentConversionSettings { converter, ai },
        kind,
        intent: ImportIntent::FullNote,
        bytes,
        source_reference: "synthetic://document-job".into(),
        auth_file: Some(root.path().join("bad-auth-must-not-open")),
        credentials_dir: None,
    }
}
fn auth(root: &tempfile::TempDir) -> PathBuf {
    let path = root.path().join("auth.json");
    std::fs::write(&path,r#"{"auth_mode":"chatgpt","tokens":{"access_token":"synthetic-not-a-credential","account_id":"synthetic"}}"#).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    path
}
fn completed(text: &str, status: &str) -> String {
    let response = json!({"type":"response.completed","sequence_number":1,"response":{"id":"synthetic-response","object":"response","created_at":1,"status":status,"model":"gpt-6-luna","incomplete_details":if status=="incomplete" {json!({"reason":"max_output_tokens"})} else {Value::Null},"output":[{"type":"message","id":"synthetic-message","status":"completed","role":"assistant","content":[{"type":"output_text","annotations":[],"text":text}]}],"tools":[],"usage":{"input_tokens":23,"output_tokens":17,"total_tokens":40}}});
    format!("data: {response}\n\ndata: [DONE]\n\n")
}
fn transport(text: &str, status: &str) -> SequencedHttpClient {
    SequencedHttpClient::new([MockHttpResponse::success_typed(
        completed(text, status),
        "text/event-stream",
    )])
}
fn result() -> ConversionResult {
    ConversionResult {
        intent: ImportIntent::FullNote,
        markdown: "Exact source".into(),
        assets: vec![],
        source_sha256: brn_intake::digest(b"source"),
        source_reference: "synthetic://source".into(),
        converter: document_conversion_label("xberg", false).unwrap(),
        sources: vec![],
        coverage: Coverage {
            status: CoverageStatus::Unchecked,
            checked_items: vec![],
            gaps: vec![],
        },
    }
}
fn response(converted: &ConversionResult) -> Refinement {
    Refinement {
        schema: 1,
        source_sha256: converted.source_sha256,
        markdown: converted.markdown.clone(),
        uncertainty: String::new(),
    }
}
#[tokio::test]
async fn ai_off_never_opens_auth_for_either_engine_all_eight() {
    let root = root();
    std::fs::write(
        root.path().join("bad-auth-must-not-open"),
        b"definitely invalid auth",
    )
    .unwrap();
    for converter in [DocumentConverter::Xberg, DocumentConverter::XbergBrn] {
        for (kind, bytes) in cases() {
            let http = SequencedHttpClient::new([]);
            let expected_hash = brn_intake::digest(&bytes);
            let converted = convert_job(
                job(&root, converter, false, kind, bytes),
                CancellationToken::new(),
                Arc::new(AtomicBool::new(false)),
                Arc::new(|_| {}),
                Some(DynHttpClient::new(http.clone())),
                TIMEOUT,
            )
            .await
            .unwrap();
            capture(&format!("off-{converter:?}-{kind:?}"), &converted);
            assert!(http.requests().is_empty());
            assert_eq!(converted.source_sha256, expected_hash);
            assert_eq!(
                converted.converter,
                document_conversion_label(
                    if converter == DocumentConverter::Xberg {
                        "xberg"
                    } else {
                        "xberg_brn"
                    },
                    false
                )
                .unwrap()
            );
        }
    }
}
#[tokio::test]
async fn ai_on_receives_selected_engine_output_for_every_kind() {
    let root = root();
    let file = auth(&root);
    for converter in [DocumentConverter::Xberg, DocumentConverter::XbergBrn] {
        for (kind, bytes) in cases() {
            let baseline = job(&root, converter, false, kind, bytes.clone());
            let extracted = baseline
                .converter
                .convert_document(
                    ConversionRequest {
                        kind,
                        bytes: &bytes,
                        intent: baseline.intent,
                        source_reference: &baseline.source_reference,
                        inventory: None,
                    },
                    &baseline.helpers,
                    backend(&baseline.settings),
                    &AtomicBool::new(false),
                )
                .unwrap();
            let mut reply = response(&extracted);
            reply.uncertainty = "Synthetic inspection remains unchecked".into();
            let http = transport(&serde_json::to_string(&reply).unwrap(), "completed");
            let mut input = job(&root, converter, true, kind, bytes);
            input.auth_file = Some(file.clone());
            let actual = convert_job(
                input,
                CancellationToken::new(),
                Arc::new(AtomicBool::new(false)),
                Arc::new(|_| {}),
                Some(DynHttpClient::new(http.clone())),
                TIMEOUT,
            )
            .await
            .unwrap();
            assert_eq!(actual.markdown, extracted.markdown);
            assert_eq!(actual.assets, extracted.assets);
            assert_eq!(actual.source_reference, extracted.source_reference);
            assert_eq!(actual.source_sha256, extracted.source_sha256);
            assert_eq!(
                actual.converter,
                document_conversion_label(
                    if converter == DocumentConverter::Xberg {
                        "xberg"
                    } else {
                        "xberg_brn"
                    },
                    true
                )
                .unwrap()
            );
            assert_eq!(actual.coverage.status, CoverageStatus::Unchecked);
            let requests = http.requests();
            assert_eq!(requests.len(), 1);
            let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
            capture(
                &format!("on-{converter:?}-{kind:?}"),
                json!({"extracted":extracted,"refined":actual,"request_body":body}),
            );
            assert_eq!(body["model"], "gpt-6-luna");
            assert_eq!(body["reasoning"], json!({"effort":"high"}));
            assert!(
                body.get("tools")
                    .is_none_or(|v| v.as_array().is_some_and(Vec::is_empty))
            );
            let input = body["input"].as_array().unwrap();
            assert_eq!(input.len(), 1);
            let prompt = input[0]["content"][0]["text"].as_str().unwrap();
            assert!(prompt.contains(&serde_json::to_string(&extracted.markdown).unwrap()));
            assert!(
                !body["instructions"]
                    .as_str()
                    .unwrap()
                    .contains(&extracted.markdown)
            );
        }
    }
}
#[tokio::test]
async fn invalid_hash_fields_terminal_and_auth_fail_without_candidate() {
    let root = root();
    let file = auth(&root);
    let extracted = result();
    for kind in 0..5 {
        let mut reply = serde_json::to_value(response(&extracted)).unwrap();
        match kind {
            0 => reply["source_sha256"] = json!(vec![0; 32]),
            1 => reply["unexpected"] = json!(true),
            2 => reply["markdown"] = json!(""),
            _ => {}
        }
        let text = if kind == 4 {
            "not JSON".into()
        } else {
            reply.to_string()
        };
        let http = transport(&text, if kind == 3 { "incomplete" } else { "completed" });
        assert!(
            refine(
                extracted.clone(),
                Some(&file),
                None,
                CancellationToken::new(),
                Some(DynHttpClient::new(http.clone())),
                TIMEOUT
            )
            .await
            .is_err()
        );
        assert_eq!(http.requests().len(), 1);
    }
    assert!(matches!(
        refine_document(extracted.clone(), None, None, CancellationToken::new()).await,
        Err(ConversionFailure::Provider(_))
    ));
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert!(matches!(
        refine_document(extracted, Some(&file), None, cancel).await,
        Err(ConversionFailure::Interrupted)
    ));
}
#[test]
fn retained_assets_and_finite_fields_are_checked_without_reextracting() {
    let mut extracted = result();
    extracted.markdown = "Figure ![one](asset.png) and ![repeat](asset.png)".into();
    extracted.assets.push(RetainedAsset {
        name: "asset.png".into(),
        media_type: "image/png".into(),
        sha256: [0; 32],
        width: 1,
        height: 1,
        bytes: vec![],
    });
    let mut reply = response(&extracted);
    assert!(validate(&extracted, &reply).is_ok());
    reply.markdown = "Figure ![one](asset.png)".into();
    assert!(validate(&extracted, &reply).is_err());
    reply.markdown = "Figure ![one](elsewhere.png) and ![repeat](asset.png)".into();
    assert!(validate(&extracted, &reply).is_err());
    reply = response(&extracted);
    reply.uncertainty = "x".repeat(4097);
    assert!(validate(&extracted, &reply).is_err());
}

#[derive(Clone)]
struct StalledHttp(SequencedHttpClient);
impl rig::http_client::HttpClientExt for StalledHttp {
    fn send<T, U>(
        &self,
        request: rig::http_client::Request<T>,
    ) -> impl Future<
        Output = rig::http_client::Result<
            rig::http_client::Response<rig::http_client::LazyBody<U>>,
        >,
    > + Send
    + 'static
    where
        T: Into<bytes::Bytes> + Send,
        U: From<bytes::Bytes> + Send + 'static,
    {
        self.0.send(request)
    }
    fn send_multipart<U>(
        &self,
        request: rig::http_client::Request<rig::http_client::MultipartForm>,
    ) -> impl Future<
        Output = rig::http_client::Result<
            rig::http_client::Response<rig::http_client::LazyBody<U>>,
        >,
    > + Send
    + 'static
    where
        U: From<bytes::Bytes> + Send + 'static,
    {
        self.0.send_multipart(request)
    }
    fn send_streaming<T>(
        &self,
        request: rig::http_client::Request<T>,
    ) -> impl Future<Output = rig::http_client::Result<rig::http_client::StreamingResponse>> + Send
    where
        T: Into<bytes::Bytes> + Send,
    {
        let response = self.0.send_streaming(request);
        async move {
            let (parts, _) = response.await?.into_parts();
            Ok(rig::http_client::Response::from_parts(
                parts,
                Box::pin(futures::stream::pending()) as rig::http_client::BoxedStream,
            ))
        }
    }
}
#[tokio::test]
async fn refinement_deadline_stop_refusal_and_failure_have_no_save_or_fallback() {
    let root = root();
    let file = auth(&root);
    for stopping in [false, true] {
        let http =
            SequencedHttpClient::new([MockHttpResponse::success_typed("", "text/event-stream")]);
        let cancel = CancellationToken::new();
        let observed = cancel.clone();
        let stopper = stopping.then(|| {
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(20)).await;
                observed.cancel();
            })
        });
        let actual = refine(
            result(),
            Some(&file),
            None,
            cancel,
            Some(DynHttpClient::new(StalledHttp(http.clone()))),
            Duration::from_millis(100),
        )
        .await;
        if stopping {
            assert!(matches!(actual, Err(ConversionFailure::Interrupted)))
        } else {
            assert!(matches!(actual, Err(ConversionFailure::Timeout)))
        };
        assert_eq!(http.requests().len(), 1);
        if let Some(stopper) = stopper {
            stopper.await.unwrap();
        }
    }
    let store = brn_threads_core::Store::open(root.path().join("canonical")).unwrap();
    let records = store.records().unwrap();
    let history = store.receipts().unwrap();
    for converter in [DocumentConverter::Xberg, DocumentConverter::XbergBrn] {
        let http = SequencedHttpClient::new([MockHttpResponse::error(
            rig::http_client::StatusCode::BAD_REQUEST,
            r#"{"error":{"code":"model_not_supported","message":"synthetic refusal"}}"#,
        )]);
        let mut input = job(
            &root,
            converter,
            true,
            DocumentKind::Text,
            b"No hidden fallback".to_vec(),
        );
        input.auth_file = Some(file.clone());
        assert!(matches!(
            convert_job(
                input,
                CancellationToken::new(),
                Arc::new(AtomicBool::new(false)),
                Arc::new(|_| {}),
                Some(DynHttpClient::new(http.clone())),
                TIMEOUT
            )
            .await,
            Err(ConversionFailure::Provider(_))
        ));
        assert_eq!(http.requests().len(), 1);
        assert_eq!(store.records().unwrap(), records);
        assert_eq!(store.receipts().unwrap(), history);
    }
}
