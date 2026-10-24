# Follow-ups, inbox and Restore qualification

Status: integration in progress. No new PR merge, release, live-model usefulness or
actual Mac acceptance is claimed. The selected [record](record.json) owns current
activity; this receipt records source-bound observations and limitations.

## Baseline and source

Baseline is merged `f60c5a2f2102f4925b1b182f9f1bab2739fde76d` (PR226).
All seven Mac/Linux/documentation jobs passed on its
[resulting-main run](https://github.com/ewq100/brn-rust/actions/runs/40058075212).
Three optional Windows jobs failed; Windows implementation remains outside scope.
The existing preview gate skipped build/package/publication, so no release occurred.

The new integration branch contains coherent core/service slices:

| Outcome | Authored slice | Integration commit | Observation |
| --- | --- | --- | --- |
| Optional Action owner, waiting-on and distinct follow-up date; schema11 | `7aa793de` | `ff207ecc` | Focused core/service checks reported below; absent old fields retain serialization. |
| Action review facts, service recovery and existing semantic fixtures | `472cf665` | `13bff3e0` | Independently source-reviewed. Four authored semantic cases remain unrun live. |
| Bounded local inbox ledger and schema12 adoption | `0fd44ec4` | `4e58e77a` | Two focused migration/authority tests passed; source-reviewed. |
| Existing staged Restore exposed for explicit confirmation | `aa2c5ff2` | `6c6b1ff4` | Fourteen focused Restore tests passed; source-reviewed. |
| Staged readability validation and reset copied inbox authorization | `e5b7edf5` | `36ca3779` | Sixteen focused Restore tests passed; source-reviewed. |

Restore desktop `569c708e` is integrated as `11386576`; its seven focused
native-feature tests passed. Actions desktop `9c40c9b3` is integrated as
`512e9eac`; both focused native-feature tests passed. The final inbox application
boundary and combined desktop implementation still await integrated qualification. These intermediate receipts are not final-head CI.

## Observed checks

- Actions lane reports 76 focused core and 13 app tests passed, including migration,
  exact raw revision/request preservation, unknown facts, review/Undo/replay,
  current-version and private-project boundaries. The later focused three-test
  follow-up rerun passed with strengthened raw-byte and label-limit assertions.
- Restore lane generated a synthetic schema10 Action backup using the actual baseline
  `f60c5a2f` production core, then prepared, published and reopened it with the new
  schema12 implementation. Exact records and original backup bytes were preserved.
  This is not a relabeled current-schema fixture.
- Integrated Python tooling: **205 run, 204 passed, one Apple Silicon packaging
  skip**, using the saved Cargo cache and existing process-local subreaper.
  Markdown navigation and generated overview checks passed at their recorded slices.
- Combined native desktop qualification is pending. Builds use pinned Rust1.98.1 and
  the saved rootless Linux dependencies, with one shared-target Cargo writer.

## Corrections and review evidence

The initial unconfigured Python run failed five generator fixtures because it used
the default Cargo cache instead of the saved environment. Its child-process timeout
witness also exposed the known container PID1 reaping limitation. Running with the
existing saved environment and process-local subreaper passed the complete suite;
no test assertion or security guard changed.

The Actions form fixture initially used the Mac select-all shortcut on Linux.
The unchanged product then correctly rejected the still-invalid date. Reusing the
existing platform-aware shortcut fixed the fixture; assertions and product behavior
were preserved, and both focused UI tests passed. The original failure is retained.

The first desktop build lacked the saved native pkg-config environment and could
not locate fontconfig. The installed dependency was verified as version2.15.0 and
the normal build retried with its existing environment; no new package/security
workaround was introduced.

Independent review identified inbox path-based enumeration and receipt identity
coupling to unrelated settings changes. The implementation is being corrected to
use descriptor-bound enumeration and stable canonical receipt keys, including
file disappearance/reappearance and owner Undo. Final tests and review must bind
to the resulting commit before this is marked qualified.

Restore's previous validation accepted a readable SQLite database with missing or
malformed current records. A failing witness preceded the narrow fix: stream the
existing typed record decoder over the staged copy before confirmation. The source
backup remains untouched. This validates current-record readability, not every
possible future semantic use of arbitrary historical payloads.

## Remaining acceptance boundaries

The [actual Mac checklist](native-acceptance.md) is unperformed. Linux toolkit tests
cannot establish Mac picker, focus, accessibility, rendering or owner acceptance.
New exact-head full CI and applicable integrated local Rust/native gates remain
pending. Known sandbox Git-ancestor credential-fixture refusal must fail promptly;
ordinary CI supplies the supported full-suite environment, without copying
credentials or weakening checks.

No new BRN live-model calls have been made. Authored semantic cases and scripted
transport fixtures are not evidence of live extraction quality. Provider admission
is still blocked by the previously observed read-only state; no unchanged retry or
new paid service is authorized. Calculator/chart/report work remains deferred.
