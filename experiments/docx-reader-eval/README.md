# DOCX reader evaluation (H4, throwaway)

Checks whether published DOCX readers could replace part of BRN's DOCX
interpretation: docx-rs 0.4.22, rdocx 0.15.0, office_oxide 0.1.13 and
betteroffice-docx-parse 0.3.0. This is not product code and nothing depends on
it. Result: none adopted. See [FINDINGS.md](FINDINGS.md).

It has its own manifest, lockfile and `[workspace]`. Use a separate target and
run it from this directory:

    export CARGO_TARGET_DIR=/private/tmp/brn-h4-eval/target
    cargo run --locked --release                    # full matrix as Markdown tables
    cargo run --locked --release -- rdocx S3 M15    # selected readers and/or case IDs

Each (case, reader) pair runs in its own child process, so an abort or hang is
recorded instead of ending the run. Each result is checked against the verdict
recorded in `src/main.rs`; the process exits 1 on any difference.

- `src/fixtures.rs` builds every package in memory from synthetic XML.
  `fixtures/ordinary.jpg` is the one exception: an 8x8 synthetic gradient that
  macOS `sips` converted from a generated PNG (SHA-256
  `91f47ed4ebc45c6fad07623d4c10181c46b2756fdff11bff290fffc4c63db7a1`).
- `src/readers.rs` holds one adapter per reader, using its public API only.
- `src/main.rs` holds the cases, the verdict rules and the process harness.

Optional environment:

- `DOCX_EVAL_DUMP=1` prints each reader's inspected output to stderr.
- `DOCX_EVAL_FIXTURES=DIR` writes every package to `DIR/<case>.docx` so BRN's
  converter can replay the same bytes (procedure in FINDINGS).

Case B1 asks docx-rs to allocate 3.75 GiB. That works on macOS because the
memory is committed lazily; on a host with strict overcommit that child may
abort, which is recorded as `Abort`.
