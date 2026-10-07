# BetterOffice multi-format field-access probe (isolated)

2026-10-07 reassessment evidence only. No product code depends on this standalone
workspace. Pins three published BetterOffice parser crates at 0.3.0; package
checksums/VCS identities are in `package-identities.json`.

From the repository root, use the pinned Rust toolchain and isolated directories:

```sh
python3 experiments/architecture-reassessment/reuse-probe/fixtures.py
CARGO_HOME="$PWD/work/reuse-cargo" CARGO_TARGET_DIR="$PWD/work/reuse-target" cargo build --manifest-path experiments/architecture-reassessment/reuse-probe/Cargo.toml --locked --release
CARGO_HOME="$PWD/work/reuse-cargo" CARGO_TARGET_DIR="$PWD/work/reuse-target" cargo run --manifest-path experiments/architecture-reassessment/reuse-probe/Cargo.toml --locked --offline --release
CARGO_HOME="$PWD/work/reuse-cargo" CARGO_TARGET_DIR="$PWD/work/reuse-target" cargo clippy --manifest-path experiments/architecture-reassessment/reuse-probe/Cargo.toml --locked --offline -- -D warnings
cargo fmt --manifest-path experiments/architecture-reassessment/reuse-probe/Cargo.toml --check
```

The first build needs public package access unless this Cargo home is populated.
On this host cargo/rustc are in `/opt/homebrew/opt/rustup/bin`, which must be on PATH.
The fixture generator uses only Python's standard library and the repository's
synthetic inline PNG DOCX fixture. No credentials, models or provider calls.

Checks: DOCX image data URL; independently reproduced DOCX ruby and unknown-wrapper
loss while original parts remain; PPTX slide/notes text, alt and exact PNG asset;
XLSX sheet/string/number model from inflated parts. The PPTX title supplement proves
**field access only**, using `part_bytes()` plus namespace-aware quick-xml. It
returns an unbound title list: no shape/image-occurrence association is verified.
No complete Markdown adapter, round-trip fidelity, rendering, chart/SmartArt/table
or diagram visual fidelity, cancellation or large-input behavior is established.
Meaningful previews of complex visuals are required before a production adapter
can claim their content is sufficiently represented.

`results.json` records three original local executions on arm64, including the
measurement scope and cumulative child peak RSS. Tiny synthetic fixtures do not
predict production costs. `metadata.json` and `tree.txt` are the original resolved
graph, before relocation; their root manifest paths therefore refer to scratch.
