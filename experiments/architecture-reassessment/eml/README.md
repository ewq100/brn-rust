# Actual EML library probe (isolated)

2026-10-07, Rust1.98.1 on Apple Silicon; `mail-parser = 0.11.8`, `full_encoding`,
MIT OR Apache-2.0. Independent Cargo workspace/lock; outside the production graph.
Source-research of0.11.9 is separately recorded in the reassessment; this probe
makes no claim to execute that newer version. Lock checksum pins published bytes.

From the repository root with cargo/rustc on PATH:

```sh
CARGO_HOME="$PWD/work/eml-cargo-home" CARGO_TARGET_DIR="$PWD/work/eml-target" cargo run --manifest-path experiments/architecture-reassessment/eml/Cargo.toml --locked
CARGO_HOME="$PWD/work/eml-cargo-home" CARGO_TARGET_DIR="$PWD/work/eml-target" cargo run --manifest-path experiments/architecture-reassessment/eml/Cargo.toml --locked --offline
CARGO_HOME="$PWD/work/eml-cargo-home" CARGO_TARGET_DIR="$PWD/work/eml-target" cargo clippy --manifest-path experiments/architecture-reassessment/eml/Cargo.toml --locked --offline -- -D warnings
cargo fmt --manifest-path experiments/architecture-reassessment/eml/Cargo.toml --check
```

First run needs public registry access unless isolated Cargo home is populated.
Lead checks used `/opt/homebrew/opt/rustup/bin` on PATH, a task-owned Cargo home and
target outside the product workspace, no models or credentials. Final relocated
run passed, as did format and strict Clippy. Initial attempts failed for missing
rustc PATH, missing MimeHeaders import, wrong launch cwd and an incorrect HTML-only
text-body-count expectation; corrected rather than reported as passes.

`fixture.eml` is entirely synthetic, generated with Python3.14.6 standard-library
EmailMessage/SMTP policy. Names use example.test. Contains encoded UTF-8 subject,
ISO-8859-1 quoted-printable text, HTML alternative, CID image sentinel, Office and
unknown attachments. `plan.docx` is a minimal synthetic ZIP with content types,
root relationships and one document paragraph. Exact CRLF fixture bytes are committed (local Git attributes disable normalization/text diff);
re-run consumes those bytes, with no external producer or private-data dependency.
The CID payload is deliberately not a valid PNG; byte extraction, not image
validation/rendering, is asserted. MIME attachments are not passed to BRN here.

Checks: From/To/Cc available, decoded subject/body, raw Date with+0300 and parsed
zone, Message-ID/In-Reply-To/References, text/HTML bodies, exact original EML,
CID bytes and exact DOCX/unknown attachment bytes. Separate HTML-only/missing-ID
input proves absent headers remain absent and body_text() can derive text from an
HTML part. Inspect PartType/raw headers to distinguish synthesized alternatives
from actual source text; count alone is not proof. No header authentication or
subject-based threading claim.

One measured standalone debug process (Python monotonic/subprocess plus Darwin
RUSAGE_CHILDREN): exit0,0.002832s including startup, peak2,506,752bytes, binary
2,009,232bytes. Tiny fixture, one sample; no production throughput/cost estimate.
`/usr/bin/time -l` could not read kern.clockrate under sandbox; its denied metric
is not used. Final output:1 text body,1 HTML body,3 attachments,8 MIME parts.

Not established: nested/malformed/deep MIME, every encoding, safe HTML display,
attachment conversion/preview, bounded cancellation/sandbox, BRN proposal/apply,
real provider usefulness, native usability or cleanup completeness.

Fixture SHA-256: `3c85d4f40a1fb00710e6af35204fe0310c497303ceee7440873f10ccf742787c`.
DOCX SHA-256: `c319ff66964a3a7ec73cf14e3b212c2f09d947f47bb5615e6d2dc62cfea19baf`.
