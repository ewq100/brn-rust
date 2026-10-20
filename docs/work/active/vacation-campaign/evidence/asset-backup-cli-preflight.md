# BRN asset backup/restore CLI preflight

Read-only inspection only; no workspace, CLI process, build, account, cache, native, or source operation was performed. Checkout: `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust`, branch `codex/agent-first-followthrough`, HEAD `83922d48126bb1da73791d8761ab9a13509262e6`; pre-existing dirty campaign evidence was left untouched. As assigned, the inspected source is product-identical to integrated `6622443e`.

## Supported synthetic route

There is a supported, provider-free route to make a real linked chat attachment with a retained figure asset, using public CLI import and the small synthetic fixture `crates/brn-threads-intake/tests/fixtures/luna/footnotes.docx` (52,486 bytes). Its maintained inventory describes an inline figure; the ordinary markdown CLI witness does not contain an asset. The stock Xberg helper exposes returned PNG bytes as retained assets, and full imports with AI Off need no credentials. This route depends on the fixture yielding a retained image; check for at least one `Asset` record immediately after intake and stop if none appears (do not substitute SQL or manufacture records).

On the requested Mac, with the already-installed supported `brn`, and qualified stock/adapted helper binaries beside it as required by the verification guide, lead can run the following in a fresh task-owned temporary root. Use a unique root; all paths are absolute. `ROOT` is a shell variable, not an existing workspace. The immutable fixture stays in the checkout.

```sh
ROOT=/private/tmp/brn-asset-backup-20261020
DATA="$ROOT/source"
RESTORED="$ROOT/restored"
BACKUP="$ROOT/backup.sqlite3"
FIXTURE=/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust/crates/brn-threads-intake/tests/fixtures/luna/footnotes.docx

brn --json --data-dir "$DATA" init --purpose test
brn --json --data-dir "$DATA" document-conversion --converter xberg --ai off
brn --json --data-dir "$DATA" thread-start --title 'Synthetic asset backup chat' --key asset-chat-1
# Take the chat ID from result.id in the preceding JSON envelope.
brn --json --data-dir "$DATA" intake --source "$FIXTURE" --intent chat --thread CHAT_ID --title 'Synthetic figure attachment' --key asset-import-1
brn --json --data-dir "$DATA" records
# Require at least one Asset record; save its ID and the Note ID from intake.changed.
brn --json --data-dir "$DATA" read --id ASSET_ID
brn --json --data-dir "$DATA" backup --destination "$BACKUP"
brn --json --data-dir "$RESTORED" restore --source "$BACKUP"
brn --json --data-dir "$RESTORED" read --id ASSET_ID
brn --json --data-dir "$RESTORED" read --id NOTE_ID
brn --json --data-dir "$RESTORED" records
```

For the exact-byte assertion, compare the complete serialized Asset record from `read` before/after restore, or compare `data.Asset.bytes` plus SHA-256 of those byte arrays. The linked Note markdown should contain the managed `brn-asset://ASSET_ID` URL. `records` should contain the Note, Asset, Source, and Link records; the Link from chat to note has relation `attachment`, and the asset record points to its note/source. Compare the IDs and full values of these relevant records before and after restore. Restore makes the copy an owner-purpose workspace; keep it read-only for this check.

To exercise supported refusal behavior without touching the backup: preserve a byte copy of `$BACKUP`, create a separate copy with non-SQLite bytes (or use a small literal corrupt file), then run `restore --source CORRUPT_COPY` into a fresh absent destination. Expect a rejected restore and no published workspace; assert the corrupt input bytes are unchanged. Separately, try restore into a deliberately non-empty fresh destination and verify its sentinel is unchanged. Existing code explicitly rejects corrupt/incompatible sources, symlinked paths and occupied targets.

## Permissions and practical limits

This synthetic sequence uses writes only under the unique `/private/tmp/...` task root and uses no owner data. It needs filesystem access to create the temporary roots/backup and execute the existing CLI plus document helpers on macOS. It needs no provider credentials, paid calls, network, model download, account action, or source edit. `intake --intent chat --thread` is the documented public attachment path; the CLI does not expose arbitrary Asset insertion.

## Existing witness overlap and gap

- `crates/brn/tests/threads.rs` has a macOS public-CLI full Markdown import/export/backup/restore witness. It compares restored notes and refusal/collision behavior, but its Markdown input has no figure asset.
- `crates/brn-threads-app/tests/chat.rs::retained_attachment_is_private_after_restart_then_keep_and_undo_change_same_note` seeds a byte-bearing attachment through the app test conversion DTO, backs up/restores, then checks visibility. It does not prove exact restored asset bytes or run the production CLI import path.
- Core backup/restore tests already cover corrupt/incompatible backup refusal, occupied destinations, symlink protection, and stage cleanup. Those are useful existing safety witnesses, but don't bridge the asset-bearing public CLI path.

Consequential missing coverage is one Mac-first end-to-end public CLI witness that imports a synthetic embedded figure with AI Off, verifies nonempty exact Asset bytes and attachment/source/link records, then proves those bytes and record identities survive backup/restore; the existing witnesses cover the two halves separately. The proposed manual run can qualify the behavior, while a lasting automated regression witness would need a stable scripted/local fixture path and should be selected by the lead.
