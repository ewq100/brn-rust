# Offline intake score

Run from any directory with Python 3 and the pinned Rust toolchain. No new runtime
dependency, AI provider, source URL fetch or credential is used. The helper's OS
sandbox denies network. A live loopback restriction witness must confirm denial
for each exact helper artifact before scoring. Encrypted PDF stays deferred.

Build both helpers with the command in [setup](../../docs/development/setup.md),
then build each standalone manifest again with `--release --locked --offline`.
Use separate stock/adapted `CARGO_TARGET_DIR` values. Build the measurement tools:

```sh
CARGO_TARGET_DIR=/absolute/probe-target cargo build --release --locked --offline \
  -p brn-threads-intake --example intake_probe -p brn-intake --example intake_inventory
BRN_SCORE_INVENTORY=/absolute/probe-target/release/examples/intake_inventory \
  python3 -m unittest discover -s tools/intake-score -p 'test_*.py'
```

Copy baseline executables into a separate immutable task directory before changing
production code. Create an arms JSON with absolute paths, in baseline/final order:

```json
{
  "baseline": {"probe": "/absolute/baseline/intake_probe", "stock": "/absolute/baseline/brn-xberg-helper", "brn": "/absolute/baseline/brn-xberg-helper-brn"},
  "final": {"probe": "/absolute/final/intake_probe", "stock": "/absolute/final/brn-xberg-helper", "brn": "/absolute/final/brn-xberg-helper-brn"}
}
```

One command writes a scorecard with every fixture, format, helper mode and arm:

```sh
python3 tools/intake-score/score.py --arms /absolute/arms.json \
  --inventory /absolute/probe-target/release/examples/intake_inventory \
  --split tuning --runs 5 --output /absolute/tuning.json
```

Use `--split held-out --held-out-ledger /absolute/held-out.started.json` only after
the candidate sequence ends, once with both baseline and final arms. Exclusive
ledger creation records even interrupted attempts. The 24 fixtures split 16/8, two tuning and one held-out
per format. All manifests were written from sources before measurement. Hashes
in `frozen.json` bind the manifests, split, fixtures and scorer source. Scoring
refuses a changed frozen input. A new climb re-freezes by updating `frozen.json`
in a reviewed commit before its first measurement. The hostile list is
frozen too. Fixture regeneration uses only
`fixtures/generate.py`, never converter output. New synthetic material is CC0.
Existing fixture provenance stays in its original README and source inventories.

Fidelity is the fraction of hand-written expected elements retained, averaged
equally per file within each mode. Table matrices include header rows and empty
cells. Heading levels, list labels, numbering, nesting, links, quotations and code
text are exact GFM AST comparisons. Nested tables use one HTML cell tree with
explicit text before/after the inner table, without duplicated visible text.
Key spans must occur exactly within visible paragraph, heading or cell text.
Each expected occurrence consumes one match. Figures must be rendered image nodes
linked to retained assets with exact source hashes or decoded RGB hashes. Merely
returning an unused asset, prose spelling Markdown, or code spelling a heading
does not pass. Missing elements and unexpected structural additions are listed.
Source order groups reject reordered matches. Baseline-to-candidate comparison
also rejects lost or reordered visible blocks, changed parent context, and new
structure that fails to satisfy a missing manifest element. This is bounded
fixture evidence, not universal
document fidelity or a completeness label in the app.

Cold means a new host process and helper with fresh scratch, including process
startup and input read. Warm means a persistent preheated host; every conversion
still creates a fresh restricted helper. Neither means evicted OS caches. No
root-level cache purge is used. Warm has one unmeasured warmup and five measured
runs. Cold has five measured fresh-process runs.
Arms interleave ABAB per fixture/temperature/mode with identical source paths,
bytes and mtimes. Report median, min, max, full samples and max-minus-min spread.
Run without competing builds. The requested fixed ABAB order can have a position
bias, so timing claims remain local and require confirmation. A speed-only
candidate requires every mode/temperature median of per-file paired gains to
exceed its median of per-file larger-arm spreads. No file may slow beyond its
larger-arm spread. Fidelity must stay unchanged and every guardrail must pass.
Keep cold and warm conclusions separate. App size uses `scripts/report-sizes.sh`
on task-owned executable bundles.

The driver calls the shared checked conversion service with FullNote intent and
AI Off. It includes input read, helper startup, validation and scratch cleanup.
It excludes SQLite publication and desktop interaction. The existing CLI and
intake suites cover those integration routes separately. Results do not establish
native observation, owner acceptance, release readiness or encrypted PDF behavior.

The small held-out sample is authored by the same engineer and remains visible,
although it cannot be converted until confirmation. It establishes only this
fixture result. The separate keep gate combines the scorecard with hashed test
logs, protected source/lock diffs and native executable-bundle sizes:

```sh
python3 tools/intake-score/guardrails.py --scorecard /absolute/tuning.json \
  --tests /absolute/test-receipts.json --baseline-app /absolute/baseline-app \
  --final-app /absolute/final-app --output /absolute/guardrails.json
```

Test receipts are a nonempty JSON list with `command`, `exit_code`, `log` and
`log_sha256`. A scorecard alone never authorizes keeping a candidate. Candidate
count and the three-rejection stop rule are recorded in the PR. The first climb's
scorecards and receipts are at tag `archive/work/intake-hill-climb`.
