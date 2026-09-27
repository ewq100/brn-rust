use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args_os();
    let _program = args.next();

    match (args.next(), args.next()) {
        (Some(flag), None) if flag == "--version" => {
            println!("brn {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        (Some(flag), None) if flag == "--help" => {
            println!("brn {}\n\nUsage: brn [--help | --version]\n\nOptions:\n  --help     Show this help\n  --version  Show the version", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        (None, None) => {
            eprintln!("error: expected --help or --version\n\nRun `brn --help` for usage.");
            ExitCode::FAILURE
        }
        _ => {
            eprintln!("error: unknown arguments\n\nRun `brn --help` for usage.");
            ExitCode::FAILURE
        }
    }
}
