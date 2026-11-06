//! Explicit live-only no-tool grader; no default credential path or input.
use std::{path::PathBuf, process::ExitCode};

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() == 2 && args[0] == "--catalog" {
        let path = PathBuf::from(&args[1]);
        if !path.is_absolute() {
            return ExitCode::from(2);
        }
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            brn_ai::codex_auth::codex_models(&path, tokio_util::sync::CancellationToken::new()),
        )
        .await;
        match result {
            Ok(Ok(models)) => {
                let models: Vec<_> = models
                    .into_iter()
                    .filter(|m| matches!(m.id.as_str(), "gpt-6.1-sol" | "gpt-6.1-terra"))
                    .collect();
                println!(
                    "{}",
                    serde_json::json!({"kind":"read-only-BRN-Codex-catalog", "models":models})
                );
                return ExitCode::SUCCESS;
            }
            Ok(Err(error)) => eprintln!("{}", serde_json::json!({"error":error})),
            Err(_) => eprintln!("Catalogue deadline reached"),
        }
        return ExitCode::FAILURE;
    }
    if args.len() != 4 || args[0] != "--live" {
        eprintln!(
            "usage: agentbench-judge --live ABS_AUTH_FILE ABS_PACKET ABS_INSTRUCTIONS\nagentbench-judge --catalog ABS_AUTH_FILE (read-only, no model generation)"
        );
        return ExitCode::from(2);
    }
    let paths: Vec<_> = args[1..].iter().map(PathBuf::from).collect();
    if paths.iter().any(|p| !p.is_absolute()) {
        return ExitCode::from(2);
    }
    let read = |p: &PathBuf, limit| {
        let size = std::fs::metadata(p).ok()?.len();
        (size <= limit)
            .then(|| std::fs::read_to_string(p).ok())
            .flatten()
    };
    let (Some(packet), Some(instructions)) =
        (read(&paths[1], 1024 * 1024), read(&paths[2], 64 * 1024))
    else {
        eprintln!("Missing, non-text or oversized frozen input");
        return ExitCode::from(2);
    };
    let receipt = brn_ai::threads::benchmark_judge::judge(&paths[0], &packet, &instructions).await;
    println!("{receipt}");
    if receipt["status"] == "completed" {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
