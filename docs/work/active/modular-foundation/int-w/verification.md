# INT-W qualification receipt

**Compiled corrected source is ready for principal independent review; no acceptance claim.** All commands run serially from the SAME bound checkout at actual precommit HEAD `e8e98304af7c6dd871455f8081887bf6a1af3e16`, with their actual dirty-source manifests. Later commits identify checked bytes, never the HEAD reported by earlier commands. [Commands](commands.json) freeze every actual command, count, raw log/hash and per-command source manifest. [Material source](source-manifest.json), [test artifacts](test-artifacts.json), [final binaries](binaries.json) and [host lint test harness](host-lint-harness.json) bind the evidence.

## Boundaries and method

Rust1.98.1, arm64 macOS26.5; fresh explicit synthetic TMPDIR per command, `CARGO_INCREMENTAL=0`, locked/offline Cargo. Dedicated SAME-root target `/private/tmp/brn-int-w-20261017-target`; raw evidence `/private/tmp/brn-int-w-20261017-verification`. No copy/before/mutant checkout exists: both new canaries compiled against unchanged accepted production in the bound checkout, before the separately authorized production edits. No cross-root artifact reuse, cache cleanup or hidden background build.

Manual serial commands preserve `--no-fail-fast` coverage and source association. They are not claimed as exact execution of verify-storage.sh/verify-desktop-shell.sh. The actual maintained `bash scripts/verify-end-to-end.sh --fixtures-only` runs its retirement checks and both offline journey programs. No wrapper, filter, skip or suppression changes the PDF fixture result.

## Source progression and failures

1. Initial test development retained five non-production failures: active-candidate sequencing, a diagnostic Debug compile error, the observed InvalidToolUse from one active candidate, a diagnostic transcript of that same restriction, and the new fixture's stale receipt expectation after explicit Apply/Undo. These are fixture/diagnostic attempts, not production bugs or failing-before proof; exact attempts remain in the receipt ledger. Original assertions were untouched.
2. First comprehensive25-command traversal uses the first passing single-variant app matrix before the retained-Source variant/native canary or fixes. Default1116/1PDF/6ignored; helper64; capability357/0/1; library44+host5; native desktop287/0/1; retrieval35; app/native247/0/1; combined55. Only the carried default PDF failure. These are earlier-source results.
3. The retained-Source variant and actual native Notes tile expose TWO genuine production defects. [Exact findings/red receipts](privacy-findings.md). Principal inspected and allocated only export::publish and note_rows canonical-visibility hunks. Original red logs/manifests are unchanged.
4. Both focused corrected canaries pass; the Notes fixture then adds a private inbound-link count assertion. Final traversal qualifies that exact production/test material, not a renamed earlier result.

The first artifact-snapshot audit caught one superseded same-root focused-test binary after later fixture assertions caused a rebuild. Its old receipt remains historical; the newer file is never presented as that old execution's binary. The two preserved first-traversal binaries and mismatch record remain in scratch. Final-source artifacts will be verified separately against actual current bytes and resolved dep-info inputs.

## Corrected-source results

All `final-*` checks use one exact332-file material fingerprint: `ab43ba50cc7ce370f329c43a8fe27d40dc41957b5248a672eb502f8d9b13558b`. Both original test prefixes remain byte-exact ([prefix receipt](final-original-prefixes.json)). No manifest/lock/CI/script bytes changed. Cargo.lock remains SHA256 `0798be049da224edbfb4ce951a80fcd20b066fba6f9632c164b111d5e30de290`.

