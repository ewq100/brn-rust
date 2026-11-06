# Development feedback Cycle 2 — cloud

## Assignment and reconciliation

Agent C owns [#255](https://github.com/ewq100/brn-rust/issues/255), then
[the separate provider refactor #263](https://github.com/ewq100/brn-rust/issues/263).
Cycle 1 ended at merged PR262 (`6897fe56`) with no retained setting; its
[observations and qualifications](development-speed-255.md) remain unchanged.
This is a new experiment, not a continuation of its timing allocation.

Frozen baseline: `6897fe56ce9457fa7d60d0354b0ba4627d5049bb`, isolated checkout
`/workspace/brn-cycle2`, branch `agent-c/development-speed-255-cycle2`.
The initial cloud checkout was stale (`aebfa12b`) and is untouched. Current
AGENTS, #253/#255/#263 and selected record were reconciled before edits.
PR258 belongs to B; preserve its updater/static-helper fix and Mac CI step.
No shared provider/runtime/CI edits are assigned to this cycle. The old A
Rig/Copilot completion is not the new response-limit/file-ownership handoff.
Coordination is [on #253](https://github.com/ewq100/brn-rust/issues/253#issuecomment-6693449661).

## Environment and bounded protocol (before candidate results)

Cloud Linux x86_64, Rust/Cargo 1.98.1, locked root and helper graphs. This
container has a 4-CPU cgroup quota (5 visible CPUs), 16 GiB memory limit,
no swap, and about 30 GiB writable disk available at selection. Use 4 Cargo
jobs, `CARGO_INCREMENTAL=0`, default features, the unchanged test profile,
fresh synthetic TMPDIR `/var/tmp/brn-cycle2-fixtures`, and no model assets.
No Mac, native interaction or live product-provider calls. Hosted native/UI/
retrieval compatibility gates remain separate and unchanged.

Existing guidance permits focused checks, but neither `verify-storage.sh` nor
the default `verify-end-to-end.sh` runs a focused test before workspace build
and Clippy. CI similarly builds/lints before workspace tests. Candidate 1 is
an explicit developer command sequence, not a new script or CI split:

```sh
focused_status=0
cargo test -p PACKAGE --lib FILTER --locked --offline || focused_status=$?
# For the headless desktop probe: --bin brn-desktop instead of --lib.
gate_status=0
bash scripts/verify-storage.sh &&
  bash scripts/verify-end-to-end.sh --fixtures-only || gate_status=$?
if [ "$focused_status" -ne 0 ]; then exit "$focused_status"; fi
exit "$gate_status"
```

Baseline runs the same two scripts without the leading focused command.
Both arms keep all their steps, default features and tests; no cache reset
occurs inside a workflow. The complete measured local gate includes format,
locked helper preparation/staging, workspace build, strict all-target Clippy,
workspace tests, scripted-document tests/lint, startup and end-to-end fixtures.
Normal hosted CI supplies its additional production/platform coverage; local
timings are not hosted CI performance claims.

One no-op control and three reversible equivalent source edits are prescribed:

| Probe | Edit | First relevant result |
| --- | --- | --- |
| Core | First `stored == canonical` in `locator.rs` reverses operands | `locator::tests::` in core library |
| Offline AI | First provider inequality in `codex_auth.rs` reverses operands | `codex_auth::portable_tests::` in AI library |
| Headless UI state | Paused-maintenance literal in `threads_state.rs` becomes equivalent `concat!` | `threads_state::tests::` in desktop binary |

Both arms use separate target roots with the same warmed original source and
same helper bytes. Establish each root with a complete baseline gate. Before
each edit, restore only differing bytes, compile the restored original build,
lint and test targets and confirm no-op freshness. Restoration compilation is
setup, not another full verification comparison: original-source behavior was
already qualified; each measured edited workflow runs the full gate. Run
A–B–B–A for each probe, keeping artifact
hashes, compiler/test summaries and complete output. Measure from the edit to
the first **completed relevant test**, and through completion of both scripts
directly in each run. Record process CPU/RSS/I/O and target sizes as well as
wall time. Work duplicated by the focused command is included. Retain failures,
interruptions and setup separately. Check nonzero matching tests and actual
recompilation. A test-only deliberate assertion failure will check focused
feedback and the full gate's nonzero status in an isolated probe; restore it.

Finite budget: at most 60 minutes setup/baseline qualification, then at most
90 minutes timed comparisons, at most 16 complete timed workflows (two no-op,
twelve edit comparisons, two fresh confirmations), plus two harmless-failure
observations. Failed/interrupted attempts count. No second candidate without
a concrete remaining bottleneck and a recorded budget decision. Stop on failed
baseline qualification after two diagnosed attempts, insufficient disk/time,
uncontrolled competing work, or unverifiable identity/coverage. There is no
compiler-flag sweep. Freeze the practical benefit/overhead thresholds after
baseline validation and before any candidate result. Review budget: one
diagnosis and one final independent review, at most 30 minutes each; further
review only for a concrete correctness/evidence finding within that allowance.

## Council capability

Cloud/executor skill catalogs and accessible installed skill paths have no
Council skill; exposed tools have no Council/T3/Kiro/Copilot delegate route.
No configured cloud fallback was discovered. Required Council diagnosis and
review are **incomplete**, not zero-finding reviews. No Mac configuration or
credentials were copied. Continue unaffected offline work as the issue directs.
Codex PR code/security review and CI remain separate requirements.

## Current result

The initial observation launcher could not find `/usr/bin/time`; no gate ran.
Use a disposable Python observer with monotonic line timestamps and OS child
resource accounting instead. That failed launch remains a setup attempt.

First cold baseline: **failed**, exit 101 after 832.676 s, first core result at
822.033 s. Stock/adapted helper builds took 3m43s/3m45s; root build 59.59 s,
Clippy 34.66 s. `process::tests::cancellation_kills_owned_descendant_and_joins_pipes`
failed because the killed grandchild remained a zombie adopted by container
PID 1 (`process::tests:` PID39585, state Z). The environment already supplies
`/workspace/brn-environment/run-with-reaping.py` for exactly this condition.
Its `PR_SET_CHILD_SUBREAPER` loop reaps orphaned descendants and preserves the
command exit status, without retries, signals or test changes. The second
qualification attempt and both comparison arms will use that existing launcher.
The first failure stays visible; it is not an intake cancellation repair or a
passing full gate. The failed gate did not reach scripted-document/startup/
end-to-end checks. The existing B static-helper race is a different failure.

The second qualification passed: 142.118 s end to end, first core test result
at 112.871 s, child CPU 152.370 s, maximum child RSS 2,026,932 KiB. This includes
the helper rebuild, workspace tests, scripted-document tests/lint, startup and
both fixture scripts (CLI journey: 108 steps, 1266 checks). That qualification's
fixtures-only subprocess inherited the environment's older helper overrides;
storage used newly built helpers and staged siblings. Before comparisons the
observer pins **both** overrides to the selected root's newly built artifacts;
both controls must pass with those same bytes. No mixed-helper comparison is
accepted.

Thresholds frozen after this passing baseline and before candidate results:
each edit's median first relevant result must improve by at least 20% and 5 s,
with consistent direction in both balanced repetitions; full-gate median wall
overhead at most 10%, child CPU overhead at most 15%, maximum child RSS/disk
overhead at most 20% (allow 75 MiB for one extra test artifact). All test identities,
features, checks and failure behavior must be preserved. Confirm against the
original baseline. Gains cannot be inferred from sums of unrelated medians.

Root B starts as an independent `cp -a --reflink=auto` copy of A's original-source
artifacts, followed by the original full gate and freshness checks; no hardlinks
are used. This matches cached dependencies without another cold dependency build.
Any relocation/rebuild cost stays in setup evidence. Cold and warm results are
not compared as candidate gains. The 90-minute trial clock starts at the first
no-op control, not after preliminary candidate work.

No candidate measurements or speedup are claimed yet. Raw output remains at
`/workspace/brn-cycle2-evidence`. Stage 2 implementation remains gated by this
cycle's closeout and A's new merged runtime plus explicit shared-file release;
B's first-alpha candidate and publication window stay separate.

## Controls and candidate decision

Trial clock began **19:45:47 UTC**, ending no later than **21:15:47 UTC**.
The original no-op control passed in 75.641 s, first core result 64.901 s,
84.849 s CPU. Package-only candidate 1 passed in 105.302 s, first core result
26.358 s, 146.942 s CPU. Its additional isolated-package dependency graph
compiled for 26 s; wall overhead 39.2% and CPU overhead 73.2% exceed the frozen
limits. Reject this as the unconditional sequence. Do not call the first-result
gain a faster full gate or silently prewarm away its startup cost.

One further candidate is justified by that observed duplication and the remaining
budget (over 80 minutes at this decision). Candidate 2 keeps workspace feature
unification but selects the relevant target kind first: `cargo test --workspace
--lib FILTER` for core/AI and `cargo test --workspace --bins FILTER` for headless
UI state, followed by the same complete gate/status handling. No Cargo/CI/product
source is changed. Original-source `--no-run --message-format=json` checks found
**every artifact fresh**, and the core, AI and desktop test executable paths
exactly match the full workspace gate. AI retains `scripted-runs`; desktop retains
`default`. This supplies the required matching target/feature identities that
the narrower package selection did not establish. Raw artifact messages are
retained. Preserve candidate 1's failed overhead result and its root separately.
Candidate 2 uses a new independent copy of the original-source artifact root;
remaining repetition/time limits and thresholds are unchanged. No third candidate
is planned. The one no-op baseline, twelve ABBA edit workflows and two fresh
confirmation slots still fit the original 16 timed-workflow cap, including the
rejected candidate's one no-op observation.

### Measurement correction before accepting an edit comparison

The initial core baseline completed in 362.271 s (first core 332.779 s). The
initial target-kind candidate completed in 407.267 s (first core 140.694 s).
These are **excluded from acceptance**, not a matched speedup: the relocated
candidate helper rebuilt, and the disposable controller sourced environment
activation a second time after its caller had already sourced it. Activation
appends native paths; two activations double C/C++ include entries from 2 to 4,
library and pkg-config entries from 3 to 6, and change PATH. The earlier root
preflight had run under a single activation. This invalidates a fully warm-cache
claim; Cargo freshness must be checked in the actual executing environment.
Both failed comparison attempts remain counted and preserved. The scheduling
controller was paused while the active observer finished normally, then stopped;
all Rust bytes were restored. No in-flight source was changed.

Correct the disposable invocation, not product code or global configuration:
activate once in a fresh shell, pin a digest of native setup variables, warm both
roots' original production/lint/test/helper graphs there, and inspect full versus
target-kind Cargo feature messages. Only then resume ABBA. Reallocate the two
reserved additional confirmation slots to replace the excluded core attempts;
the last A in each ABBA group provides fresh original-workflow confirmation.
The 16-workflow cap and 21:15:47 UTC deadline remain unchanged. If all three edits
cannot be qualified in that budget, retain no workflow change and report the
incomplete evidence. Post-run hashes added for later observations are collected
after the measured workflow and cannot qualify missing earlier hash receipts.
