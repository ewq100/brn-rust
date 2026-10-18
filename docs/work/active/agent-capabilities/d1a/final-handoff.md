# D1A handoff for fresh Opus4 and principal disposition

Material source/test pin: **4db5b357e23cb0c4466e7fd91c71cf926cad44f8**, parent efc0e380fe3fb3ab872c8bfaa31fdfd1d41eaf65. Whole tree18f7abf2dc76ff7e623486ef062ae7c9af9a7108; crates tree3aa58760b268a20302ddabf449da9e0625704150. Clean at pin; later changes are evidence only. [Pin receipt](review3-pin.json), [actual material delta](review3-material-verified.diff) SHA256 `6e5d5e55736377c9d9632643b6537267b820ac570afd906f9d77d62d6fec57f7`.

Bound checkout `/Users/evokessler/.t3/worktrees/brn-rust/t3-9527c407`, branch work/d1a-20261016, original accepted local CW2 base b617d7842c1e668292b4f23c7b321ec195a76a1d. Principal owns architecture/integration/reviews/build scheduling. No publication, push, PR, merge, release, GUI, owner data or live provider/model invocation. No child delegation.

## Review material and scope

Read the [full verbatim Opus3 report](opus-review-round3.md), [response](review3-response.md), [source/artifact association](review3-source-association.json), and original plan/requirements. Opus3 confirmed round2 fixes and requested output adaptation for committed prepare/apply, source/historical reader fitting, and section continuation. Principal also identified overflow of compact dependent metadata as the same after-effect defect. This pin addresses those findings; independent Opus4 approval remains pending.

Exactly7 AI source/test paths change from efc:

- threads.rs: prepared_output/applied_output/change_write_output/change_acknowledgement; SourceArgs/schema/read_source descriptions and optional start; read_note section limit. Full-if-fits, compact-if-fits, then explicitly incomplete bounded metadata acknowledgement. Applied adaptation follows original attention/ledger work and cannot return a result-size error. No durable candidate/receipt/asset/hash/owner-review/Apply/Undo/replay mutation.
- threads_sources.rs: source_metadata adapts only the read_source tool after existing source_value figure-link remapping. Other source_value/ingestion consumers are unchanged. Exact UTF-8 offsets/coverage and stable source metadata; accepted source1MiB/assets limits unchanged.
- threads_history.rs: exact-version canonical reader output fitting, no current-note ledger grant; allocated refresh final return uses the same prepared-output helper. Existing scope/authority/refresh state are unchanged.
- threads_budget_tests.rs: seven focused new tests (six round3 witnesses plus the real one-character UTF-8 refusal edge), strengthened existing heading/partial/quotation evidence, and a cfg(test)-only intentionally oversized tool through the production BudgetHook + actual later writer. No production catalogue entry.
- threads_chat_tests.rs, threads_sources_tests.rs, threads_writing_tests.rs: only mechanical SourceArgs start0 literals.

Core830 archive-fence prerequisite, manifests/lock, shared actual CompletionRequest hook, request/result caps, local-origin diagnostics, parent/helper routing and primary source fixture are byte-identical to efc. Full old CW2/actual-working-Note-only/context-bound evidence semantics remain guarded by the existing tests. Opus1/2 findings remain closed subject to independent regression review. D1B summaries/C02/C06, D2/D3/D4, UI and writing features are outside this lane.

## Actual checks, source-bound evidence

| Command/phase | Actual result |
|---|---|
| Exact-efc production + test-only before, separate Git root/target |26passed/5behavioralfailed; four real per-result refusals and one wrong section cursor |
| First candidate focused |30passed/2fixture failures, raw retained |
| Corrected focused |32passed/0failed |
| Full brn-ai scripted --lib --tests |393passed/0failed/1existing ignored |
| Core note_read + message_search_scope |15passed/0failed |
| Full brn CLI,20raw summaries |161passed/0failed |
| Strict AI/core/CLI all-target Clippy -Dwarnings |PASS |
| Workspace fmt |PASS |

