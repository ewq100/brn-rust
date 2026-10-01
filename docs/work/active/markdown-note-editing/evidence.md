# Markdown note editing evidence

Date: 1 October 2026

Implementation: pending. Product verification: not run. Native/user acceptance: pending. Integration: documentation only; no feature implementation, migration, merge or release.

## Design and planning

- Inspected a clean local `main` at `4f059881f5a34d22ffdf1bb53e79954c94c8608f`.
- User selected explicit Save/Cmd-S, a local vault, no force overwrite, and coordinated journaled exchange with explicit non-cooperating-writer limitations.
- User approved four design sections, then explicitly reviewed/approved the written [specification](../../../superpowers/specs/2026-10-01-open-and-safely-edit-markdown-notes-design.md) committed at `32077d8`.
- [Implementation plan](plan.md) prepared from the existing store/workflow/CLI/worker/native structures. Execution choice remains pending.
- The visual companion was declined. No application process, provider, note vault, credentials or model assets were accessed.

## Documentation checks

The specification commit passed `git diff --cached --check`; a local checker validated 47 file/fragment links across its three changed documents. Commit scope and clean worktree were inspected after `32077d8`. These are documentation observations, not application verification.

Planning documentation passed `git diff --check`; the local checker validated 60 file/fragment links across the six changed documents. A red-flag scan found no unfinished markers or deferred-code instructions in the plan. Inline self-review checked spec coverage, task dependencies, interface consistency, replay ordering, copy reconciliation and atomic provider-outcome/currentness bookkeeping. All nine implementation tasks remain unchecked.

No Rust build, test suite, crash experiment or native editing demonstration is claimed.

## Plan review and revision

Opus 5.5 with high reasoning reviewed the plan at `dca2b98` read-only. Its verdict was ready after fixes, without a redesign. The user requested a plan update, not execution. This documentation revision started on clean local `main` at `de0eb45621f9d724cf0fa60be74f36a6ab5c08ae`; the independent workspace-shell design/plan remains unchanged.

| Finding | Revised contract or check |
| --- | --- |
| 1. Global ownership versus test isolation | Directory-descriptor locks share the production identity namespace while using disposable fixture roots; no private lock-directory override. |
| 2. Restart acquisition and multiple vaults | Registry-based lazy root reacquisition, one vault per workspace, explicit unavailable/busy state, recovery/history accessible without the root. |
| 3. Startup-interrupted saves | Dedicated typed note reconciliation; generic Interrupted transitions remain unchanged. |
| 4. Inaccessible crash hooks | Process-crash children run as lib unit tests; integration tests exercise only public reopen/replay interfaces. |
| 5. Unresolved-original gate | Persisted resolution and partial uniqueness constraint, separate from generic operation status and cleanup. |
| 6. Current document surfaces | Shared source projection, explicit state/reason, guarded CLI/native/headless presentation; no old bytes labeled current. |
| 7. Weak evidence tests | Real keyword index and positive query before invalidation, exact error assertions, explicit approval/no-op/reapproval paths. |
| 8. Buffer/save generations | Equal-generation identical-text saves and higher-generation capture; later edits cannot be lowered by completion. |
| 9. Reserved paths and aliases | Preserve registered-path ownership; case/Unicode alias checks veto ambiguous copies and require volume qualification. |
| 10. Intermediate CLI breakage | Exhaustive mappings and affected constructors/callers updated in the introducing task; workspace/native compile gates. |
| 11. Stale ask replay | Typed stale failure with original provider outcome/history, no resubmission; CLI subprocess assertion. |
| 12. Shutdown and close | Critical note jobs drain/join; GUI close waits asynchronously for durable recovery/save acknowledgement or explicit discard. |
| 13. Presenter event route | Dedicated queue to lightweight worker polling, independent of terminal events, with coalescing/rescan and idle observation. |
| 14. Copy/artifact/receipt precision | Explicit absent precondition, preallocated target ID, source/destination receipts, tagged stored failures and retained non-text artifact metadata. |
| 15. Phases/proof/cleanup | Crash checkpoint assertions, missing-proof uncertainty, separately journaled cleanup and protected uncertain recovery after explicit acknowledgement. |
| 16. Foundation feature name | `NSOperation` supplies `NSOperationQueue`; concrete API compatibility remains a compile-gate requirement. |
| 17. Duplicate qualification | Run the integrated default script once; separate optional/native/resource checks and observed native acceptance. |

