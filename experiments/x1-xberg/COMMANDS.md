# Commands and reproduction

Qualification commands ran on 15 October 2026 at `/Users/evokessler/Documents/Codex/brn-rust/.worktrees/x1-xberg`. The current worktree is `/Users/evokessler/repos/brn-rust/.worktrees/x1-xberg`, created in a fresh clone after the owner selected that location. Captured command paths preserve the original execution location; relative reproduction commands work from the current worktree. All Cargo invocations used the existing 1.98.1 binaries and `CARGO_TARGET_DIR=/private/tmp/brn-x1-target` (except harmless metadata/tree invocations where the variable has no build effect). No root Cargo command was run.

## Preflight and source reads

- `git fetch origin` in `/Users/evokessler/repos/brn-threads` (matching repository remote).
- `git show origin/main:<path>` for AGENTS.md, docs/status.md, the modular-foundation README, final master-plan sections 8 and 11, final intake prompt, development setup/verification/workflow and existing Rust Luna intake plan. An earlier overlapping read saw the stale ref before fetch finished; required reads were repeated at `7e9d0376`.
- `git status --short`, `git remote -v`, `git worktree list --porcelain`, `git rev-parse origin/main`, `git branch --list work/x1-xberg`.
- `git worktree add .worktrees/x1-xberg -b work/x1-xberg origin/main` in the existing primary checkout. Requested `/Users/evokessler/repos/brn-rust` was absent.
- `sed`/`rg`/`cat` reads of qualification.rs, pdf_tools.rs, prepared.rs, lib.rs, process.rs, fixture inventories, fixture README and `docs/work/active/rust-luna-intake/{evidence,source-qa}.md`.
- `sw_vers`; direct pinned `rustc -Vv`; `pdftotext -v`; `pdfimages -v`; `pdftoppm -v`; process samples with `ps -axo pid,ppid,etime,command`.
- Environment deviation: `rustup toolchain list`, issued outside the repository before selecting the pin, auto-installed the missing default stable toolchain. It was not used; repository pins were not changed. No deliberate toolchain/system installation was performed afterwards.

## Acquisition and graph

```sh
export PATH=/Users/evokessler/.rustup/toolchains/1.98.1-aarch64-apple-darwin/bin:$PATH
export CARGO_TARGET_DIR=/private/tmp/brn-x1-target
cargo fetch --manifest-path experiments/x1-xberg/Cargo.toml
# After adding only the already-resolved serde_json utility dependency:
cargo fetch --manifest-path experiments/x1-xberg/Cargo.toml --offline
python3 experiments/x1-xberg/graph.py
```

`graph.py` saves exact outputs of `cargo metadata --locked --offline --features candidate --format-version 1` both with and without `--filter-platform aarch64-apple-darwin`; `cargo tree -d`; and `cargo tree -e features`. Initial equivalent metadata/tree commands preceded the script. Native build scripts and package manifests were inspected before the first build. `cargo tree ... -i image` explained the image-codec graph. Acquisition logs and lock checksums are retained. No pin was upgraded.

## Offline cold build and size

```sh
python3 experiments/x1-xberg/build.py
```

The target directory did not exist before this run. The script uses four jobs and runs these sequentially inside `/usr/bin/sandbox-exec -p '(version 1) (allow default) (deny network*)'`:

```sh
cargo build --manifest-path experiments/x1-xberg/Cargo.toml --locked --offline --release --bin baseline
cargo build --manifest-path experiments/x1-xberg/Cargo.toml --locked --offline --release --bin xberg-probe --features candidate
otool -L /private/tmp/brn-x1-target/release/baseline
otool -L /private/tmp/brn-x1-target/release/xberg-probe
/usr/bin/strip -S -o /private/tmp/brn-x1-target/release/baseline.apple-stripped /private/tmp/brn-x1-target/release/baseline
/usr/bin/strip -S -o /private/tmp/brn-x1-target/release/xberg-probe.apple-stripped /private/tmp/brn-x1-target/release/xberg-probe
```

