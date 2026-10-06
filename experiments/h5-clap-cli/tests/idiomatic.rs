//! Witnesses for idiomatic clap 4.6.7 configuration: each behavior below
//! differs from BRN's contract and is why the candidate needs an adapter.
use clap::{error::ErrorKind, Arg, ArgAction, Command};

fn run(cmd: Command, argv: &[&str]) -> Result<clap::ArgMatches, ErrorKind> {
    cmd.no_binary_name(true)
        .try_get_matches_from(argv)
        .map_err(|e| e.kind())
}

fn value(name: &'static str) -> Arg {
    Arg::new(name).long(name).action(ArgAction::Set)
}

#[test]
fn propagated_global_value_silently_overwrites_across_levels() {
    let cmd = Command::new("brn")
        .arg(value("data-dir").global(true))
        .subcommand(Command::new("status"));
    let m = run(cmd, &["--data-dir", "/a", "status", "--data-dir", "/b"]).unwrap();
    // BRN refuses this as `duplicate option: --data-dir`.
    assert_eq!(m.get_one::<String>("data-dir").unwrap(), "/b");
}

#[test]
fn propagated_global_flag_repeats_across_levels() {
    let cmd = Command::new("brn")
        .arg(
            Arg::new("json")
                .long("json")
                .action(ArgAction::SetTrue)
                .global(true),
        )
        .subcommand(Command::new("status"));
    // BRN refuses this as `duplicate option: --json`.
    assert!(run(cmd, &["--json", "status", "--json"]).is_ok());
}

fn with_help_action() -> Command {
    let help = || Arg::new("help").long("help").action(ArgAction::Help);
    Command::new("brn")
        .disable_help_flag(true)
        .arg(help())
        .subcommand(Command::new("status").disable_help_flag(true).arg(help()))
}

#[test]
fn help_action_loses_to_an_earlier_invalid_token() {
    // The action does work on its own...
    assert_eq!(
        run(with_help_action(), &["status", "--help"]).unwrap_err(),
        ErrorKind::DisplayHelp
    );
    // ...but BRN prints help for any exact `--help` token.
    assert_eq!(
        run(with_help_action(), &["status", "--bogus", "--help"]).unwrap_err(),
        ErrorKind::UnknownArgument
    );
}

#[test]
fn default_values_refuse_single_dash_text() {
    let cmd = Command::new("brn").arg(value("title"));
    // BRN accepts `--title "- agenda"` and `--title -draft`.
    assert_eq!(
        run(cmd, &["--title", "-draft"]).unwrap_err(),
        ErrorKind::UnknownArgument
    );
}

#[test]
fn hyphen_values_also_accept_double_dash_options() {
    let cmd = Command::new("brn")
        .arg(value("limit").allow_hyphen_values(true))
        .arg(Arg::new("json").long("json").action(ArgAction::SetTrue));
    let m = run(cmd, &["--limit", "--json"]).unwrap();
    // BRN refuses this as `missing value for --limit`.
    assert_eq!(m.get_one::<String>("limit").unwrap(), "--json");
}

#[test]
fn hyphen_positional_absorbs_following_options() {
    let cmd = Command::new("brn")
        .arg(Arg::new("rest").num_args(0..).allow_hyphen_values(true))
        .arg(value("limit"));
    let m = run(cmd, &["q", "--limit", "3"]).unwrap();
    assert_eq!(m.get_one::<String>("limit"), None);
    assert_eq!(m.get_many::<String>("rest").unwrap().count(), 3);
}

#[test]
fn bare_escape_is_consumed() {
    let cmd = Command::new("brn").arg(Arg::new("query").num_args(0..));
    let m = run(cmd, &["--", "--bogus"]).unwrap();
    // BRN refuses `--` as `unknown option: --`.
    assert_eq!(m.get_one::<String>("query").unwrap(), "--bogus");
}

#[test]
fn repeated_set_true_flag_is_refused() {
    let cmd = Command::new("brn").arg(
        Arg::new("version")
            .long("version")
            .action(ArgAction::SetTrue),
    );
    // BRN accepts a repeated --version.
    assert_eq!(
        run(cmd, &["--version", "--version"]).unwrap_err(),
        ErrorKind::ArgumentConflict
    );
}

#[test]
fn errors_carry_no_subcommand_path() {
    let cmd = || {
        Command::new("brn")
            .subcommand(Command::new("status"))
            .subcommand(Command::new("inbox").subcommand(Command::new("add")))
    };
    // BRN's envelope names the command reached; clap reports the same error
    // and context at the root, a leaf and a nested leaf.
    let errors: Vec<_> = [
        &["--bogus"][..],
        &["status", "--bogus"],
        &["inbox", "add", "--bogus"],
    ]
    .into_iter()
    .map(|argv| {
        let error = cmd()
            .no_binary_name(true)
            .try_get_matches_from(argv)
            .unwrap_err();
        let context: Vec<_> = error
            .context()
            .map(|(kind, value)| format!("{kind:?}={value}"))
            .collect();
        (error.kind(), context)
    })
    .collect();
    assert_eq!(
        errors[0],
        (
            ErrorKind::UnknownArgument,
            vec!["InvalidArg=--bogus".to_string()]
        )
    );
    assert!(errors.iter().all(|e| *e == errors[0]));
}
