# Post-merge cloud verification

Owner request: finish useful remaining cloud verification and provide a headless
Mac prompt for the live-model part. This does not authorize Mac execution, new
features, calculator activation, release, deployment or another merge.

Baseline: PR227 merged at `90e5bcebac1d64fc98ef315675fd2810e1e26f1e`, with the
same tree as reviewed `c6930139302681bc732bad342f187cca5815ba63`.
[Resulting-main CI](https://github.com/ewq100/brn-rust/actions/runs/40182655027)
is terminal: all seven Mac/Linux/documentation jobs passed; three optional Windows jobs failed. Preview gate40184109752 skipped all build/package/publication steps.

The [existing qualification](qualification.md) already covers Action fields,
review/Undo/reopen/stale versions; inbox consent/private context, canonical receipt
replay, source/settings races, cancellation and restart; Restore corrupt data,
fresh destinations, cancellation, killed-process staging recovery and reopen.
Do not repeat entire suites merely to add counts.

Independent inventory selected two uncovered boundaries:

1. In the existing native test harness, click Scan and observe the real inbox
   worker, actual Stock helper, durable Imported state, reopened result and inert
   private queue. Existing native tests used direct service calls/injected finishes.
2. In the existing inbox integration suite, kill an owned child after its first
   import commits and its second conversion starts. Reopen and prove lock release,
   receipt reuse without reconverting the first file, and safe remaining work.

## Results

Initial test source: `ece02f7f348fd4e27baada3f44f029d9e99fac44`; its production source
was unchanged from merged main `90e5bcebac1d64fc98ef315675fd2810e1e26f1e`.
The later CI-discovered lock correction below is the only production change.

- `cargo test -p brn-threads-app --test manual_inbox killed_scan_releases_lock_and_recovers_committed_prefix_without_reconversion --locked --offline`: **1 passed**. The owned child is killed and reaped; its partial test banner is not an additional pass.
- `cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop inbox_tests::scan_button_runs_stock_worker --locked --offline`: **1 passed** (0.59 seconds after compilation). This is the GPUI test harness with real Stock conversion, not actual native picker acceptance.
- `cargo fmt --all --check`, Markdown links, generated overview and `git diff --check`: passed. Strict Clippy (`-D warnings`) passed for the app `manual_inbox` test target and desktop native binary/tests.

The helper artifacts use the current unchanged conversion source: Stock SHA256
`ee475910694951858eb8ae84daa4232a1724e03affb7968322a512684e963b1c`;
adapted `cd286c69654e486c0b86086fd666bd910e934167d899811c322809f3598f6d2b`.
[Targeted test output](evidence/cloud-inbox-tests.txt) preserves actual results.
Independent source review found no blockers; both additions extend existing suites
and add no production capability.

## CI-discovered lock lifetime correction

Initial draft PR228 head `0006b8dea990d38378feba1af0a87d79f0517cf7` failed
[Linux core CI40185145844](https://github.com/ewq100/brn-rust/actions/runs/40185145844).
The existing folder-reselection test received “An inbox scan is already active”
immediately after its scan returned. The new child-process test was spawning
concurrently. All other Mac/Linux/docs jobs passed; optional Windows failed on
Unix-only APIs, outside this batch.

The original lease relied on closing its `File`. A duplicate descriptor retains
the same open-file description and therefore its flock; `O_CLOEXEC` does not
close an inherited descriptor until exec. A concurrent subprocess is a plausible
trigger for the observed CI timing, not a captured syscall trace.

Regression commit `aa38ca4d` deterministically retained a `try_clone()` descriptor:
competing configuration was refused while the owner lived, but reacquisition
incorrectly failed after owner drop. It failed with the same busy error. Fix
`48e82be9` wraps the private lease in an explicit-unlock-on-drop guard, as the
existing staging guard does. It keeps the named inode, nonblocking acquisition,
no-follow and link-count guards. No retries, sleeps or test serialization hide
the defect. The regression also checks inode stability and exclusion before drop.
Independent source review cleared both commits. The corrected regression passed
(1 test), the parallel inbox integration suite passed (23 tests), and strict app
library/inbox-test Clippy passed. [Red/green and suite output](evidence/cloud-lease-red-green.txt)
preserves the original failure. The first broad reproduction command was cancelled
during a dependency rebuild before any tests ran; it contributes no result.
Corrected-head remote CI remains pending at publication; earlier-head passes are
not substituted for that gate.

## Native observation limits

The existing [qualification](qualification.md) records successful Linux initial
Owner keyboard input, visible text and saved/reopened Action. Additional attempts
at all three fields did **not** establish input delivery:

- `run.zyhUpN`: selector unavailable before the dialog was exposed; stopped.
- `run.kRUdqa`: blank fields, unchanged corrected-save readback; failed assertion.
  Unchanged records after an attempted invalid date are **not** validation proof.
- `run.lbu5SR`: one structurally different keyboard-only Owner/Tab/Waiting-on/Tab/
  Follow-up attempt; screenshot inspection still showed blank fields. Stopped
  **before Save**, exit 3, without another retry.

The [final pre-Save capture](evidence/cloud-action-fields-unqualified.png) retains
that failed observation. These used disposable synthetic data and existing binaries
whose production source is unchanged through the reviewed PR227 candidate; they
predate the lock-release fix and did not invoke a provider. AT-SPI text/focus reporting and reliable extended input remain
unqualified; no new product defect was established by these probes. Logs and prior
captures remain in the cloud evidence directory. No actual picker/portal retry,
Mac access or unchanged live-admission workaround occurred.

## Mac live-model handoff and remaining gates

The [Mac live-model prompt](mac-live-model-prompt.md) is ready for a separately
executing Mac build agent. It is headless, uses the normal existing login, begins
with admission and one smoke, then caps representative Runs and tool rounds.
Authored fixtures/deterministic outcomes remain distinct from live semantic
correctness. Mac UI and owner acceptance are outside that prompt. No live-model
calls were made in this cloud follow-up; usage and cost are not fabricated.

Actual native pickers, Mac focus/accessibility/rendering and owner acceptance remain
pending. Calculator is deferred. The follow-up tests and evidence require a draft
PR and separate merge decision; PR227's earlier merge does not authorize this one.
