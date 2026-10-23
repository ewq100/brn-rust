# File workflows: Linux evidence and blocked native chooser

Observed 2026-10-23 UTC on the local `codex/linux-native-gaps-20261023` work
based on `7b416591275f2552823cb18fe357f5a20617168a`. This record separates actual
native observations from CLI service checks. No Mac acceptance, live-provider
qualification, publication or release is claimed.

## Native routes and current result

New → Import full note… opens the GPUI single-file chooser. Supported extensions
are DOCX, PDF, HTML/HTM, XLSX, PPTX, MD/Markdown, TXT and EML. Projects also exposes
Import file… for private scoped import. All formats use established restricted
Xberg helpers; AI conversion was off. Add useful information… is a separate
processing route and was not exercised.

Settings → Data exposes Create full backup, Change folder… and Export Markdown….
First backup use chooses/saves a folder; subsequent creation uses the saved folder
and a fresh filename. Changing folder alone does not create a backup.

**BLOCKED:** actual native Import selection displays “Could not choose a file.”
The portal bus monitor records `org.freedesktop.DBus.Error.AccessDenied`:
`Portal operation not allowed: Unable to open /proc/19341/root`.
This is the app's actual FileChooser request, not a successful chooser. Native
chooser Cancel, selected-file import, repeated selected-file import, backup
folder selection/Cancel/change, and native backup creation remain unverified.
No successful dialog interaction is inferred from CLI results.

The portal frontend exposes FileChooser v4 and owns its D-Bus name; admission
fails before the GTK chooser receives the request. A separate read-only Settings
request produced the same failure against `/proc/19803/root`. Reading
`/proc/19005/fd/1` also returned EACCES. Denied accesses were not retried or
bypassed. Cross-sandbox process identity inspection is the suspected cause;
changing security/access configuration requires authority. No identity spoofing,
portal validation disablement or security relaxation was performed.

**NOT IMPLEMENTED:** full-backup Restore has no desktop control. Other “Restore”
labels concern projects, layouts or work-status banners. CLI restore plus opening
a restored copy is separate coverage, not GUI Restore acceptance.

Source routes: `crates/brn-desktop/src/threads_tiles.rs` (`new_menu`,
`data_settings`), `threads_ui.rs` (`choose_import_for_reference`, `convert_file`,
`choose_output`, `choose_backup_folder`); CLI restore dispatch in
`crates/brn/src/main.rs`; checked restore in
`crates/brn-threads-core/src/restore.rs`.

## Rootless portal setup

Pinned `gpui-pre-linux 0.3.9` uses ashpd's FileChooser portal; it has no zenity
fallback. Debian trixie official packages were downloaded using the environment's
signed APT metadata, download-only, and extracted into
`/workspace/brn-environment/portal/root`. No host package installation, global
service configuration or persistent access was added. All library dependencies
resolved. Parent launched task-local GTK backend and frontend; helper inventory
started no processes. Activation/configuration and package manifest remain under
`/workspace/brn-environment/portal`.

| Official Debian archive | SHA-256 |
|---|---|
| `xdg-desktop-portal_1.20.3+ds-1_amd64.deb` | `9dce4bf7ca1cc9afce6d48735a4a9897a27bcfd17791a05031eb73ac76788922` |
| `xdg-desktop-portal-gtk_1.15.3-1_amd64.deb` | `84fdc449f05ffa0772e6eb4c6ccb348a353d2abd382dacc206ba3c5eee71f630` |

Archive origins are Debian's `pool/main/x/xdg-desktop-portal/` and
`pool/main/x/xdg-desktop-portal-gtk/` on `https://deb.debian.org/debian/`.
The source and package inspection establishes availability, not chooser success.
Both existing Xberg helpers are executable at normal activation paths.

## CLI checks completed separately

All inputs and data were synthetic under `/tmp/brn-linux-files-20261023`.
AI conversion was off; existing converter helpers were used.

| Check | Observed result |
|---|---|
| Full import Markdown, TXT, HTML, PDF, DOCX, XLSX, PPTX, EML | PASS after the setup/input corrections below; all eight formats returned success |
| Repeated full import | PASS: fresh observation receipt, existing Source updated to version 2, no duplicate Note; not UI repetition |
| Unsupported extension | Expected refusal: `invalid_value`, exit 2 |
| Malformed document | Expected refusal: `conversion_failed`, reason `invalid`, exit 7 |
| Two backups to fresh files | PASS `first.sqlite3` and `second.sqlite3` |
| Restore into fresh directory | PASS; restored workspace purpose `owner` |
| Restore again into existing directory | Expected refusal: `unsafe_directory`, exit 6; fresh empty destination required |
| Restore non-BRN input | Expected refusal: `unsafe_directory`, exit 6 |

