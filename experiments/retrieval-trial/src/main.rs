use brn_retrieval_trial::{fixtures, search_keyword, Filters, Profile, Request};
use std::{env, path::PathBuf, process::ExitCode};

fn usage() -> &'static str {
    "Usage: brn-retrieval-trial check | build --state NEW_DIR | evaluate --state DIR | reopen --state DIR\n\
     check runs offline keyword contract fixtures. Native commands require --features native."
}
fn state_arg(args: &[String]) -> Result<PathBuf, String> {
    if args.len() != 4 || args[2] != "--state" {
        return Err(usage().into());
    }
    Ok(PathBuf::from(&args[3]))
}
fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("--help" | "-h") if args.len() == 2 => {
            println!("{}", usage());
            Ok(())
        }
        Some("check") if args.len() == 2 => {
            let docs = fixtures();
            let cases = [
                ("launch window", "launch-current"),
                ("café", "unicode-note"),
                ("BRN-482", "identifier-note"),
            ];
            for (query, expected) in cases {
                let request = Request::new(query, Profile::Keyword, 3, Filters::default())
                    .map_err(|e| e.to_string())?;
                let hits = search_keyword(&docs, &request).map_err(|e| e.to_string())?;
                for hit in &hits {
                    hit.verify(&docs).map_err(|e| e.to_string())?;
                }
                let found = hits.iter().any(|h| h.source_id == expected);
                println!("keyword query={query:?} expected={expected} hit@3={found}");
                if !found {
                    return Err(format!("expected source absent: {expected}"));
                }
            }
            Ok(())
        }
        Some("build" | "evaluate" | "reopen") => {
            let state = state_arg(&args)?;
            #[cfg(feature = "native")]
            {
                let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
                let result = match args[1].as_str() {
                    "build" => rt.block_on(brn_retrieval_trial::native::build(&state)),
                    "evaluate" => rt.block_on(brn_retrieval_trial::native::evaluate(&state)),
                    _ => rt.block_on(brn_retrieval_trial::native::reopen(&state)),
                };
                result.map_err(|e| e.to_string())
            }
            #[cfg(not(feature = "native"))]
            {
                let _ = state;
                Err("native retrieval requires --features native".into())
            }
        }
        _ => Err(usage().into()),
    }
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
