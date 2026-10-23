# Precise editing and remaining Linux UI qualification

Active local work on `codex/precise-edit-ui-qualification-20261023`, based on
integrated main `9df81ae66f7c5955427f4d3ecb81f65d41d439f7`. Owner authorized this
scope after completing the upgrade integrations. New publication and merge are
not assumed. No release, credential transfer, security changes or Mac access.

## Completed prerequisite

PR221, PR222 and PR223 are merged, preserving their independent commits. Final
main's four required checks, both Linux native jobs and documentation passed.
The three optional Windows lanes failed, as on the earlier baseline; exact log
causes could not be compared because the baseline log read was denied. The whole
integrated tree equals the reviewed Rig candidate. A freshly invoked native and
CLI build on main passed. Actual native old-store reopening, Unicode Save,
AT-SPI menus, repeated keyboard/pointer menu paths, Quit/reopen, exact persisted
state and clipboard comparison, unchanged sentinel and OS close passed.
[Source-bound receipt](evidence/upgrade-main.json). Screenshots and full raw
records remain in `/workspace/brn-environment/logs/upgrade-publication/native-main`.
PR222 temporarily reported open although its signed merge was already on main;
GitHub reconciled it after PR223. No duplicate merge was attempted.

## Prior targeted-edit failures and next bounded test

The newer campaign branch `f0c1c0ba` contains evidence absent from main's older
campaign projection. Read those published records without contacting the Mac.
The original three-attempt session failed to prepare a candidate: the requested
UTF-8 range was 43..55 but observed calls used other integer ranges. The initial
trace did not record those arguments. Later receipts expose 48..60, 49..61 and
47..59 and the exact host mismatch refusal. A subsequent one-attempt session
after quote selection was implemented still used 44..56 and 39..51; it did not
exercise null/null selection. Both sessions are explicitly closed. Their raw
expected/replacement strings were not traced; the evidence does not establish
the model's arithmetic mechanism or a quote-selector defect.

Current base guidance already prefers null/null for one unique known quote when
its byte range has not been established. Do not add another prompt change or
automatically relocate an invalid integer range. The selected distinct test is
an offline runtime replay of the observed locations using the known requested
phrase, followed by deliberate exact-quote preparation, idempotent repetition,
and native review/Apply/Undo/reopen. This does not reproduce unknown historical
wire arguments and cannot qualify live model strategy selection.

Retained receipts: [initial](evidence/targeted-edit-account-qualification.json),
[diagnostic](evidence/targeted-edit-diagnostic-qualification.json),
[coordinate guidance](evidence/targeted-edit-guidance-account-qualification.json),
[post-quote attempt](evidence/post-quote-repair-account-qualification.json).
The owner authorized a new bounded live qualification at 08:37 UTC on 23 October.
One new attempt is planned, with Luna/low, four tool rounds and a 180-second
deadline; old sessions remain closed. The existing cloud ChatGPT login is present,
but current allowance/model admission is blocked before dispatch as recorded below.

## Remaining UI boundaries

- Linux disabled state: assess/backport the exact official AccessKit correction
  while retaining GPUI and the compatible AccessKit family. Actual AT-SPI state,
  actions and persisted-state observations are required, not screenshots alone.
- Native picker: previous `org.freedesktop.portal.FileChooser.OpenFile` caller
  inspection failed opening `/proc/19341/root`, before dialog dispatch. No new
  relevant security authority; action remains denied and is not retried.
- Mac native close/menu/accessibility: no authorized accessible Mac here. Pending;
  Linux checks do not substitute for it. Full-backup Restore has no desktop
  control; prior CLI restore checks are not GUI acceptance.

Implementation, offline fixture and native observations are complete as described
below. Selected regression gates passed; independent source reviews found no blockers.
Live-model, file-picker and Mac acceptance remain unpassed.

## Native observations on the local candidate

The native window rendered the precise candidate diff; Result preview was also
opened. Apply saved exactly the requested replacement at version 2. Undo restored
all original Note data at version 3. Repeating the original Undo showed the stale
content notice and left records unchanged. Quit/reopen retained the original
note, exact Unicode clipboard text and unchanged sentinel. A second fresh fixture
closed review without Apply, quit and reopened: the same immutable candidate and
all persisted snapshots remained unchanged. These are offline runtime-to-native
checks, not proof that a live model will choose quote selection.

The patched translator passed two actual AT-SPI enabled → disabled → enabled
cycles. While an editor draft was dirty, Rewrite reported neither Enabled nor
Sensitive and offered no action. Restoring the original saved text restored the
states and action. No Rewrite action was invoked. Menus, Projects and normal OS
close also passed. This observes queryable states/actions, not delivery of every
assistive-technology event or full screen-reader usability.

Evidence: [native receipt](evidence/native-result.json),
[review screenshot](evidence/review.png), [reopened note](evidence/reopened.png),
[disabled control](evidence/a11y-disabled-1.png),
[actual AT-SPI transitions](evidence/a11y-transitions.json),
[fixture manifest](evidence/manifest.json), [refusals](evidence/refused-locations.json)
and [quote calls](evidence/quote-trace.json). Full snapshots, screenshots and action
logs remain in the raw directory named in the receipt.

## Reproduction and checks

Use the documented Linux cloud display and task-local AT-SPI setup. Create a fresh
synthetic fixture; never substitute an owner vault:

