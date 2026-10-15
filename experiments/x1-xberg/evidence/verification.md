# Final verification

15 October 2026, Rust 1.98.1, snapshot 7e9d0376d677ed2a46bdbcd8933f55f9e1746a02.

- Cold offline release: baseline and actual Xberg probe passed with network denied; time measurements provisional due to concurrent Z1 build. See builds.json and contention.json.
- Final incremental offline release after rustfmt: passed with network denied; final-release-build.json/log. Same byte sizes; cold and final hashes are separately recorded.
- `cargo fmt --manifest-path experiments/x1-xberg/Cargo.toml --check`: passed.
- `cargo clippy --manifest-path experiments/x1-xberg/Cargo.toml --locked --offline --features candidate --bins -- -D warnings`: passed; clippy.log (20.55 seconds).
- Both fixture runs repeated using the final binary under deny-default runtime.sb: passed; comparison-final-run.log and comparison-commands.json.
- `python3 experiments/x1-xberg/check_inventory.py`: passed, including exact complete source sequences, explicit process-page-2 permutation, intentionally detected raw-order mismatch, empty native scan text, fixture SHA bindings and exact embedded-image RGB pixels.
- All five original/final page pairs visually inspected; independent reviewer confirmed inventoried prose, rows, figures and nine scan units. No native application or Luna inference acceptance is claimed.
- Same-profile excluded-file / live-loopback denial and allowed scratch-write witness passed.
- Experiment Markdown links and final source-hash consistency checks passed.
- Independent plan/evidence reviews approved; four small evidence/checker findings corrected and independently verified. See final-review.md.

Git scope and staged whitespace verification passed before the local commit: every staged path is under experiments/x1-xberg. The local .gitattributes exempts only raw Poppler page text, build logs and the copied build-script source from whitespace diagnostics, preserving their original bytes. Authored scripts and report retain ordinary whitespace checking.

## Fresh-clone verification

The owner subsequently selected `/Users/evokessler/repos/brn-rust`. The fresh clone's `.worktrees/x1-xberg` contains the exact original local X1 commit and unchanged qualification snapshot. Before documentation edits, all 132 transferred experiment files were byte-identical to the preserved checkout. Recorded source hashes and `check_inventory.py` passed at the new location; relocation-checks.log records the check output. Original benchmark/extraction evidence was preserved. See relocation.md for setup and scope details.