Feedback was checked against the existing store startup/transition rules, ask replay, CLI exhaustive error mapping and document display, worker admission/shutdown, native draft-only close guard, and integrated verification script. Expected whole-corpus IndexStale remains valid; not every exclusion should be forced into EvidenceStale. Enrollment does not inherit approval, and unavailable native profiles cannot stand in for evidence-filtering tests.

The directory-descriptor lock choice is a planned implementation of process-global vault ownership, not a verified filesystem capability. macOS `flock(2)`/`fsync(2)` manuals and [Foundation's feature list](https://docs.rs/crate/objc2-foundation/0.3.2/features) informed the correction. Directory-lock support, overlapping-root exclusion, file/directory durability, accessor/protocol compatibility and native exact-byte round trips still require execution-time qualification. No assumption that `F_FULLFSYNC` works on directory descriptors or that process-kill checks prove power-loss durability was adopted.

Only this evidence file and the [plan](plan.md) were revised. The approved specification and implementation scope remain unchanged; all implementation task checkboxes remain unchecked.

Revision checks: `git diff --check` passed. The local checker validated 66 file/fragment links across the plan, evidence, approved specification, documentation index, status and active-work index. Static checks confirmed nine unchecked tasks, the reviewed contracts, no unfinished markers/deferred-code instructions, and one nonduplicated final integrated-suite invocation. Inline self-review checked persisted result/resolution types, staging-path crash recovery, mutable observation callers, copy acknowledgements and specification coverage. These are documentation checks only; no Rust/native/provider/vault checks were run.

## Next action

Select execution when authorized. At that time, establish an isolated worktree, implement task-by-task with red-green verification, and append actual commands/results/limitations here. Preserve the original vault and all unrelated work.

## Integrated qualification — 1 October 2026

This entry supersedes the planning-only state above without rewriting it.
Implementation: **implemented** (Tasks 1–8). Verification: **partial**.
Native/user acceptance: **pending**, not observed (R9). Integration:
`feat/markdown-note-editing`, based on `main@9786d2d`, tested code
`8574ed0a019cebc79ec995f386d7505fde49ec7d`; not merged, pushed or released.
The qualification documentation commit follows that code commit.

Initial Task 9 inspection at `e40884624b44e6acea12611c24b9456aa107d1c0`
and resumed inspection at `8574ed0` both had empty `git status --short`.
Qualification changes are documentation only: architecture overview/invariants,
status, documentation/active-work indexes and this evidence. Existing crate
READMEs were inspected; their Tasks 1–8 contracts and architecture links already
cover these behaviors, so no redundant rewrite was made.

### Environment and provenance

- macOS 26.5, build 25F71, arm64; rustc 1.98.1
  (`48a229cea`, 2026-09-01), cargo 1.98.1 (`797e8a9bc`, 2026-08-05).
- Controller logs record run 1 from 16:34:47 to 16:35:16 EEST and run 2 from
  16:39:57 to 16:41:13 EEST on 2026-10-01; run 2 explicitly records `sw_vers`
  and rustc. Optional commands below were run by the Task 9 implementer at
  `8574ed0` on the same Mac with pinned features/toolchain.
- R21 environment substitutions: controller set worktree-local
  `TMPDIR="$PWD/.superpowers/sdd/plan/tmp"` and prepended
  `PATH="$PWD/.superpowers/sdd/plan/shim:$PATH"`. Ripgrep is a documented setup
  prerequisite but is absent; the git-ignored controller shim implements only
  the fixture's `rg -q PATTERN FILE` through `grep -qE`. The stale-index literal
  assertion therefore ran under grep, not ripgrep. Optional Cargo commands
  used the same TMPDIR, without the PATH shim.
- R22: the controller executed the integrated script, because the implementer's
  execution environment could not invoke its `mktemp`. The implementer read
  both logs and did not execute or rerun that script. The original standalone
  inspection's `/bin/bash: rg: command not found` is not a verification run.
- Machine-local logs/reports are under `.superpowers/sdd/plan/` and git-ignored,
  not portable durable artifacts. Essential outcomes/failure text are retained
  here. No live provider, original vault, model acquisition or main-checkout
  mutation was performed.

### Integrated default verification (controller-run)

Exact command, from the worktree, for both controller runs:

```sh
TMPDIR="$PWD/.superpowers/sdd/plan/tmp" PATH="$PWD/.superpowers/sdd/plan/shim:$PATH" bash scripts/verify-end-to-end.sh
```

| Run | Commit and local log | Outcome |
| --- | --- | --- |
| 1 | `e408846`, `.superpowers/sdd/plan/verify-e2e.log` | Failed, exit 101. Format/build/Clippy completed; tests stopped at `cli_basic`: 26 + 19 + 10 passed, 1 failed, 0 ignored across the three reached result groups. Fixture was not reached. |
| 2 | `8574ed0`, `.superpowers/sdd/plan/verify-e2e-2.log` | Passed, explicit `EXIT=0`: format/build/Clippy, 429 test executions, 0 failed/ignored across 44 result groups (including zero-test/doc-test groups), plus provider-free import/approval/index/search/staleness fixture. |

Run 1's exact failure:

```text
---- documents_list_json_matches_seed_order_and_fields stdout ----

thread 'documents_list_json_matches_seed_order_and_fields' (119084409) panicked at crates/brn/tests/cli_basic.rs:220:5:
48366f3f-64d1-409c-9528-09dc8d2306a6 d6866333-1da4-4ae3-94ef-246357f800ef draft alpha.md Current
8442efcb-7af5-4336-aaa8-2e13d34dbff6 9b70e6d9-45d8-4327-a0a0-c80a37ceff36 approved beta.md Current

note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

failures:
    documents_list_json_matches_seed_order_and_fields

test result: FAILED. 10 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s

error: test failed, to rerun pass `-p brn --test cli_basic`
EXIT=101
```

The controller routed the fix outside this documentation task:
`8574ed0 fix(cli): preserve current document list text`, reviewed clean per
controller handoff. R23 preserves legacy human Current lines byte-for-byte;
only non-current managed rows append state/reason, and JSON retains metadata.
Run 2 ends:

```text
End-to-end workflow verification passed (local fixture; no provider, credentials, or native model used).
EXIT=0
```

This is controller-run automated evidence, not native editor observation.
No overlapping full workspace/storage/desktop scripts were run by Task 9.
The sample desktop headless check does not qualify the integrated note editor.

### Optional native commands (implementer-run)

Exact commands (logs captured separately in `.superpowers/sdd/plan/`):

```sh
TMPDIR="$PWD/.superpowers/sdd/plan/tmp" cargo test -p brn-desktop --features native-ui --locked
TMPDIR="$PWD/.superpowers/sdd/plan/tmp" cargo clippy -p brn-desktop --features native-ui --all-targets --locked -- -D warnings
TMPDIR="$PWD/.superpowers/sdd/plan/tmp" cargo test -p brn-workflow --features native-retrieval --locked
```

| Command | Outcome |
| --- | --- |
| Native desktop tests | Exit 0; 49 unit tests + 5 CLI tests = 54 passed, 0 failed/ignored. Includes exact GPUI Rope text path, real owned-worker save round trip and scheduling/close state tests, not human GUI observations. Log: `task-9-native-test.log`. |
| Native desktop Clippy | Exit 0, warnings denied. Upstream Cargo future-incompatibility notice for `block v0.1.6` remains; this is not a Clippy failure. Log: `task-9-native-clippy.log`. |
| Native workflow retrieval | Build failed, exit 101, before any tests: missing `protoc` in `lance-encoding v12.0.0`. Zero native-retrieval tests executed; unavailable build prerequisite, not passed semantic/hybrid behavior or a tested model-resource gate. Log: `task-9-native-retrieval.log`. |

Native-retrieval failure:

```text
error: failed to run custom build command for `lance-encoding v12.0.0`

Caused by:
  process didn't exit successfully: `/Users/evokessler/repos/brn-rust/.worktrees/markdown-note-editing/target/debug/build/lance-encoding-a4e486e791d13050/build-script-build` (exit status: 1)
  --- stdout
  cargo:rerun-if-changed=protos

  --- stderr
  Error: Custom { kind: NotFound, error: "Could not find `protoc`. If `protoc` is installed, try setting the `PROTOC` environment variable to the path of the `protoc` binary. To install it on macOS, run `brew install protobuf`. It is also available at https://github.com/protocolbuffers/protobuf/releases  For more information: https://docs.rs/prost-build/#sourcing-protoc" }
warning: build failed, waiting for other jobs to finish...
```

`command -v protoc` and inspection of `/opt/homebrew/bin/protoc` and
`/usr/local/bin/protoc` found no existing executable. No system package install
or model download was attempted. Native retrieval remains unqualified; default
keyword/fake-provider checks cannot stand in for semantic/hybrid resources.

### Earlier task gates — historical, not freshly rerun here

Sources: implementer reports `.superpowers/sdd/plan/task-{1..8}-report.md`,
including their fix rounds. The integrated result above independently exercises
the default suite at the current code commit; the following are task-gate
results, not additional Task 9 test counts.

| Task | Implemented contract and earlier task-gate evidence |
| --- | --- |
| 1 | Schema V6 registry, generations, buffers, save/copy reservations, tagged results, rolling recovery and identity-bound cleanup. Recorded store notes/storage/workflow and migration gates, including review fixes protecting reserved copies and Applied recovery. |
| 2 | Bounded descriptor reads, vault ancestry ownership, Foundation coordination/presenter, exchange/exclusive primitives and attributes. Corrected explicit directory `libc::fsync` gate: 16 notes-lib cases; required failure propagation verified without fallback. |
| 3 | Fresh saved observations versus protected editing baselines, offline recovery and durable observation/enrollment tokens. Final alias-fix gate: 32 store + 19 workflow + 16 adapter cases; 4 alias cases additionally rerun. |
| 4 | Journaled original saves, no-op/replay, proof-based restart and separate cleanup. Latest fix gate: 30 notes-lib + 9 public recovery + 19 public notes cases. Process-kill/checkpoint/race coverage only, never power-loss proof. |
| 5 | Read-only compare, confirmed reload/relink, separate accept-current and exclusive copy with independent receipts/recovery. Latest alias/reservation fix gate: 15 conflict + 9 recovery + 20 notes + 36 lib cases = 80 passed. |
| 6 | One live eligibility/projection, immutable shadowed imports, epoch guards, stale selected/prior-session evidence and truthful provider outcome/currentness. Recorded 270 supported test executions in its original final gate, plus fixture/projection fixes; no live/native-model claim. |
| 7 | Thirteen thin CLI note commands, typed context and exact preconditions with schema-version-1 envelopes. Latest fix gate: 19 ask + 11 core + 11 notes + 26 CLI-unit cases; copy replay and reviewed interrupted accept-current covered. |
| 8 | Owned critical-job lifecycle, idle notice observations, exact text/generation scheduling and guarded close/switch. Latest fix gate: 49 native unit + 5 CLI cases and 6 worker-selected cases; R20 termination limitation remains. |

### Filesystem, durability and concurrency limitations

Task 2/5 observations identify the local Data APFS volume (`/dev/disk3s5`,
`/System/Volumes/Data`) as case-insensitive and normalization-insensitive,
including `Straße`/`STRASSE` expansion. These are prior measured fixtures,
not inferred universal filesystem properties. Other volumes, including HFS
and case-sensitive APFS, are unqualified; conservative absent-name vetoes can
over-reject. Supported-operation checks refuse unsafe fallback.

Regular files require `F_FULLFSYNC`; directories deliberately use explicit
`libc::fsync`, not `F_FULLFSYNC`. Successful syscalls and process-kill restart
tests do not prove power-loss durability. Foundation coordination serializes
participating writers only; arbitrary direct writers can race after precheck,
and conflict may be detected after submitted content is installed. Displaced
objects/recovery are protected, with no lossless arbitrary-simultaneous-editing
or blind-rollback promise.

R20: pinned GPUI cannot veto Dock/system termination. The optional best-effort
final-hook flush was skipped as unsafe: a newly admitted critical transaction
cannot promise the quit deadline while still requiring join. Existing admitted
critical jobs drain/join, possibly exceeding GPUI's 200 ms future deadline.
Unadmitted/unacknowledged typing can be lost on that route. Restart reconciliation
handles durably recorded interrupted saves, not never-recorded keystrokes.

### Native acceptance checklist — pending, not executed

A human must observe the native UI on the unlocked target Mac. Record observer,
date, HEAD/dirty state, OS/toolchain, filesystem/case/normalization, features,
second editor/version and whether it actually participates in Foundation
coordination (unknown is not participating). Record observations separately
from user acceptance. Do not use an original vault, existing trials, provider
or model resources.

1. From this worktree create **new, nonexisting** sibling disposable data/vault
   directories (never data inside vault). These commands are a checklist, not
   an executed Task 9 observation:

   ```sh
   case_root="$PWD/.superpowers/sdd/plan/manual-notes-$(date +%Y%m%d-%H%M%S)"
   mkdir "$case_root" && mkdir "$case_root/data" "$case_root/vault"
   python3 - "$case_root" <<'PY'
   import pathlib, sys
   root = pathlib.Path(sys.argv[1])
   content = b'\xef\xbb\xbf---\r\ntitle: caf\xc3\xa9\r\n---\r\n# Plan\r\nUnicode: \xce\xbb\r\n'
   (root / 'vault/plan.md').write_bytes(content)
   (root / 'expected-open.bin').write_bytes(content)
   (root / 'vault/empty.md').write_bytes(b'')
   PY
   TMPDIR="$PWD/.superpowers/sdd/plan/tmp" cargo run -p brn-desktop --features native-ui --locked -- --data-dir "$case_root/data"
   ```

2. In Notes choose only that vault, open `plan.md`, save unchanged, and run
   `cmp "$case_root/expected-open.bin" "$case_root/vault/plan.md"` in a second
   terminal. Record exact bytes and unchanged identity/timestamps. Open the
   same file in the named second editor; deliberately edit in BRN, Cmd-S, wait
   for Saved to Markdown, and independently verify bytes (not only rendering).
   A buffer's Recoverable in BRN state alone must not alter the file. Exercise
   empty content and byte limits; deliberately edit only intended text, retaining
   BOM/frontmatter/CRLF. Near-limit performance remains separate qualification.
3. Keep local BRN edits, externally change `plan.md`, return focus and attempt
   Save. Observe External conflict, retained local text and baseline/local/disk
   comparison, with no silent overwrite. Repeat on fresh fixtures for deletion
   (`rm "$case_root/vault/plan.md"`) and inode replacement (write
   `plan.replacement`, then `mv` onto `plan.md`). A deleted original must not be
   recreated by Save; equal-byte replacement must still require reconciliation.
   Reload refuses an inode change. Atomic-save editors such as TextEdit commonly
   replace the inode: observe that Reload refuses, then explicitly confirm Relink
   to the same path (retains local edits) before Reload with discard if desired.
   Do not treat atomic-save replacement as an ordinary in-place external edit.
   Never reuse an unresolved fixture as if clean: use explicit confirmed
   reload/relink/accept-current or a new case root.
4. Attempt Save a separate copy to an occupied `collision.md` containing a
   marker; verify refusal and unchanged marker. Then copy to a noncolliding
   `rescue.md`, verify exact local bytes, distinct identity/no inherited search
   approval, original buffer retained and unresolved original outcome unchanged.
   Confirmed reload must explicitly discard; relink must explicitly adopt identity.
5. Submit Save and type additional characters **while Saving is visibly pending**.
   Observe only the submitted generation saved and later typing retained/recovered.
   If the save finishes too fast to observe overlap, record this case not observed
   rather than claiming concurrency from a sequential interaction; deterministic
   worker tests are separate evidence.
6. Wait for a durable recovery acknowledgement without Save. Use window close,
   application Quit/menu/Cmd-Q and note switch separately, restart with the same
   disposable data, and verify recovered edits versus unchanged live Markdown.
   Pending/failed recovery must defer guarded close or require explicit discard.
   Record Dock/system termination separately with R20's limitation, not as a
   guarded route or a promised recovery of unacknowledged typing.
7. Exercise a test-induced interrupted save and observe its recovery in the
   native UI using the reproducible fixture below. Repeat with
   `phase="exchange_returned"` in a new fixture to observe an Applied outcome.
   The `prepared` fixture must reconcile NotApplied, retain the submitted work
   and leave original bytes unchanged. Replay must not write. Do not inject
   test hooks into production native code.

#### Reproducible interrupted-save fixture for manual observation

Run only when performing the pending checklist. This uses the existing
`cfg(test)` lib child and normal CLI enrollment, not SQL or a production crash
hook. It waits for the exact phase acknowledgement, kills only its child and
reaps it. Keep the UI closed until the child is reaped to avoid ownership races.

```sh
crash_root="$PWD/.superpowers/sdd/plan/manual-crash-$(date +%Y%m%d-%H%M%S)"
mkdir "$crash_root" && mkdir "$crash_root/data" "$crash_root/vault"
printf base > "$crash_root/vault/plan.md"
cargo build -p brn --locked
TMPDIR="$PWD/.superpowers/sdd/plan/tmp" cargo test -p brn-workflow --lib --no-run --locked --message-format=json > "$crash_root/test-build.json"
target/debug/brn notes open "$crash_root/vault/plan.md" --vault "$crash_root/vault" --data-dir "$crash_root/data" --json > "$crash_root/open.json"
python3 - "$crash_root" <<'PY'
import json, pathlib, selectors, subprocess, sys, time, uuid
root = pathlib.Path(sys.argv[1])
view = json.loads((root / 'open.json').read_text())['data']
request = dict(operation_id=str(uuid.uuid4()), note_id=view['id'],
               expected=view['stamp'], generation=view['stamp']['generation'] + 1,
               text='interrupted manual submission\n')
(root / 'request.json').write_text(json.dumps(request))
build = [json.loads(line) for line in (root / 'test-build.json').read_text().splitlines()]
binary = next(x['executable'] for x in build
              if x.get('reason') == 'compiler-artifact'
              and x.get('executable') and x['target']['name'] == 'brn_workflow')
phase = 'prepared'
import os
env = dict(os.environ, BRN_NOTE_TEST_FIXTURE=str(root), BRN_NOTE_TEST_PHASE=phase)
child = subprocess.Popen([binary, '--exact', 'notes::crash_tests::child_worker',
                          '--nocapture'], stdout=subprocess.PIPE, bufsize=0, env=env)
selector = selectors.DefaultSelector()
selector.register(child.stdout, selectors.EVENT_READ)
reached = False
deadline = time.monotonic() + 30
try:
    while time.monotonic() < deadline:
        if not selector.select(1):
            continue
        line = child.stdout.readline()
        if not line:
            break
        if line.strip() == ('ACK:' + phase).encode():
            reached = True
            break
finally:
    if child.poll() is None:
        child.kill()
    child.wait()
    selector.close()
assert reached, 'checkpoint not reached; do not claim interrupted-save observation'
print('Save operation:', request['operation_id'], 'phase:', phase)
PY
TMPDIR="$PWD/.superpowers/sdd/plan/tmp" cargo run -p brn-desktop --features native-ui --locked -- --data-dir "$crash_root/data"
```

In the native Notes/recovery page reopen the registered note, inspect its
interrupted operation, reconcile the printed operation ID, compare recovered
work/current file, and verify bytes externally. This checklist itself has not
been run or human-accepted; observed results must be appended before closing
the task. Clean up only the explicitly created case roots after evidence is
recorded and recovery is no longer needed.

### Controller rulings and cost if wrong

Source: the `Ruling:` lines in controller `.superpowers/sdd/plan/progress.md`.
R6/R7 do not appear in that source; they are not inferred approvals.

- R1: Worktree ignored through `.git/info/exclude`, not main's `.gitignore`; cost if wrong: other clones lack the trivial ignore entry.
- R2: Execution baseline is `main@9786d2d`, not `32077d8`, with unchanged crates/lockfile; cost if wrong: none observed.
- R3: Introduce `EvidenceCurrentness` and its sole ChatTurn constructor together; cost if wrong: minor Task 6 ownership overlap.
- R4: Update every exhaustive consumer when adding public variants, regardless of file ownership; cost if wrong: small overlap or broken intermediate compilation.
- R5: Temporary Task 2 adapter dead-code allowance must be removed by Task 3; cost if wrong: transient lint suppression persists.
- R6: No ruling issued in the supplied controller source; cost if wrong: unknown, do not invent authorization.
- R7: No ruling issued in the supplied controller source; cost if wrong: unknown, do not invent authorization.
- R8: Task 2 creates only private note adapter declarations/notices, Task 3 owns workflow content; cost if wrong: none recorded.
- R9: Native acceptance requires a human observing the unlocked Mac; cost if wrong: false acceptance, so task remains active/pending.
- R10: Add identity-bound monotonic store cleanup acknowledgement with resolved-terminal pruning gates; cost if wrong: cheap additive Task 4 seam adjustment.
- R11: Keep the mandated large store notes module in one file; cost if wrong: maintainability/future split.
- R12: Only Applied saves refresh the latest successful recovery pair; cost if wrong: harmless no-op not refreshing the pair.
- R13: Add registry/recovery/observation/enrollment replay APIs, amend unreleased V6, never silently rebase; cost if wrong: explicit clean-editor reload needed or schema amendment if already shipped.
- R14: Allow notice-drain dead code only until Task 8 removes it; cost if wrong: final lint suppression remains.
- R15: Proven pre-exchange refusal keeps Conflict/NotApplied, while changed destination leaves original resolution Unresolved; cost if wrong: misleading uncertainty and one fix round.
- R16: Relink must refuse an inode belonging to another registered note; cost if wrong: identity violation and one fix item.
- R17: Uncomputable alias fallback is scoped to equal parent identity or unresolved original parent; cost if wrong: differently spelled parent alias escapes (inode matching protects case/NFD parents).
- R18: Open must refuse reserved copy destinations; cost if wrong: applied copy becomes permanently unreconcilable and needs a fix.
- R19: CLI interrupted accept-current coverage may seed via public store intent API after workflow enrollment; cost if wrong: test coupling to intent API.
- R20: Guard window/application Quit/Cmd-Q/switch, document non-vetoable Dock/system route; unsafe optional final flush omitted as recorded in Task 8; cost if wrong: follow-up cancellable termination integration.
- R21: Controller uses worktree-local TMPDIR and restricted grep-backed rg shim; cost if wrong: stale-index text assertion uses grep rather than ripgrep.
- R22: Controller executes integrated verification and hands logs to Task 9; cost if wrong: none recorded, same command/environment; both actual runs/fix are disclosed above.
- R23: Preserve legacy human Current document rows, append only non-current state/reason, keep JSON metadata; cost if wrong: human column count differs between current/non-current rows.

### Handoff and next action

Architecture now records live-file saved authority, SQLite registry/unfinished
work, derived indexes and the single eligibility boundary. Historical evidence
is preserved; no active folder was moved to completed. Documentation whitespace
and local link/fragment checks are recorded in the final handoff below.

Next: a human executes and records the pending native checklist; resolve the
missing `protoc` prerequisite before rerunning only the native-retrieval
command, and report model-resource gates honestly without unauthorized model
acquisition. IME/accessibility, sustained near-limit performance, other volumes,
power loss and arbitrary simultaneous editing remain unqualified. Merge/push/
release and real-vault migration require separate authorization.

### Documentation handoff checks — 2026-10-01

At code HEAD `8574ed0`, `git diff --check` passed and
`python3 .superpowers/sdd/plan/task-9-links.py` validated **69 local
file/fragment links across 6 changed Markdown files**, with zero failures.
The throwaway checker lives only in the ignored worktree planning directory
and is removed after final checks; it checks local targets and GitHub-style
heading fragments, excluding fenced command examples/external URLs.
No record paths were moved or removed; the active-work/documentation indexes
retain and link this active task. Manual Python snippets received syntax-only
validation, not execution or GUI observation.

Pre-commit dirty state consists only of the six documentation files listed
above; no crate/code/script/manifest/lockfile changes. `git merge-base
--is-ancestor 8574ed0 main` returned nonzero: tested code is not in local main.
This qualification is recorded by the child commit with subject
`docs: record safe Markdown editing qualification` and the required Copilot
trailer; no merge, push or release is authorized or performed.

## Final review fix wave — 2026-10-01

Baseline: clean `feat/markdown-note-editing@29792a0`, in its existing isolated
worktree. Fix commit: `3348d9b` — `fix(workflow): resolve proven not-applied copy
and staging failures`. This evidence is recorded by the following commit,
`docs: record final Markdown review fix-wave evidence`; both include the required
Copilot trailer and pass `cargo check --workspace --locked` before commit.
Implementation of this fix wave: **implemented**; targeted automated verification:
**verified**; native/user acceptance: **pending**, not observed. No merge/push.

### Findings and rulings

- **I-1 / R25:** live copies with no installation attempted resolve NotApplied
  only after a fresh destination absence/non-prepared-identity observation.
  An EEXIST collision resolves using the exact recorded unconsumed prepared
  stage through additive store API `reconcile_note_copy_not_installed`.
  Reservation release changes only metadata; the occupant and stage remain
  untouched, and submitted recovery and the original failure replay survive.
  Interrupted-copy classification accepts this same stage proof. Explicit
  reconciliation can also resolve a previously recorded transient observation
  failure after proof becomes available, replaying that historical failure
  byte-for-byte unchanged. Without proof, copies remain reserved.
- **I-2 / R24 (revises R15):** when live progress proves exchange was never
  attempted, the matching destination baseline alone resolves an original save
  NotApplied. A stage whose creation failed before identity recording remains
  RetainedUnexpected; it does not turn execution disproof into Unknown.
  The original typed failure is preserved. An externally changed destination
  still reports Conflict/NotApplied with an Unresolved intent (R15); restart
  cannot infer live progress from phase alone.
- **Minor:** CLI/desktop READMEs and the pending native checklist explicitly
  distinguish in-place external edits from atomic-save inode replacement
  (for example TextEdit). Reload refuses the latter; confirmed same-path Relink
  retains edits, then a separately confirmed Reload may discard them.
  Reload semantics were not changed.

### RED before production fixes

Commands used `TMPDIR="$PWD/.superpowers/sdd/plan/tmp"` inside this worktree.
The new private cfg(test) staging-failure seam was present; production save/store
behavior was still unchanged.

```text
cargo test -p brn-workflow --lib notes::crash_tests::staging_io_failures --locked
assertion `left == right` failed: false/write: pre-exchange refusal has
unproven staging state: filesystem outcome is uncertain: injected staging I/O failure
  left: SaveUncertain
 right: Io
test result: FAILED. 0 passed; 1 failed; 0 ignored; 51 filtered out
exit 101

cargo test -p brn-workflow --lib notes::crash_tests::copy_eexist --locked
assertion `left == right` failed
  left: Unresolved
 right: NotApplied
test result: FAILED. 0 passed; 1 failed; 0 ignored; 51 filtered out
exit 101
```

### GREEN and final gates

Final commands ran with the same worktree-local TMPDIR, default features and
pinned lockfile on macOS 26.5 (25F71), arm64; rustc 1.98.1
(`48a229cea`, 2026-09-01), cargo 1.98.1 (`797e8a9bc`, 2026-08-05).
Each command exited 0:

| Command | Actual result |
| --- | --- |
| `cargo test -p brn-workflow --lib notes:: --locked` | 42 passed, 0 failed/ignored; 12 unrelated lib tests filtered. |
| `cargo test -p brn-workflow --test notes --test note_recovery --test note_conflicts --locked` | 21 notes + 9 recovery + 16 conflicts = 46 passed, 0 failed/ignored. |
| `cargo test -p brn-store --test notes --locked` | 42 passed, 0 failed/ignored. |
| `cargo test -p brn --test cli_notes --locked` | 11 passed, 0 failed/ignored. |
| `cargo check --workspace --locked` | Passed before the fix commit; repeated before this documentation commit. |
| `cargo fmt --all` | Passed; final format check also passed. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed, warnings denied. |
| `git diff --check` | Passed. |

New regressions cover replace write/attribute/file-sync/directory-sync failures
after stage creation, copy write/file-sync/directory-sync failures, successful
subsequent saves/copies, preserved unrecorded staging metadata, restart replay,
EEXIST with subsequent enrollment, live uncertain-copy reservation, and transient
observation-error reconciliation both before staging and after preparation.
Additive store tests exercise proof validation (wrong stage path/fingerprint,
unprepared intent, Replace kind, recorded exchange), atomic refusal, immutable
historical Unknown replay, independent enrollment after reservation release and
protected submitted recovery.

Local ignored logs: `.superpowers/sdd/plan/fix-red-staging.log`,
`fix-red-copy.log`, `fix-green-workflow-lib.log`,
`fix-green-workflow-integration.log`, `fix-green-store.log`, `fix-green-cli.log`,
`fix-check.log`, and `fix-clippy.log`. An intermediate workflow compile failed
because the new helper was an associated function but called without `Self::`;
that was corrected before all final gates above.

### Self-review and remaining qualification

No failure is rewritten into success; store result/hash/replay rules remain
unchanged. SQLite transitions are additive and transactional. No enum variants,
schema/manifest/lockfile changes, filename-based deletions, weaker filesystem
writes, real-vault access, provider calls, model acquisition or nested agents.
The new reconciliation paths only observe files and commit metadata; they do
not rename, create, unlink or alter content. RetainedUnexpected artifacts remain
intentionally present/protected, and historical Unknown results still do not
authorize cleanup even after reservation release.

**The integrated script was not run or rerun in this fix wave.** The controller
must rerun `scripts/verify-end-to-end.sh` and record its result separately.
Existing native-user acceptance, optional native-retrieval prerequisite,
power-loss/other-volume and arbitrary-writer limitations above remain unchanged.
