use std::{env, fs, path::PathBuf};
fn main() {
    let workspace = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let identity = workspace.join("build-identity.txt");
    println!("cargo:rerun-if-changed={}", identity.display());
    let text =
        fs::read_to_string(&identity).expect("workspace-local build-identity.txt is required");
    let mut lines = text.lines();
    let backend = lines
        .next()
        .and_then(|s| s.strip_prefix("backend="))
        .expect("backend identity line");
    let engine = lines
        .next()
        .and_then(|s| s.strip_prefix("engine_identity="))
        .expect("engine identity line");
    assert!(lines.next().is_none(), "unexpected identity fields");
    assert_eq!(
        engine,
        brn_intake::document::expected_engine_identity(backend).expect("known backend")
    );
    let manifest = fs::read_to_string(workspace.join("Cargo.toml")).unwrap();
    assert_eq!(
        manifest.contains("[patch.crates-io]"),
        backend == "xberg_brn",
        "workspace patch/backend mismatch"
    );
    let source = PathBuf::from(file!());
    let source = if source.is_absolute() {
        source
    } else {
        workspace.join(source)
    };
    let repo = source
        .canonicalize()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned();
    println!("cargo:rustc-env=BRN_BACKEND={backend}");
    println!("cargo:rustc-env=BRN_ENGINE_ID={engine}");
    println!("cargo:rustc-env=BRN_REPO_ROOT={}", repo.display());
}
