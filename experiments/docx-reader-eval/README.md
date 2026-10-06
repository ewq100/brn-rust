# docx-rs reader evaluation (H4, throwaway)

Checks whether published `docx-rs` 0.4.22 could replace part of BRN's DOCX
interpretation. This is not product code and nothing depends on it. Result:
not adopted. See [FINDINGS.md](FINDINGS.md).

It has its own manifest, lockfile and `[workspace]`. Use a separate target and
run it from this directory:

    export CARGO_TARGET_DIR=/private/tmp/brn-h4-eval/target
    cargo run --locked --release            # full matrix as a Markdown table
    cargo run --locked --release -- S3 M7   # selected case IDs only

Every fixture is built in memory from synthetic XML, except
`fixtures/ordinary.jpg`, an 8x8 synthetic gradient that macOS `sips` converted
from a generated PNG (SHA-256 `91f47ed4ebc45c6fad07623d4c10181c46b2756fdff11bff290fffc4c63db7a1`).
Each case asserts its recorded verdict; the process exits 1 on any difference.

Optional environment:

- `DOCX_EVAL_DUMP=1` prints each serialized read model to stderr.
- `DOCX_EVAL_FIXTURES=DIR` writes every evaluated package to `DIR/<case>-<n>.docx`
  so BRN's converter can replay the same bytes (procedure in FINDINGS).

Case B1 asks docx-rs to allocate 3.75 GiB. That works on macOS because the
memory is committed lazily; on a host with strict overcommit the run may abort.