```sh
TMPDIR=/tmp cargo test -p brn-threads-app --example native_precise_edit_fixture \
  --locked --offline --jobs 1
TMPDIR=/tmp cargo run -p brn-threads-app --example native_precise_edit_fixture \
  --locked --offline --jobs 1 -- --output /tmp/brn-precise-new
cargo build -p brn-desktop --features native-ui --locked --offline --jobs 1
brn-desktop --data-dir /tmp/brn-precise-new/workspace
```

Open Home's review row, inspect Changes and Result, then Apply. Changes contains
`Undo: Replace only the requested synthetic passage`. Check Note bytes, versions,
receipts and sentinel through the JSON CLI after each action. Repeat Undo, then
quit/reopen. For interruption use a second new fixture, close review and quit
before Apply; the candidate and records must remain unchanged. For accessibility,
query Rewrite before a draft, while dirty, and after restoring/saving; repeat.
Do not invoke Rewrite or Send. The fixture uses mock transport and an unsupported
final provider selection, with no credential sources.

Adapter regression: both new tests failed on the old conditional; all nine tests
passed after the exact upstream backport. [Red witness](evidence/accesskit-baseline-red-corrected.log),
[green result](evidence/accesskit-patched-green.log), and
[example test](evidence/precise-edit-writable-tmp.log). See the
[vendor provenance and reproduction](../../../../vendor/accesskit_atspi_common/BRN-PATCH.md).
Independent reviews found no blocking source, dependency or fixture issues.
Formatting and Markdown link checks passed. Desktop and writing regression checks passed (352 + 44 + 12 tests; one
intentional desktop ignore), as did strict native Clippy. Strict fixture Clippy also passed.
Live qualification remains blocked independently of these passing local gates.

Failures remain recorded: initial example compilation used the crate's Result
alias incorrectly; initial TMPDIR was read-only; vendor test package routing and
one trait import were corrected before obtaining the real red witness. A combined
clipboard select/copy check raced clipboard availability; separate key events and
explicit UTF8_STRING succeeded. No application assertion was weakened.

The focused edit filter initially selected zero tests and is not counted. With the
correct module filter, 16 passed and the mocked HTTP fixture failed the credential
guard. Narrow filesystem permission did not resolve it: this sandbox injects
protected `.git` ancestors at writable roots, as previously documented. The
approved supported test escalation, using the established disposable directory,
passed all 17 tests; no live transport or security setting changed:

```sh
TMPDIR=/var/tmp/brn-runtime-schema-20261023 cargo test -p brn-ai --lib \
  threads::note_edit_tests --locked --offline --jobs 1
```

[Zero-test attempt](evidence/precise-ui-note-edit-tests.log),
[sandbox refusal](evidence/precise-ui-note-edit-corrected.log),
[narrow-permission refusal](evidence/precise-ui-note-edit-protected.log),
[17-test passing result](evidence/precise-ui-note-edit-supported.log).

## Newly authorized live qualification: admission blocked

[Admission receipt](evidence/live-admission.json) records the new authorization and
one-attempt bounds. `codex login status` confirms an existing cloud ChatGPT login;
only auth schema/presence was inspected, with no secret values recorded. No auth
was copied or transferred. Historical 3/3 and 1/1 sessions remain closed.

The read-only `account/rateLimits/read` and `model/list` queries could not initialize
the local app server. Its SQLite state first targeted the managed read-only home.
The documented per-process `sqlite_home` option redirected that state into the
task workspace, but startup still requires creating/opening
`/run/codex-environment/codex-home/installation_id` for writing. A file-operation
trace confirmed EROFS on that exact path. No query or model turn executed.

Do not copy auth, alter managed permissions, substitute stale Mac allowance, or
ignore the existing 5% allowance floor. Current quota and model availability are
unverified. Required next step is a supported admission interface or approved
platform handling of the managed local-state restriction. This is a concrete
runtime access blocker, not a claim that the newly granted model permission is
missing. Attempts used: 0; model cost: not reported because no dispatch occurred.
[Official state-directory setting](https://developers.openai.com/codex/config-reference/).

## Local candidate and remaining gates

Product correction commit: `471fe0b02cc493d69c999aaca998f835abd5a0c1`. Native evidence
records exact source hashes and binary SHA. The fixture/evidence is retained as a
separate local commit on the same branch. No new branch publication or merge has
been authorized or performed. Upstream main remains the completed upgrade baseline.

Final selected commands, all passed:

```sh
TMPDIR=/tmp cargo test -p brn-desktop -p brn-writing-ui \
  --features brn-desktop/native-test-support,brn-writing-ui/native-test-support \
  --locked --offline --jobs 1
TMPDIR=/tmp cargo clippy -p brn-desktop -p brn-writing-ui \
  --features brn-desktop/native-test-support,brn-writing-ui/native-test-support \
  --all-targets --locked --offline --jobs 1 -- -D warnings
TMPDIR=/tmp cargo clippy -p brn-threads-app --example native_precise_edit_fixture \
  --locked --offline --jobs 1 -- -D warnings
cargo fmt --all -- --check
git diff --check
python3 scripts/check-markdown-links.py
```

[Native tests](evidence/precise-ui-native-support-tests.log),
[native Clippy](evidence/precise-ui-native-clippy.log),
[fixture Clippy](evidence/precise-ui-fixture-clippy.log). The ignored desktop case
requires a separate shared CLI useful-intake fixture; it did not self-pass. Full
workspace tests were not repeated for this Linux translator-only product change;
new-branch CI and Mac execution have not run. Independent evidence review confirmed
the persisted snapshots and actual AX observations, with no material inaccuracies.
