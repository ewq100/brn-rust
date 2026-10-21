# Ordinary import target ownership implementation

- Checkout: `/Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust`
- Branch/base: `codex/import-target-owner` at `bd60131dfd8b4652b17c0b142c411bfe87550570`
- Candidate: `e7f06e5a04becff4148704f7ef13a3f24e87954c` (`Pass chat import target explicitly`)
- Scope: source-only migration, five desktop source files.

## Change

Removed `ThreadsUi.attaching_chat`, its initializer, both assignments and the picker helper's `.take()`. `choose_import` and `choose_import_for_reference` now accept `attachment_chat: Option<String>` explicitly. The chat attachment action passes `Some(id)`; the project menu action passes `Some(import.clone())`; two tile imports and two writing imports pass `None`; the validated writing-reference path passes `None` on its separate checked route. Existing conversion/worker payload remains unchanged.

## Inventory and preserved behavior

Searched all `attaching_chat` and `choose_import` occurrences under `crates/brn-desktop/src`. No `attaching_chat` occurrence remains. The six ordinary chooser callers match the selected mapping, and the only direct reference chooser passes `None`. `attach_chat_path` remains an existing direct conversion path with `Some(chat)` and was not changed.

Destination capture remains at the same caller moment before opening the picker. The helper's busy guard still precedes the native prompt; the post-picker busy guard and `convert_file` guard remain. Reference target validation, picker cancellation/failure handling, conversion, `WorkerEvent::Converted.chat`, and checked persistence were not changed. No cancellation defect is claimed.

## Validation and limits

- `git diff --check`: passed.
- Source inventory: complete for `attaching_chat` and chooser calls.
- No formatting, build, tests, native interaction, network/account activity, or shared-record changes; explicitly reserved for lead qualification.
- Candidate is unreviewed/unqualified and not integrated. No behavioral, native-usability, acceptance, merge, or release claim.
