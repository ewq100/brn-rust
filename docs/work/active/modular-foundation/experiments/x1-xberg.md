# X1 — Xberg PDF qualification result

Recorded 15 October 2026 from the pinned experiment sources below.
The owner authorized isolated qualification and written design before the broader program started.

**Status:** gates 1–2 passed on two synthetic PDFs (five pages); gate 3 written design delivered.
**Verdict: bounded adaptation, NOT adopted. Production is unchanged.**

## Pin and dependency gate

`xberg =1.3.10`, default features off, `pdf` feature only (required transitive feature closure included).
No pdfium, OCR or ORT in the experiment graph. The locked build ran offline with network denied;
the audited build/runtime route performs no automatic native-library or model downloads.
Normal pinned Cargo acquisition preceded the offline build.

## Evidence

- Native page rasters supplied all five page previews without pdfium.
- Both embedded images were pixel-identical to Poppler, with matching page associations.
- Text, previews and images were checked against independent source inventories.
- The process-fixture table's raw text order differs. Correct row associations require the existing
  complete source-line permutation receipt; no values are rewritten. Raw order equivalence is not claimed.

## Costs and qualification limits

- Approximately **35 MiB stripped harness size delta**; the real helper's incremental size is unmeasured.
- Build timings are provisional because another build overlapped.
- `option-ext 0.2.0` declares MPL-2.0; bundled font/fontdb notices need distribution review.
- No hard memory (RSS) limit is established. Allocation/expansion enforcement remains a pending gate.
- The exact production helper restriction profile is untested and stricter than the experiment profile.
- Two synthetic PDFs are a small corpus; no live Luna comparison or production integration was exercised.

## Integration preconditions

Production integration requires `work/daily-workflow-reliability` landed, C0 complete, C's exact file
allocation, explicit owner approval, and the pending gates in DESIGN.md below. Those include exact-helper
containment, resource/fidelity checks, combined dependency/package qualification and unchanged Luna/checked-save
behavior. The proposed replacement stays inside the existing restricted helper with no hidden Poppler fallback.
This result does not authorize cutover or change the broader proposal's status.

## Environment note

A preflight `rustup` command unexpectedly auto-installed stable **1.99.0** on the qualification machine.
There was **no repository impact**: the repository pin was unchanged and qualification used existing Rust 1.98.1.

## Pinned sources

- [REPORT.md](https://github.com/ewq100/brn-rust/blob/65715ee538bfaffdd442307d2ffa631cbd2b68bc/experiments/x1-xberg/REPORT.md)
- [DESIGN.md](https://github.com/ewq100/brn-rust/blob/65715ee538bfaffdd442307d2ffa631cbd2b68bc/experiments/x1-xberg/DESIGN.md)
- [Experiment tree at 65715ee](https://github.com/ewq100/brn-rust/tree/65715ee538bfaffdd442307d2ffa631cbd2b68bc/experiments/x1-xberg)
