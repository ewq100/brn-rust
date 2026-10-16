# B3 acceptance mapping

The original B1b/B1a/CW1/CW2 receipts keep their exact source associations. The twelve native fixtures and six appended app scenarios have now executed successfully; see [actual checks and limits](verification.md). They remain local synthetic evidence, not native-human observation, independent review or acceptance.

| Prepared fixture | Actual boundary exercised | Acceptance |
|---|---|---|
| `b3_brief_reference_purpose_persist_through_chats_navigation_closed_chat_and_reopen` | Real scoped search result, native Attach/Purpose/Save/Remove clicks, checked setup, explicit new writing conversation, unrelated/closed Chat, Note navigation and separately reopened Workspace | F04/F06; no Note version/content change; canonical reference identity and optional free-text brief |
| `b3_stale_setup_save_refuses_preserves_form_and_checked_replay_is_exact` | Concurrent checked setup edit, same-key service replay, stale form save | F04/F10/F16; newer state preserved, form retained, no duplicate receipt |
| `b3_setup_labels_and_drafts_fail_closed_on_scope_switch_and_notify_only_revocation` | Same Note private Project/general; private reference/brief; knowledge-off and unlink/relink; notify without refresh | F14/F21 subset; no old reference controls/metadata in general or revoked context; explicit reload restores permitted state |
| `b3_full_note_import_attaches_once_and_wrong_target_completion_keeps_canonical_result` | Existing full-note import service and completion adapter, stable import key replay, Note navigation during conversion | F01/F02/F10; protected canonical imported Note retained, no wrong-target config mutation, partial original coverage |
| `b3_import_completion_refuses_stale_setup_and_revoked_working_context` | Target setup changes and working Note access loss while original scope remains an active legitimate import destination | F01/F10/F14; imported Note stays in original scope, no general/other setup fallback or overwritten brief |
| `b3_sources_used_separates_import_reading_claims_quotes_and_changed_removed_material` | Valid immutable host-evidence candidate; partial original import/read; later owner reference edit/config removal; transient malformed/forged display candidate | F08/F10; old evidence unchanged, changed/removed labels, provenance vs model claims, unavailable evidence renders no new source/quote/claim UI |
| `b3_configured_requirements_run_uses_real_shared_start_and_recorded_read_ledger` | Configured exhaustive/current capture, other/closed Chat, B1b real shared preparer and existing scripted runtime with actual read_note and prepare_changes tools | F06/F07/F08; requirements read in full over retained text while original import remains partial; attached unread Style source stays not read; exact quote/claim from host evidence |
| `b3_exhaustive_check_refuses_unavailable_requirements_before_new_saved_request` | Existing availability service after archive, new explicit exhaustive UI admission | F08/F10; no silent omission of an unavailable attached Requirements source, no new request/worker |
| `b3_saved_host_evidence_has_visible_sources_used_without_opening_inputs` | Only base-existing APIs create a real immutable partial-read candidate; rendered Sources used outside Inputs | F08; meaningful before fixture (missing UI), no absent-API compile-error proof |
| `b3_reimport_same_bytes_attaches_existing_canonical_note_without_a_duplicate` | Observation-only re-import receipt and checked app resolver | F01/F02; attach the existing canonical Note without duplicating or overwriting retained content |
| `b3_delayed_scoped_search_reply_cannot_publish_titles_or_counts_into_another_form` | Real scoped search pages delivered through the sole production reply handler after navigation/reload or notify-only canonical access loss | F14/F21 subset; stale generation/target/token cannot publish matches or counts |
| `b3_cancelled_reference_conversion_never_imports_or_attaches_and_clears_intent` | Captured intent + real Converted channel/Stop/poll path | F03/F16; no imported Note/Source/Asset/setup mutation; pending intent cleared; existing queue Stop semantics unchanged |

The native pointer/scroll and scripted runtime paths now execute in the actual test toolkit. Human native layout/IME/file-picker observation remains pending. The import fixtures enter the existing completed-conversion adapter using synthetic ConversionResult; real file-picker interaction and converter/provider fidelity are separate existing lanes, not simulated live evidence. Search service inputs are real scoped results; the persistence fixture supplies their completed page to the form to isolate Attach/Save. Production async search uses the same concrete finish_reference_search method exercised by the prepared delayed-result witness; the delayed-reply fixture is local reply-handler evidence, not a native-human race observation.

The Project/general fixture is a proportional new UI witness, not full F21. Existing original P1/P2 private-source disclosure/early Review+Compare/access-cycle/real typing/cache assertions remain byte-identical and require native suite coverage on the actual compiled candidate. Existing F05/F09/F12/F15/F16 queue/recovery/editor/Apply/Undo tests likewise retain original assertions and fixtures. No extra UI-only Sources-used policy, runtime or permission reader is introduced.

## Verification commands and remaining qualification

Use pinned Rust1.98.1, locked/offline, CARGO_INCREMENTAL=0, fresh synthetic TMPDIR, one task target bound to this literal source root. Separate before source AND target; never alias B3/candidate artifacts. Initial targeted commands after explicit grant:

```sh
cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop b3_ --locked --offline
cargo clippy -p brn-writing-ui --features native-ui,native-test-support --all-targets --locked --offline -- -D warnings
cargo clippy -p brn-desktop --features native-ui,native-test-support --all-targets --locked --offline -- -D warnings
cargo fmt --all -- --check
```

The current explicit grant permits maintained native/default/combined and relevant app integration; actual completed commands are recorded in verification.md. No zero-test or self-skipped run is proof. Library currently retains its 44 existing moved tests; desktop adds twelve registrations, with its one existing ignored fixture unchanged. Actual complete native outcome: 284 passed, 0 failed, 1 existing ignored.

F22 source gate stays native-only with no dependency/manifest/lock change. No ranking/index/model change is assigned; scoped search integration still needs existing scope/tool regression coverage per master §10. Missing native pdftoppm/PDF baseline, hosted Linux/macOS, full F21/program, native-human/IME/file-picker and owner gates remain open. Fresh independent Opus actual-diff review is principal-owned. B3 guard rechecks, if warranted after actual before/normal proof and a pin, require a separate original ≤1800s grant; B1b's closed budget is not reused.

## Additional app resolver grant

Six appended `tests/import_coverage.rs` scenarios exercise the readonly helper without changing importer behavior: initial/observation-only resolution and legitimate current owner edits; exact differing SHA X/Y and old-X-after-Y with Source.external_version still Y; fabricated/unapplied receipt and hidden scope; archived Source/Note/link, superseded or moved Note and moved link; ambiguous eligible Notes; canonical scope movement. All six appended scenarios passed in the eight-test import_coverage target. No new permission policy or metadata; actual applied receipt identity and existing scope/visibility predicates remain authority. Run `cargo test -p brn-threads-app --test import_coverage --locked --offline` under the current grant, with proportional existing intake/writing integration.

## Executed window outcome

See [actual qualification](verification.md) and its 34-command receipt. The complete native suite passed 284 tests with one existing ignored fixture; the library passed 44. Default workspace tests completed 1,115 passed, one carried PDF failure and six ignored. App native tests passed 246 with one ignored; combined desktop state passed 55. Default F22 graphs/build/strict lint, offline CLI journey (108 steps, 1,266 checks) and fresh/reopened headless startup also passed at their recorded source pins.

The persistence fixture additionally passes at 900×600 with bounded real wheel events and unchanged state assertions. That test-only supplement does not relabel the earlier full-suite execution. Native-human/IME/picker, hosted, owner, full F21 and principal acceptance remain pending. Heavy slot released; no mutation executed.
