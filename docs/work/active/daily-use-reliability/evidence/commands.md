# Command record

This is a reproduction record of existing gates, not an additional test harness.
The archived `environment.sh.gz` records the pinned cloud paths. All data is
synthetic. `container-subreaper.py.gz` records the process-local reaping wrapper.

The initial integrated commands and source pin are in
`integrated-commands.sh.gz`. Format, build and strict Clippy completed; the full
workspace run failed synthetic credential admission and was terminated after
three barrier waits. Later stages were executed separately as below; no complete
local full-suite success is claimed.

```sh
cargo test -p brn-ai --lib useful_intake_model_tools --locked --offline
python3 -m unittest discover -s scripts/tests -v
bash scripts/verify-end-to-end.sh --fixtures-only
cargo test -p brn --features scripted-document-tests --bin brn document_route_tests --locked --offline --jobs 1
cargo clippy --workspace --all-targets --features brn/scripted-document-tests --locked --offline -- -D warnings
cargo test -p brn-threads-core -p brn-intake -p brn-threads-intake -p brn-retrieval -p brn-writing-ui --no-fail-fast --locked --offline
cargo test -p brn-ai --features scripted-runs --lib --no-run --locked --offline
```

The scripted-document command genuinely selects one Linux test after the cfg
correction. Its `UnsafeCredentials` failure is recorded as blocked qualification.
The `--no-run` command is compilation only; it is not counted as passing tests.

Both qualified helper binaries are supplied through `BRN_XBERG_HELPER` and
`BRN_XBERG_HELPER_BRN` and copied beside the CLI. The helper artifact/source
receipts bind their identity and hashes. Standalone helper tests and Clippy use
the separate Stock/adapted locked manifests and targets recorded in their logs.

Final focused app checks on correction head `3393d4e0`:

```sh
cargo test -p brn-threads-app --test document_conversion_settings figures_profile_reimport_preserves_old_note_and_deduplicates_current_variant --locked --offline
cargo test -p brn-threads-app --test daily_workflow_eval --test wiki_eval_seed --locked --offline
```

The three exact test names in `barrier-diagnostics.json` were then invoked from
the compiled test binary with `--exact NAME --nocapture`, each under an external
10-second diagnostic cap. Each returned the expected explicit credential error
in under 0.1 seconds; this demonstrates bounded failure, not successful behavior.