Rust1.98.1, locked/offline, CARGO_INCREMENTAL0, fresh synthetic TMPDIR. Actual normal checks ran at precommitHEAD efc with WIP diffSHA650be3ae8e6bc57bc7e5ba66001489500f755c4f0ab9037738dee22c0b759c77. The association compares every actual source/input hash against the verified material, records distinct before/candidate binary/core-library/dep-info paths+hashes, and retains raw log hashes/counts. No future pin is claimed contemporaneously.

The first corrected fixture failure lacked a newline before the next dense-section heading; the second tried to add a second active candidate for the same Note. Both were corrected test-only. Positive delivered quotation is now part of the original successful model proposal and verified in durable host evidence; original tail-refusal/count checks remain. No production guard was relaxed. The before run's five failures are behavioral, not compiler errors; committed Note/selected figure exact durable content is checked before the terminal assertion fails.

## Final targeted mutation supplement and release

The copied source remains the same genuine isolated Git root `/private/tmp/brn-d1a-20261016-target/mutation-202bf2d7/source`, with distinct copied target `.../artifacts`, never the candidate target. All294 material blobs match the pin before baseline; actual baselinePASS393/0/1 precedes rechecks.

[Exact per-ID/diffs/phase artifact hashes](mutation-final-per-id.json):

| Fixed manual mutation | Actual failing witness | Outcome |
|---|---|---|
| Return full post-apply output regardless of fit |Large committed apply model-loop + overflowing metadata acknowledgement |Caught |
| Wrong final acknowledgement refresh omitted count |Overflowing metadata acknowledgement |Caught |
| Ignore source page offset when slicing |Escaped source actual paging model-loop |Caught |
| Ignore section_end |Nonlast whole/cropped section model-loop |Caught |
| Initial UTF-8 search high +1 |No complete UTF-8 character fits boundary |Caught |

All five builds succeeded, then actual tests exited101 with the named failures. Every source mutation was restored/hash-verified. Phase artifact hashes are time-bound snapshots; later copied builds overwrite paths in that copied target, without touching candidate artifacts. No broad41/20 restart, old8seed rerun, timeout-as-catch or retroactive kill.

Original202bf core8 =2caught/6unviable; AI27/41 =16caught/7unviable/4missed,14withoutverdict. Efc manual8/8caught; automatic20 =9caught/6timeout/2missed/3withoutverdict. Those raw outcomes are unchanged. New-pin +1 was actually caught; efc +1 stays MISSED. [Exact /1 disposition](fit-prefix-survivor-disposition.md) qualifies redundancy only under current deterministic production sizing/full-text-reject preconditions, with raw MISSED retained. Timeouts/incomplete IDs remain distinct gaps.

[Final release receipt](mutation-final-release.json): controller session64547 exited0 and is reaped; own Cargo/rustc/test/controller processes empty; all294 copied material hashes restored; changed=[]; candidate material diff empty and source pin unchanged. The initial release-receipt parser forgot to skip the ps header; its error is retained separately, then corrected without any heavy command or source change, and its elapsed cleanup time is charged.

Single original1800s cap: prior1192.5642304420471 + final195.23291897773743 = **1387.7971494197845s charged**, **412.20285058021545s remaining**. All final planning/controller preparation/setup/baseline/build/test/restore/verification/cleanup is charged. Heavy explicitly released to principal; no new heavy command without explicit grant. Remainder is not consumed merely to fill the cap.

## Remaining gates

Implemented and locally verified are distinct from independently approved, integrated, native-observed, owner-accepted or merged. Fresh Opus4/principal disposition remains required. Full workspace/native/hosted/live/owner qualification remains principal-owned; earlier1094/1PDFUnavailable/6ignored workspace evidence belongs to its original pin. No new all-green workspace/F21/native-human claim. Model limits remain route-specific configured fallback metadata or explicitly unknown; byte/JSON admission remains an estimate, not token-fit proof. Provider overflow is a truthful backstop. No semantic superiority/live-cache metric claim. C02/C06 stay deferred to D1B.