The first Markdown conversion failed `conversion_failed: Io` with the activation
script's restricted TMPDIR. Using permitted `TMPDIR=/tmp` succeeded. The first
EML attempt used an incorrect fixture path and failed `invalid_input: unreadable`;
the corrected source succeeded. Those failures remain recorded and are not
reported as successful initial attempts.

Complete table comparison found source equal to backup for every compared table.
Restore matched backup except `workspace_profile`, intentionally changed from
`test` to `owner`. Evidence includes 27 records, 30 revisions, 14 operations and
14 receipts. Recovery/edit-session tables were empty in this fixture, so this
comparison does not establish nonempty recovery/session-token restoration.
Restore intentionally clears original edit-session bindings; original tokens
must not authorize writes into the copy.

Reproducible command shapes, using fresh absolute paths and activated helpers:

```sh
TMPDIR=/tmp brn --data-dir /absolute/synthetic/workspace intake --source /absolute/synthetic/document.md --intent full
brn --data-dir /absolute/synthetic/workspace backup --destination /absolute/synthetic/first.sqlite3
brn --data-dir /absolute/synthetic/fresh-copy restore --source /absolute/synthetic/first.sqlite3
```

Native reopening subsequently passed the observations below. These checks do
not close the blocked native chooser flows or missing desktop Restore feature.

## Evidence locations

Raw evidence: `/workspace/brn-environment/logs/linux-native-gaps/`.

- Native: `files-new-menu.png`, `import-picker.png`, `import-picker-ax.json`,
  `portal-bus-monitor.log` (actual AccessDenied reply).
- Import: `cli-import*.json`, `cli-import*.stderr`; `cli-import-summary.json`
  retains the initial EML error, superseded only by
  `cli-import-eml-corrected.json` for its corrected-path result.
- Backup/restore: `backup-first.json`, `backup-second.json`, `restore.json`,
  `restore-repeat.json`, `restore-invalid.json`, `restore-table-comparison.json`,
  `files-prebackup.json`, `files-restored.json`, `restored-settings.json`.
- Portal setup/logs: `/workspace/brn-environment/portal/`.

These are local evidence, not a published artifact or new PR.

## Native restored-copy observations and nonempty recovery follow-up

The native actor launched the restored copy and opened the imported
Markdown, imported PDF and ordinary sentinel. Screenshots inspected directly
show Markdown `Synthetic café — λ.`, the PDF “Harbor handoff study” text with
its coverage-not-independently-checked warning, and sentinel
`Synthetic sentinel — preserve exactly.` at version 1. This demonstrates native
reading of restored content, not complete PDF layout/figure fidelity or native
file selection.

A second backup, `recovery.sqlite3`, contained a nonempty unsaved edit. The
restored session preserved its ID, Note, base version 1, generation 1 and exact
`Unsaved restore recovery — café λ.` text. Table comparison found all tables
identical except intentional changes: edit-session bindings 1 → 0 and workspace
purpose test → owner. Service presentation changed holder caller → desktop.
Using the original token against the restored copy was explicitly refused with
`session_token_invalid`, exit 6; no token value is retained in these artifacts.

The actual restored native app showed Recover edit and opened that exact draft
with Unsaved/recoverable status. The native actor observed AT-SPI focus followed by
Ctrl+A/copy returned exact text; no independent clipboard artifact was saved,
so that observation is attributed to the native actor. The screenshot itself
shows the draft. The actual Save action committed version 2 with exact draft
text and empty recovery. A separate read of the original workspace still showed
version 1, its original sentinel text and its original unsaved recovery intact.
Thus saving the restored copy did not save or clear the original edit.

Selected durable evidence (synthetic only):

- [Restored Markdown](evidence/files/restored-markdown.png),
  [PDF](evidence/files/restored-pdf.png),
  [sentinel](evidence/files/restored-sentinel.png).
- [Recovery opened](evidence/files/restored-recovery-open.png),
  [saved](evidence/files/restored-recovery-saved.png).
- [Exact recovery/state excerpts](evidence/files/recovery-state-summary.json)
  include SHA-256 hashes of their full raw source JSON files.
- [Recovery table comparison](evidence/files/recovery-restore-tables.json),
  [old-token refusal](evidence/files/restored-old-token-refused.json).
- [Artifact hashes](evidence/files/SHA256SUMS).

No edit-token file or begin-edit metadata was copied. The native file chooser
remains denied; no further portal action was attempted by this documentation
helper. Full-backup GUI Restore remains absent.
