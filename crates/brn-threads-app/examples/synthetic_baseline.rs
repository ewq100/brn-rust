//! Opt-in qualification; never uses a default BRN data directory.
#[path = "synthetic_baseline/support.rs"]
mod support;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let usage = "Usage: synthetic_baseline create --root /absolute/NEW-root [--notes 500] [--source COMMIT]; or synthetic_baseline measure --root /absolute/fixture-root";
    let command = args.first().ok_or(usage)?;
    let mut root = None;
    let mut notes = 500;
    let mut source = "unspecified; record source identity alongside results".to_string();
    let (options, remainder) = args[1..].as_chunks::<2>();
    for pair in options {
        match pair[0].as_str() {
            "--root" => root = Some(std::path::PathBuf::from(&pair[1])),
            "--notes" if command == "create" => notes = pair[1].parse()?,
            "--source" if command == "create" => source.clone_from(&pair[1]),
            _ => return Err(usage.into()),
        }
    }
    if !remainder.is_empty() {
        return Err(usage.into());
    }
    let root = root.ok_or(usage)?;
    let report = match command.as_str() {
        "create" => serde_json::to_value(support::create(&root, notes, &source)?)?,
        "measure" => support::measure(&root)?,
        _ => return Err(usage.into()),
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
