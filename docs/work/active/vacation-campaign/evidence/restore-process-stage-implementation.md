# Restore process-stage witness implementation

Commit: `2f3f65c884d18919a5553b18b7bbe72ccf0962a5` on `codex/restore-process-interruption`.

Changed only `crates/brn-threads-core/src/restore.rs`, extending the existing Unix test. Its child re-executes the same exact unit test under `BRN_TEST_RESTORE_PROCESS_STAGE_ROOT`, builds a complete stage through unchanged `StagedRestore::create`, reports that stage path after the copy, and holds the owner lock. The parent checks that the reported path is a direct restore-stage child of its own canonical temporary fixture, snapshots its full tree, and performs a public restore into a sibling while the child lives. The stage must remain byte-for-byte unchanged. The parent then kills and reaps the owned child and requires Unix signal 9; it verifies the full stage remains and the child destination was never published. A final public restore sweeps the killed child stage and the original same-process full-stage and wrapper-abandon witnesses. Existing exact backup bytes, previous completed tree, unrelated stage-like directory, empty records and no published stage marker assertions remain. The child has a 30-second fail-safe; the parent's readiness deadline is 10 seconds and its RAII guard kills/reaps on panic paths.

Commands actually run:

- Read `AGENTS.md`, the assigned restore-stage design, `implementing-product-tasks/SKILL.md`, its shared runtime resource policy, and the target source/stage implementation.
- `source /tmp/brn-launcher-focus-qualification/environment.sh && rustfmt --edition 2024 crates/brn-threads-core/src/restore.rs`
- `source /tmp/brn-launcher-focus-qualification/environment.sh && rustfmt --edition 2024 --check crates/brn-threads-core/src/restore.rs && git diff --check`
- Inspected the final one-file diff and status, then committed the file.

No tests, builds, native checks, provider/account/network activity, or access to non-fixture data were performed. The focused test, broader suite and lint remain for the lead's serialized gate. This source witness is limited to Unix SIGKILL stage-lock ownership and later sweep behavior; it does not establish interruption during copying, power-loss behavior or hardware durability.

Correction pass requested by lead's independent source review:

- Correction commit: `f4650db41dde3c1460af946aa59244f74a481671`.
- Readiness publication now writes the complete stage path to a same-parent temporary file and atomically renames it to the ready path, removing the parent `exists`/read partial-write race. The parent reconstructs the path with Unix `OsStringExt::from_vec`, preserving arbitrary path bytes.
- The same-process complete-copy stage is now prepared and abandoned while the child still holds its stage lock. Its prepare/sweep therefore cannot remove the killed child's stage after termination. The parent explicitly requires the killed child stage to exist immediately before the final public `Store::restore`; that restore itself must remove it. Wrapper abandon remains via `StageDir::create`, which does not sweep.
- The sibling public restore's full resulting tree is captured and checked again after the final restore.
- Correction commands actually run: `source /tmp/brn-launcher-focus-qualification/environment.sh && rustfmt --edition 2024 crates/brn-threads-core/src/restore.rs && rustfmt --edition 2024 --check crates/brn-threads-core/src/restore.rs && git diff --check`; inspected the correction diff/status and committed only `crates/brn-threads-core/src/restore.rs`.
- No test, build or lint was run in this correction pass; lead owns those gates.
