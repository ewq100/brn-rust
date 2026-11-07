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

Use `--split held-out` only after the candidate sequence ends, once with both
baseline and final arms. The 24 fixtures split 16/8, two tuning and one held-out
per format. All manifests were written from sources before measurement. Hashes
in `frozen.json` bind the manifests, split, fixtures and scorer source. Scoring
refuses a changed frozen input. Fixture regeneration is optional and uses only
`fixtures/generate.py`, never converter output. New synthetic material is CC0.
Existing fixture provenance stays in its original README and source inventories.

Fidelity is the fraction of hand-written expected elements retained, averaged
equally per file within each mode. Table matrices include header rows and empty
cells. Heading levels, list labels and code text are exact GFM AST comparisons.
Key spans must occur exactly within visible paragraph, heading or cell text.
Each expected occurrence consumes one match. Figures must be rendered image nodes
linked to retained assets with exact source hashes or decoded RGB hashes. Merely
returning an unused asset, prose spelling Markdown, or code spelling a heading
does not pass. Missing elements and unexpected structural additions are listed.
Baseline-to-candidate comparison separately rejects lost visible blocks or typed
structure outside the manifest. This is bounded fixture evidence, not universal
document fidelity or a completeness label in the app.

Cold means a new host process and helper with fresh scratch, including process
startup and input read. Warm means a persistent preheated host; every conversion
still creates a fresh restricted helper. Neither means evicted OS caches. No
root-level cache purge is used. Five measured runs follow an unmeasured warmup.
Arms interleave ABAB per fixture/temperature/mode with identical source paths,
bytes and mtimes. Report median, min, max, full samples and max-minus-min spread.
Run without competing builds. A speed-only candidate requires median paired gain
greater than the larger arm's measured spread, with unchanged fidelity and all
loss, refusal, test, network and size gates passing. Keep cold and warm conclusions
separate. App size uses `scripts/report-sizes.sh` on task-owned executable bundles.

The driver calls the shared checked conversion service with FullNote intent and
AI Off. It includes input read, helper startup, validation and scratch cleanup.
It excludes SQLite publication and desktop interaction. The existing CLI and
intake suites cover those integration routes separately. Results do not establish
native observation, owner acceptance, release readiness or encrypted PDF behavior.
