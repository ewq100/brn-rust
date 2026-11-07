//! Explicit live-only no-tool grader; no default credential path or input.
use std::{path::PathBuf, process::ExitCode};

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 5 || args[0] != "--live" {
        eprintln!(
            "usage: agentbench-judge --live ABS_AUTH_FILE ABS_PACKET ABS_INSTRUCTIONS TIMEOUT_SECONDS (1..600)"
        );
        return ExitCode::from(2);
    }
    let Some(timeout_seconds) = args[4]
        .to_str()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|n| (1..=600).contains(n))
    else {
        return ExitCode::from(2);
    };
    let paths: Vec<_> = args[1..4].iter().map(PathBuf::from).collect();
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
    let receipt =
        brn_ai::threads::benchmark_judge::judge(&paths[0], &packet, &instructions, timeout_seconds)
            .await;
    println!("{receipt}");
    if receipt["status"] == "completed" {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