Exact absolute commands, timestamps, exit statuses, sizes and process samples are in evidence/builds.json, build-*.log, binary-sizes.json and processes-*.txt. Concurrent Z1 compilation makes timings provisional. A rerun without clearing the task target is incremental, not a cold measurement; do not delete another task's artifacts.

## PDF comparison and deterministic source checks

```sh
/Users/evokessler/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3 experiments/x1-xberg/compare.py
python3 experiments/x1-xberg/check_inventory.py
```

For each tracked PDF, `compare.py` logs exact commands and durations in evidence/comparison-commands.json. It runs Xberg with `runtime.sb` using a fresh environment with PATH and owned TMPDIR only, and runs:

```sh
pdftotext -bbox-layout -enc UTF-8 INPUT.pdf -
pdftotext -layout -enc UTF-8 INPUT.pdf -
pdfimages -list INPUT.pdf
pdfimages -png INPUT.pdf OWNED/image
pdftoppm -f 1 -l PAGE_COUNT -scale-to 1600 -png INPUT.pdf OWNED/preview
```

The Poppler source-line projection mirrors the production isolated-row rule for these trusted XML outputs; this is not an invocation of the production helper. Raw XML/layout output remains available for independent inspection. `PdfRenderSession` renders at 150 DPI; Pillow downsamples to a 1600-pixel long edge, without a native/runtime download. RGB pixel hashes compare embedded images after decoding, independent of PNG compression.

A Pillow command generated the five `visual-page-*.png` pairs from saved previews. Every pair was inspected with the local image viewer; no UI automation was used. The hand-checked source-inventory.json records the process fixture's exact rows, nine scan units, heading/list counts and vector diagram.

The pinned `rustc --edition 2024 -O src/bin/sandbox_witness.rs -o /private/tmp/brn-x1-target/sandbox-witness` compiled the separate canary. Its exact profile/loopback/file command and result are in evidence/sandbox/result.json. The listening socket was a local synthetic Python witness; no provider was contacted.

## Final verification

```sh
cargo fmt --manifest-path experiments/x1-xberg/Cargo.toml
cargo fmt --manifest-path experiments/x1-xberg/Cargo.toml --check
cargo clippy --manifest-path experiments/x1-xberg/Cargo.toml --locked --offline --features candidate --bins -- -D warnings
python3 experiments/x1-xberg/check_inventory.py
git diff --check
git diff --name-only 7e9d0376d677ed2a46bdbcd8933f55f9e1746a02
git status --short
```

The experiment includes direct deterministic checks instead of adding production tests. No BRN workspace/native UI/provider test, deliberate package installation, push, PR, merge or gate-3 design was run. The unintended default-stable acquisition is disclosed above.

## Owner-selected fresh-clone location

After qualification, the owner's answer selected a fresh clone at the originally requested path. These commands created it and transferred the existing local commit without a push:

```sh
git clone https://github.com/ewq100/brn-rust.git /Users/evokessler/repos/brn-rust
cd /Users/evokessler/repos/brn-rust
git fetch --no-tags /Users/evokessler/Documents/Codex/brn-rust refs/heads/work/x1-xberg:refs/heads/work/x1-xberg
git worktree add .worktrees/x1-xberg work/x1-xberg
cd /Users/evokessler/repos/brn-rust/.worktrees/x1-xberg
python3 experiments/x1-xberg/check_inventory.py
```

The fresh clone has its own Git object store (no alternates); its only configured remote is the original GitHub origin. `/.worktrees/` was added only to the fresh clone's local `.git/info/exclude`. All 132 transferred experiment files were compared byte-for-byte with the preserved checkout before documentation changes; recorded source hashes and fixture inventories passed again. See [relocation evidence](evidence/relocation.md). Build/PDF extraction benchmarks were not repeated or relabeled; the original snapshot and gate evidence remain unchanged. Only location documentation and relocation verification records were added in the new worktree.
