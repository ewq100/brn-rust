//! Seeds one wiki-guidance evaluation case (fixture v2) into a fresh explicit
//! workspace through checked owner writes and prints a JSON key map to stdout.
//! Synthetic data only; never touches a default BRN data directory.
#[path = "wiki_eval_seed/seed.rs"]
mod seed;

use std::path::PathBuf;

const USAGE: &str = "usage: wiki_eval_seed --data-dir ABS_FRESH_DIR --fixture PATH --case ID [--filler N] [--seed S]\n       wiki_eval_seed --warm ABS_DATA_DIR  (native build; embeds every passage with BRN_EMBEDDING_MODEL_DIR)";

/// Opens an existing seeded workspace and waits in-process until the dense
/// lane covers every passage, so a template starts with semantic coverage
/// `complete`. The CLI never waits for idle-time embedding.
fn warm(data: &std::path::Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut workspace = brn_threads_app::Workspace::open(data)?;
    let model = workspace
        .embedding_model_from_env()
        .ok_or("set BRN_EMBEDDING_MODEL_DIR to an installed model folder")??;
    let handle = workspace.search_handle()?;
    let started = std::time::Instant::now();
    loop {
        handle.prepare().wait()?;
        if let Some((with, all)) = handle.embedding_progress()
            && with >= all
        {
            return Ok(serde_json::json!({
                "model": model,
                "passages": all,
                "seconds": started.elapsed().as_secs_f64(),
            })
            .to_string());
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

fn parse() -> Result<seed::Args, String> {
    let mut args = std::env::args().skip(1);
    let (mut data_dir, mut fixture, mut case) = (None, None, None);
    let (mut filler, mut seed_value) = (0usize, 1u64);
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("{flag} needs a value\n{USAGE}"))?;
        match flag.as_str() {
            "--data-dir" => data_dir = Some(PathBuf::from(value)),
            "--fixture" => fixture = Some(PathBuf::from(value)),
            "--case" => case = Some(value),
            "--filler" => filler = value.parse().map_err(|_| format!("bad --filler {value}"))?,
            "--seed" => seed_value = value.parse().map_err(|_| format!("bad --seed {value}"))?,
            other => return Err(format!("unknown argument {other}\n{USAGE}")),
        }
    }
    Ok(seed::Args {
        data_dir: data_dir.ok_or(format!("--data-dir is required\n{USAGE}"))?,
        fixture: fixture.ok_or(format!("--fixture is required\n{USAGE}"))?,
        case: case.ok_or(format!("--case is required\n{USAGE}"))?,
        filler,
        seed: seed_value,
    })
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    if let [flag, data] = argv.as_slice()
        && flag == "--warm"
    {
        match warm(std::path::Path::new(data)) {
            Ok(report) => println!("{report}"),
            Err(error) => {
                eprintln!("wiki_eval_seed: {error}");
                std::process::exit(1);
            }
        }
        return;
    }
    let result = parse()
        .map_err(Into::into)
        .and_then(|args| seed::run(&args));
    match result {
        Ok(report) => println!(
            "{}",
            serde_json::to_string(&report).expect("report serializes")
        ),
        Err(error) => {
            eprintln!("wiki_eval_seed: {error}");
            std::process::exit(1);
        }
    }
}
