# X1 qualification plan

Owner assignment: 15 October 2026; gates 1–2 only. Snapshot: 7e9d0376d677ed2a46bdbcd8933f55f9e1746a02.

1. Acquire exactly Xberg 1.3.10, disable defaults, enable only pdf. Inspect resolved features and native build/runtime paths before any build. Stop if a mandatory gate fails; classify cache/network failures as blocked.
2. Save active metadata/licenses, graph duplicates and root-lock comparison. Build minimal and actual extraction binaries offline with Rust 1.98.1 and CARGO_TARGET_DIR=/private/tmp/brn-x1-target. Record elapsed time, size, linked libraries and concurrent builds.
3. Trace schema-2 Poppler consumers, locate both PDF fixtures and independent inventories. Run deterministic Xberg/Poppler comparisons only after gate 1 passes. Save per-page source lines, page previews if supported without pdfium, and embedded images. Verify ordering, table associations, figures and scan limitations.
4. Write REPORT.md with exact commands/evidence, each gate status and residual risks; stop or recommend gate 3. Verify all changes are within experiments/x1-xberg, commit locally; do not push.

Files: Cargo.toml/Cargo.lock and src/bin for isolated harness; evidence/ for logs, metadata and fixture outputs; qualification scripts for reproducibility; REPORT.md for the decision. No production changes, provider calls, model downloads, system installs, real data or UI use.

Repository workflow calls for proportional planning and no routine reauthorization. This owner-specified qualification runs directly in this session. No production implementation or integration plan is being approved.
