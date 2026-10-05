//! Bounded synthetic probes. Running with --live requires owner authorization.
use brn_ai::capability_probe::{ProbeKind, ProbeReport, ProbeTerminal, run_probe};
use brn_ai::{AiError, AiErrorKind, Auth, Provider, Selection};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

const USAGE: &str = "usage: provider-capabilities --synthetic-catalog (built-in offline fixtures only)\n\
provider-capabilities --live chatgpt|copilot ABSOLUTE_CREDENTIALS_DIR EXACT_MODEL low|high|image|web\n\
Runs a bounded synthetic probe against an already connected, task-specific account.\n\
Owner authorization is required for provider calls; there is no default provider or model.";

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

/// Offline harness observation only. Never retain bodies, arbitrary metadata,
/// messages, headers, credentials or account identifiers.
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
struct CatalogObservation {
    provider: &'static str,
    envelope_shape: &'static str,
    container: &'static str,
    container_shape: &'static str,
    raw_count: Option<usize>,
    ids: Option<Vec<String>>,
}

fn shape(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

fn catalog_observation(provider: Provider, body: &serde_json::Value) -> CatalogObservation {
    let (provider_name, container, key) = match provider {
        Provider::Chatgpt => ("chatgpt", "models", "slug"),
        Provider::Copilot => ("copilot", "data", "id"),
    };
    let entries = body.get(container).and_then(serde_json::Value::as_array);
    let mut seen = std::collections::HashSet::new();
    let ids = entries.and_then(|entries| {
        entries
            .iter()
            .map(|entry| {
                let id = entry.get(key)?.as_str()?;
                let selection = Selection {
                    provider,
                    model: id.to_owned(),
                };
                (selection.validate().is_ok() && seen.insert(id)).then(|| id.to_owned())
            })
            .collect::<Option<Vec<_>>>()
    });
    CatalogObservation {
        provider: provider_name,
        envelope_shape: shape(body),
        container,
        container_shape: body.get(container).map(shape).unwrap_or("missing"),
        raw_count: entries.map(Vec::len),
        ids,
    }
}

// Built-in synthetic payloads only: this path cannot read files or open Auth.
fn synthetic_catalog_observations() -> Vec<CatalogObservation> {
    [
        (
            Provider::Chatgpt,
            serde_json::json!({"models": [
            {"slug": "synthetic-first", "visibility": "list", "priority": 2},
            {"slug": "synthetic-hidden", "visibility": "hide", "priority": 0}
        ], "metadata": "SYNTHETIC_SECRET"}),
        ),
        (
            Provider::Copilot,
            serde_json::json!({"data": [
                {"id": "synthetic-one"}, {"id": "synthetic-two"}
            ]}),
        ),
        (Provider::Chatgpt, serde_json::json!({"data": []})),
        (Provider::Chatgpt, serde_json::json!({"models": []})),
    ]
    .into_iter()
    .map(|(provider, body)| catalog_observation(provider, &body))
    .collect()
}

extern "C" fn on_sigint(_: libc::c_int) {
    INTERRUPTED.store(true, Ordering::SeqCst);
}

async fn interrupted() {
    let mut interval = tokio::time::interval(Duration::from_millis(25));
    loop {
        interval.tick().await;
        if INTERRUPTED.load(Ordering::SeqCst) {
            return;
        }
    }
}

fn auth_failure(error: AiError, cancelled: bool) -> ProbeTerminal {
    // Auth finalizes protected cache permissions before returning. A specific
    // failure, especially UnsafeCredentials, must survive simultaneous Stop.
    if cancelled && error.kind == AiErrorKind::Other {
        ProbeTerminal::Interrupted
    } else {
        ProbeTerminal::Failed(error)
    }
}

fn arguments() -> Result<(Selection, PathBuf, ProbeKind), ()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 5 || args[0] != "--live" {
        return Err(());
    }
    let provider = match args[1].to_str() {
        Some("chatgpt") => Provider::Chatgpt,
        Some("copilot") => Provider::Copilot,
        _ => return Err(()),
    };
    let credentials = PathBuf::from(&args[2]);
    let model = args[3].to_str().ok_or(())?.to_owned();
    let selection = Selection { provider, model };
    selection.validate().map_err(|_| ())?;
    if !credentials.is_absolute() {
        return Err(());
    }
    let kind = match args[4].to_str() {
        Some("low") => ProbeKind::Low,
        Some("high") => ProbeKind::High,
        Some("image") => ProbeKind::Image,
        Some("web") => ProbeKind::Web,
        _ => return Err(()),
    };
    Ok((selection, credentials, kind))
}

