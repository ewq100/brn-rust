//! Bounded synthetic probes. Running with --live requires owner authorization.
use brn_ai::capability_probe::{ProbeKind, ProbeReport, ProbeTerminal, run_probe};
use brn_ai::{AiError, AiErrorKind, Auth, Provider, Selection};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

const USAGE: &str = "usage: provider-capabilities --live chatgpt|copilot ABSOLUTE_CREDENTIALS_DIR EXACT_MODEL low|high|image|web\n\
Runs a bounded synthetic probe against an already connected, task-specific account.\n\
Owner authorization is required for provider calls; there is no default provider or model.";

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

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
