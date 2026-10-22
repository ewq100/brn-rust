# MAC-13 command and artifact route

Source baseline: `main213` `3827d6a13953533c7322fd7157d975a32dee5cb2`; read-only checkout HEAD `34399938a84c0ae4e014c42350acec5258303a1d`, no production-crate diff from that baseline. Existing lead-owned campaign/overview edits were preserved. No workspace command, fixture, build, test, or native action was run.

## Checked synthetic preparation

Use a unique canonical absolute root, absolute input/token paths and unique CLI keys. Do not pass `init --purpose test`; MAC-13 needs an owner workspace. Use an exact-byte `.md` copy of `native-fixtures/save-initial.txt` for input; retain `save-final.txt` for the later version advance and `unsaved-recovery.txt` for actual typing. Verify the importer’s resulting Markdown; do not assume it is byte-identical to the input.

```sh
BRN=/absolute/path/to/qualified/brn
ROOT=/private/tmp/brn-mac13-private-UNIQUE
SOURCE=/private/tmp/brn-mac13-fixtures/private-initial.md
SENTINEL=/private/tmp/brn-mac13-fixtures/general-sentinel.md
ADVANCE=/private/tmp/brn-mac13-fixtures/save-final.txt
TOKEN=/private/tmp/brn-mac13-fixtures/advance.token

"$BRN" --json --data-dir "$ROOT" init
"$BRN" --json --data-dir "$ROOT" thread-start --title 'MAC-13 private chat' --key mac13-chat-UNIQUE
# CHAT is result.id from the thread-start JSON envelope.
"$BRN" --json --data-dir "$ROOT" intake --source "$SOURCE" --intent chat --thread "$CHAT" --title 'MAC-13 private note' --key mac13-private-UNIQUE
"$BRN" --json --data-dir "$ROOT" note-create --title 'MAC-13 general sentinel' --text-file "$SENTINEL" --key mac13-sentinel-UNIQUE
# NOTE is the after.id whose after.data.Note is present in the intake receipt.
"$BRN" --json --data-dir "$ROOT" read --id "$NOTE"
# Capture result.version as BASE. In BRN, select CHAT and use the attachment's Inspect control while clean.
"$BRN" --json --data-dir "$ROOT" begin-edit --note "$NOTE" --base "$BASE" --token-out "$TOKEN"
# SESSION is result.id; first update generation is 1 (new session starts at 0).
"$BRN" --json --data-dir "$ROOT" update-buffer --session "$SESSION" --generation 1 --text-file "$ADVANCE" --token-file "$TOKEN"
"$BRN" --json --data-dir "$ROOT" save --session "$SESSION" --generation 1 --token-file "$TOKEN"
"$BRN" --json --data-dir "$ROOT" read --id "$NOTE"
```

All responses are one JSON envelope. Require `.ok == true` each time. `thread-start` and `note-create` place the created Record directly at `.result` (ID `.result.id`). Chat `intake` places `ApplyOutcome` at `.result` (`.result.outcome == "Applied"`, `.result.receipt.writes[]`); extract Note ID with `jq -er '.result.receipt.writes[] | select(.after.data.Note != null) | .after.id'`. `read` returns the Record at `.result`. `begin-edit` returns session fields directly at `.result` plus `.token_file`; with `--token-out`, `.token` is null and the token is written to the new file. `update-buffer` returns `{outcome, session}` and must be `Updated`; Save returns `{outcome, receipt}` and must be `Saved`. The API ignores an update whose generation is not greater than the stored one (`crates/brn-threads-core/src/edit.rs:252-266`). Keep the token out of argv and logs.

For independent supported reads, use `records`/`read --id` for private Note bytes, version, `chat`, import/protection metadata, and attachment Link; `notes` for the general Note projection; `history` for checked operation receipts; `recovery` for open sessions. After actual desktop typing, confirm exact recovery body and original base; after refusal also compare source, sentinel, receipt list and general Note/search/export projections. The checked CLI Save copies the current Note, changes Markdown and possibly confirmation, and preserves its other fields (`edit.rs:301-320`); capture actual version rather than assuming it.

Actual visible route: open CHAT from the normal Chats/Threads view and click the attachment `Inspect` control (`crates/brn-desktop/src/threads_chat.rs:674-719`). It calls ordinary `open_note`; `active_writing_context` derives the Note’s saved `chat` scope (`threads_ui.rs:1183-1207`; `brn-threads-app/src/writing.rs:145-169`). Do not use `Keep in BRN`, which makes the attachment general.

## Artifact identity

No exact-main213 CLI/helper qualification was found. The existing CLI receipt is main208 `7dde6a41a4fbc66468e0400d2cc3c859f04c8ef5`, binary SHA-256 `edf780009231fb887ef9db15d9e34e9bb1557caa914976e1675b8f80de412301` (71,484,016 bytes); it cannot qualify main213. Its recorded build is the narrow reusable command `cargo build -p brn --locked --offline --jobs 1`, with repository-pinned Rust `1.98.1`, from a clean exact-source checkout and followed by source/tree, binary hash and size receipt. Rebuild for the selected current source; do not relabel the old artifact.

CLI intake calls `DocumentHelpers::discover()` and converts the absolute source path (`crates/brn/src/main.rs:1460-1590`). By default it looks beside the CLI for `brn-xberg-helper` and `brn-xberg-helper-brn`; development overrides are `BRN_XBERG_HELPER` and `BRN_XBERG_HELPER_BRN` (`crates/brn-threads-intake/src/documents.rs:11-27`). Existing helper builder is `scripts/build-document-helpers.sh --offline --jobs 1 --target-root ABSOLUTE_TASK_TARGET`, pinned to Rust 1.98.1 and cached Xberg 1.3.10; it fails offline if the required archive is absent. `scripts/verify-storage.sh` also builds/copies both helpers before the workspace, but is a broader gate than needed for CLI fixture prep. At native use, reconcile the actual desktop binary and both helper hashes to approved source. `native-fixtures/manifest.json` calls for a workspace created by the actual app; if that requirement is literal, use the app’s ordinary first-run flow on the same fresh root. A CLI-created root alone is not native qualification. Also, the existing `change_saved` test changes state in a same-process test workspace before typing; a separate CLI process preserving the live desktop’s displayed base still requires actual stale-banner observation.