#[tokio::main]
async fn main() -> ExitCode {
    if std::env::args_os().skip(1).collect::<Vec<_>>() == ["--synthetic-catalog"] {
        println!(
            "{}",
            serde_json::to_string_pretty(&synthetic_catalog_observations()).unwrap()
        );
        return ExitCode::SUCCESS;
    }
    let Ok((selection, credentials, kind)) = arguments() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    // The pinned Copilot adapter routes non-Codex models through Chat Completions.
    // It has no native web-search seam; refuse before account access.
    if matches!(kind, ProbeKind::Web)
        && selection.provider == Provider::Copilot
        && !rig::providers::copilot::wire::routes_through_responses(&selection.model)
    {
        eprintln!("Native web is unavailable through this selected adapter route.");
        return ExitCode::from(2);
    }
    // SAFETY: the standalone process installs a handler that only stores to a
    // lock-free atomic. Cancellation is performed on the normal Tokio task so
    // protected refresh permissions are finalized before the process exits.
    if unsafe {
        libc::signal(
            libc::SIGINT,
            on_sigint as extern "C" fn(libc::c_int) as libc::sighandler_t,
        )
    } == libc::SIG_ERR
    {
        eprintln!("Could not install the interrupt handler.");
        return ExitCode::FAILURE;
    }
    let cancel = CancellationToken::new();
    let mut operation = std::pin::pin!(async {
        let result = async {
            if INTERRUPTED.load(Ordering::SeqCst) {
                cancel.cancel();
            }
            if cancel.is_cancelled() {
                return Err(brn_ai::AiError::new(AiErrorKind::Other));
            }
            let auth = Auth::open(&credentials)?;
            auth.client(&selection, cancel.clone()).await
        }
        .await;
        match result {
            Ok(client) => run_probe(client, kind, cancel.clone()).await,
            Err(error) => ProbeReport {
                selection: selection.clone(),
                kind,
                terminal: auth_failure(error, cancel.is_cancelled()),
                text: String::new(),
                read_tool_calls: 0,
                web_search_observed: false,
                citations: Vec::new(),
                failure: None,
            },
        }
    });
    let report = tokio::select! {
        biased;
        report = &mut operation => report,
        _ = interrupted() => {
            cancel.cancel();
            operation.await
        }
        _ = tokio::time::sleep(Duration::from_secs(120)) => {
            cancel.cancel();
            operation.await
        }
    };
    let successful = matches!(report.terminal, ProbeTerminal::Completed);
    match serde_json::to_string_pretty(&report) {
        Ok(json) => println!("{json}"),
        Err(_) => {
            // Never print an unsanitized provider/serialization error.
            eprintln!("Could not serialize the safe probe report.");
            return ExitCode::FAILURE;
        }
    }
    if successful {
        ExitCode::SUCCESS
    } else if matches!(report.terminal, ProbeTerminal::Interrupted) {
        ExitCode::from(130)
    } else if matches!(
        report.terminal,
        ProbeTerminal::Failed(brn_ai::AiError {
            kind: AiErrorKind::RateLimited,
            ..
        })
    ) {
        ExitCode::from(3)
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_discovery_retains_shape_count_and_exact_order_without_metadata() {
        let reports = synthetic_catalog_observations();
        assert_eq!(reports[0].raw_count, Some(2));
        assert_eq!(
            reports[0].ids.as_ref().unwrap(),
            &["synthetic-first", "synthetic-hidden"]
        );
        assert_eq!(
            reports[1].ids.as_ref().unwrap(),
            &["synthetic-one", "synthetic-two"]
        );
        assert_eq!(reports[2].container_shape, "missing");
        assert_eq!(reports[2].raw_count, None);
        assert_eq!(reports[2].ids, None);
        assert_eq!(reports[3].raw_count, Some(0));
        assert_eq!(reports[3].ids, Some(vec![]));
        assert!(
            !serde_json::to_string(&reports)
                .unwrap()
                .contains("SYNTHETIC_SECRET")
        );
    }

    #[test]
    fn malformed_discovery_preserves_observed_count_without_partial_ids() {
        for body in [
            serde_json::json!({"models": [{"slug":"valid"}, {"slug":"bad\n"}]}),
            serde_json::json!({"models": [{"slug":"same"}, {"slug":"same"}]}),
            serde_json::json!({"models": [{"slug":"valid"}, {"other":"SYNTHETIC_SECRET"}]}),
        ] {
            let report = catalog_observation(Provider::Chatgpt, &body);
            assert_eq!(report.raw_count, Some(2));
            assert_eq!(report.ids, None);
            assert!(
                !serde_json::to_string(&report)
                    .unwrap()
                    .contains("SYNTHETIC_SECRET")
            );
        }
        for body in [
            serde_json::json!(null),
            serde_json::json!([]),
            serde_json::json!({"models":null}),
        ] {
            let report = catalog_observation(Provider::Chatgpt, &body);
            assert_eq!(report.raw_count, None);
            assert_eq!(report.ids, None);
        }
    }

    #[test]
    fn cancellation_preserves_protected_authentication_failures() {
        for kind in [
            AiErrorKind::UnsafeCredentials,
            AiErrorKind::Storage,
            AiErrorKind::ReconnectNeeded,
            AiErrorKind::RateLimited,
        ] {
            let error = AiError::new(kind);
            assert_eq!(
                auth_failure(error.clone(), true),
                ProbeTerminal::Failed(error)
            );
        }
        assert_eq!(
            auth_failure(AiError::new(AiErrorKind::Other), true),
            ProbeTerminal::Interrupted
        );
        assert_eq!(
            auth_failure(AiError::new(AiErrorKind::Other), false),
            ProbeTerminal::Failed(AiError::new(AiErrorKind::Other))
        );
    }
}
