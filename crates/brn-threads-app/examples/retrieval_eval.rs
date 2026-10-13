//! Opt-in retrieval evaluation over a fresh synthetic workspace; never uses a
//! default BRN data directory.
#[path = "retrieval_eval/support.rs"]
#[allow(dead_code)]
mod support;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let usage = "Usage: retrieval_eval --root /absolute/NEW-root [--notes 1000] [--seed N] [--out /absolute/report.json]";
    let args: Vec<_> = std::env::args().skip(1).collect();
    let (options, remainder) = args.as_chunks::<2>();
    if !remainder.is_empty() {
        return Err(usage.into());
    }
    let (mut root, mut notes, mut seed, mut out) = (None, 1000, support::DEFAULT_SEED, None);
    for pair in options {
        match pair[0].as_str() {
            "--root" => root = Some(std::path::PathBuf::from(&pair[1])),
            "--notes" => notes = pair[1].parse()?,
            "--seed" => seed = pair[1].parse()?,
            "--out" => out = Some(std::path::PathBuf::from(&pair[1])),
            _ => return Err(usage.into()),
        }
    }
    if !(200..=20_000).contains(&notes) {
        return Err("Use 200..=20000 notes".into());
    }
    let data = support::reserve(&root.ok_or(usage)?)?;
    let report = support::run(&data, notes, seed)?;
    let json = serde_json::to_string_pretty(&report)?;
    match out {
        Some(path) if path.is_absolute() => std::fs::write(path, json + "\n")?,
        Some(_) => return Err("--out must be absolute".into()),
        None => println!("{json}"),
    }
    eprintln!("{}", support::summary(&report));
    Ok(())
}