| Actual serial command / label | Actual result |
|---|---|
| `cargo test -p brn-threads-app --test cw2_review --locked --offline int_w_f21_private_context_never_enters_p2_or_general_delivery -- --exact --nocapture` |1 passed /0 failed /2 filtered; both private Note and retained-Source variants |
| `cargo test -p brn-desktop --features native-ui,native-test-support --locked --offline threads_ui::writing_tests::int_w_general_notes_list_hides_p1_reference_with_scoped_retained_source -- --exact --nocapture` |1/0/288 filtered, actual recent/kept title exclusion and private inbound-count exclusion in general and P2 display |
| `cargo tree --workspace --edges normal` and `--edges normal,features` (both locked/offline) |Pass; zero GPUI packages in both default graphs |
| `cargo tree -p brn-writing-ui --features native-ui --edges normal,dev --locked --offline` |Pass; no brn-desktop dependency |
| `cargo build --workspace --locked --offline`; `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` |Pass |
| `cargo test --workspace --no-fail-fast --locked --offline` |**1116 passed /1 failed /6 existing ignored**; sole carried native PDF Unavailable, qualification.rs470. Exit101 retained; not a green workspace test gate |
| Writing library `build --features native-ui`; strict all-target Clippy/test `--features native-ui,native-test-support` |Pass; **44 library +5 actual host tests**,0 failed/ignored |
| Native writing_host example build `--features native-ui` without test-support |Pass; actual reusable panel/service composition, no full desktop dependency |
| Desktop build/strict all-target Clippy/test `--features native-ui,native-test-support` |Pass; **288/0/1 existing ignored**. Original A/B, CW2 privacy/typing/cache, B3 hidden-draft/import, B1b shared-start, F15 editor Undo, queue/recovery assertions unchanged |
| App tests `--features native-retrieval --no-fail-fast` |**247/0/1 existing ignored** |
| CLI build `--features native-retrieval` |Pass |
| Desktop build/strict all-target Clippy/test `--features native-ui,native-retrieval` |Pass; **55/0/0** state tests |
| Actual maintained `bash scripts/verify-end-to-end.sh --fixtures-only` |Retirement checks, Save/recovery/search/export/backup/startup/refusal passed; CLI **108 steps /1266 checks** |
| Final combined binary `--headless-check startup`, fresh then reopen over same fresh synthetic directory |2/2 success, same binary hash; canonical threads.sqlite3 present, retired brn.sqlite absent. No GUI/native observation |
| `cargo fmt --all -- --check` |Pass |
| `python3 -m unittest discover -s scripts/tests -v` |**156 tests passed**; synthetic error-path fixture output is not a live-provider run |

Every Cargo command above uses locked/offline; the ledger preserves full literal arguments, target, TMPDIR, source hash, elapsed time, exit and log SHA. Native retrieval/combined use only the pre-existing cached arm64 ORT1.28 archive, SHA256 `d68759bf6c644dd5c6cb4e735699fba89e431b49ca588f73c11ee1c48ba838b3`; no model assets/download or semantic relevance claim. [Tool identities](tool-identities.json) retain actual compiler/cargo binaries, Poppler26.10 pdftotext/pdfimages and helper; native pdftoppm is absent.

Helper build/strict all-target lint/64 tests, capability strict lint/357 tests +1 existing ignore, and native retrieval exact CI lib+model_download35 tests passed in the first traversal. Their production/dependency inputs are unchanged by these two app/desktop corrections; they are retained at their earlier source manifests, not relabelled as final-source executions or repeated without cause. The default and affected app/native/desktop/CLI/combined/offline lanes were rerun because the two real production changes warranted it.

The example's ordinary native test header executes all5 journeys, including real scripted shared preparation/tools/candidate/exact review/Apply/Undo, editor value after Apply, shortcuts/menu action dispatch and reopen/own scratch lifetime. Strict all-target lint separately emits `writing_host` **profile.test=true** with journey.rs in its dep-info; the non-test example artifact alone is not that proof. Native GUI right-click/quit and pre-render first-edit observation remain unqualified.

### Existing ignored cases

Default6: provider stderr child, native_check_store owner-supplied data fixture, backup stage-holder child, process-tree child, renderer-probe child and sandbox-probe child. Native desktop1: `a_desktop_supply_reuses_the_chat_a_cli_useful_intake_left` requires supplied CLI fixture environment. App/native1 repeats the same native_check_store fixture. Full literal reasons/names are in commands.json; counts overlap lanes, never sum them as unique coverage. No ignore or assertion was added, removed or weakened.

## Static gates and review state

Current Markdown links:229 files /1145 local links /0 failures; all maintained shell scripts pass `bash -n`. Full development tooling156/0. `git diff --check` passes. The command ledger retains61 actual attempts,26 on the corrected material identity; earlier fixture/diagnostic failures and both genuine red canaries remain distinct. Counts are not sums of unique tests. Raw logs remain in the named scratch directory; four red/final canary logs plus exact historical test-only patches are also committed. No unchanged broad tests are repeated after the final traversal.

All332 current material hashes and every final actual Running test artifact were reconciled. Historical superseded binaries retain their old execution hashes and are explicitly marked as no longer matching current files, never silently relabelled. Final executable/dep-info inputs are in binaries.json; no cross-root cached workspace artifact or copied-source target was used.

Fresh comprehensive independent Opus review and principal acceptance are pending. The subsequent evidence-only handoff records the immutable compiled source pin/tree and explicit heavy/path release. It does not alter production/test bytes or attribute commands to a future SHA.

## Open qualification

Native-human/GUI/IME/picker/menu/quit/first-input pre-render observation, live provider, hosted Ubuntu/macOS candidate contexts, owner/program acceptance and native pdftoppm remain unperformed/unavailable. D-lane source is not integrated. F13 ranking evaluation is inapplicable without ranking/index/model changes, while required scope/read/search checks execute. Existing ignored fixtures remain identified; no new ignore. No new mutation campaign is authorized or run.
