//! Design §7.4 guard: the CLI and the app crate never call the owner-host
//! edit functions (`begin_edit`, `update_buffer`, `save`, `discard`) or
//! `set_workspace_purpose`. Only the desktop, the owner host, may. The CLI
//! uses the caller-bound twins, which need the session's token.
//!
//! Every `*.rs` under `crates/brn/src` and `crates/brn-threads-app/src`,
//! test modules included, is scanned with all ASCII whitespace removed, so
//! rustfmt-split chains (`store\n    .save(`) still match. Path calls
//! (`Store::save(`) and `use … as` renames (`as save`) are matched too.
use std::path::{Path, PathBuf};

/// Whitespace-free patterns that call or rename an owner-host function.
const FORBIDDEN: &[&str] = &[
    ".begin_edit(",
    ".update_buffer(",
    ".save(",
    ".discard(",
    "set_workspace_purpose(",
    "Store::begin_edit(",
    "Store::update_buffer(",
    "Store::save(",
    "Store::discard(",
    "Store::set_workspace_purpose(",
    "asbegin_edit",
    "asupdate_buffer",
    "assave",
    "asdiscard",
];

/// Legitimate non-edit matches, as `(file name, pattern)`. Empty: none exist.
const ALLOWED: &[(&str, &str)] = &[];

fn stripped(text: &str) -> String {
    text.chars().filter(|c| !c.is_ascii_whitespace()).collect()
}

/// The forbidden patterns found in `text`.
fn violations(text: &str) -> Vec<&'static str> {
    let text = stripped(text);
    FORBIDDEN
        .iter()
        .copied()
        .filter(|pattern| text.contains(pattern))
        .collect()
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn the_cli_and_the_app_crate_never_call_owner_host_edit_functions() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = vec![];
    for dir in [
        manifest.join("src"),
        manifest.join("../brn-threads-app/src"),
    ] {
        rust_files(&dir, &mut files);
    }
    assert!(files.len() > 10, "{files:?}");
    let mut found = vec![];
    for file in &files {
        let name = file.file_name().unwrap().to_str().unwrap();
        for pattern in violations(&std::fs::read_to_string(file).unwrap()) {
            if !ALLOWED.contains(&(name, pattern)) {
                found.push(format!("{}: {pattern}", file.display()));
            }
        }
    }
    assert!(found.is_empty(), "owner-host calls: {found:#?}");
}

#[test]
fn every_pattern_trips_on_a_sample_and_the_bound_twins_do_not() {
    let samples = [
        ("workspace.store.begin_edit(&note, 1)", ".begin_edit("),
        ("store\n    .update_buffer(&id, 1, text)", ".update_buffer("),
        ("workspace\n    .store\n    .save(&id, 1)", ".save("),
        ("store.discard(\n    &id, 0)", ".discard("),
        (
            "store.set_workspace_purpose(WorkspacePurpose::Test)",
            "set_workspace_purpose(",
        ),
        (
            "Store::begin_edit(&mut store, &note, 1)",
            "Store::begin_edit(",
        ),
        (
            "Store::update_buffer(&mut s, &id, 1, t)",
            "Store::update_buffer(",
        ),
        ("Store::save(&mut workspace.store, &id, 1)", "Store::save("),
        ("Store :: discard (&mut s, &id, 0)", "Store::discard("),
        (
            "Store::set_workspace_purpose(&mut s, p)",
            "Store::set_workspace_purpose(",
        ),
        (
            "use brn_threads_app::Store::begin_edit as begin_edit;",
            "asbegin_edit",
        ),
        ("use x::update_buffer as update_buffer;", "asupdate_buffer"),
        ("use x::{save as save};", "assave"),
        ("use x::discard as discard;", "asdiscard"),
    ];
    assert_eq!(samples.len(), FORBIDDEN.len());
    for pattern in FORBIDDEN {
        assert!(
            samples.iter().any(|(_, p)| p == pattern),
            "no sample for {pattern}"
        );
    }
    for (sample, pattern) in samples {
        assert!(
            violations(sample).contains(&pattern),
            "{sample:?} must trip {pattern}"
        );
    }
    for bound in [
        "store.begin_caller_edit(&note, 1)",
        "store.update_caller_buffer(&id, &token, 1, text)",
        "store\n    .save_caller_edit(&id, &token, 1)",
        "store.discard_caller_edit(&id, &token, 0)",
        "store.reclaim_caller_edit(&id)",
        "contract::begin_edit_error(error)",
    ] {
        assert!(violations(bound).is_empty(), "{bound:?}");
    }
}
