//! Opt-in retrieval evaluation over a fresh synthetic workspace; never uses a
//! default BRN data directory.
#[path = "retrieval_eval/support.rs"]
#[allow(dead_code)]
mod support;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let usage = "Usage: retrieval_eval --root /absolute/NEW-root [--notes 1000] [--seed N] [--retriever current|worker|hybrid] [--out /absolute/report.json]";
    let args: Vec<_> = std::env::args().skip(1).collect();
    let (options, remainder) = args.as_chunks::<2>();
    if !remainder.is_empty() {
        return Err(usage.into());
    }
    let (mut root, mut notes, mut seed, mut out) = (None, 1000, support::DEFAULT_SEED, None);
    let mut retriever = "current".to_string();
    for pair in options {
        match pair[0].as_str() {
            "--root" => root = Some(std::path::PathBuf::from(&pair[1])),
            "--notes" => notes = pair[1].parse()?,
            "--seed" => seed = pair[1].parse()?,
            "--out" => out = Some(std::path::PathBuf::from(&pair[1])),
            "--retriever" => retriever = pair[1].clone(),
            _ => return Err(usage.into()),
        }
    }
    if !(300..=20_000).contains(&notes) {
        return Err("Use 300..=20000 notes".into());
    }
    let data = support::reserve(&root.ok_or(usage)?)?;
    let report = match retriever.as_str() {
        "current" => support::run(&data, notes, seed)?,
        "worker" => support::run_with(&data, notes, seed, &mut support::WorkerSearch::default())?,
        "hybrid" => support::run_with(&data, notes, seed, &mut support::HybridSearch::default())?,
        _ => return Err(usage.into()),
    };
    let json = serde_json::to_string_pretty(&report)?;
    match out {
        Some(path) if path.is_absolute() => std::fs::write(path, json + "\n")?,
        Some(_) => return Err("--out must be absolute".into()),
        None => println!("{json}"),
    }
    eprintln!("{}", support::summary(&report));
    Ok(())
}
