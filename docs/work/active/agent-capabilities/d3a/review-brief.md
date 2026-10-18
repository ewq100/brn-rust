# D3a independent actual-diff review brief

Review the complete base59d1152b..source-pin diff against the original build-sequence D3a table, amended Rig-first §7/S01–S04, dated owner D-S/D-M amendments and this bounded allocation. Principal requests a NEW independent Copilot Opus review; implementation does not self-approve. Local source pin/actual tree/diff hashes follow in the source-pin receipt. Earlier D1A/B/D2 refs and evidence are preserved, not reimplemented.

## Owned implementation and boundaries

- New AI `skill_packages.rs`, `skill_packages_fs.rs`, `skill_packages_tests.rs`, minimal lib export. Existing `threads_guides.rs` adds a strict metadata/parser validation seam; runtime prefix before cfg(test) is byte-identical to base. Existing catalogue descriptions are intentionally unchanged until D3b single-source runtime preparation; management reads actual built-in frontmatter and adds no new hardcoded description source.
- New app `skills.rs`, minimal lib registration and one real retention/reopen test. Workspace/checked Store supplies the actual data folder; no alternate DB/job/permission service. No model installation route.
- CLI four commands (`skills`, `skill-inspect --name`, `skill-install --source`, `skill-remove --name --digest`), normal contract3/envelope/help/spec/owner-command parsing, actual process journey, README. Install/remove owner-intent commands do not invent canonical receipts/request keys. Actor strings grant nothing. No enable/disable/select/load.
- Example `agent/skill-examples/document-review/` is copied through an actual service fixture, separate from compiled built-ins.
- **Exact principal-added core allocation:** Store::open_with workspace-entry allowlist/type condition adds only literal skills as a real directory; retains symlink rejection and all existing marker/adoption/ancestor/nonempty/other-entry/DB guards. Existing data_folder.rs has one appended synthetic witness; original prefix/assertions intact. No other core/backup/restore/schema hunk.
- Owned D3a docs and justified behavior/setup/verification/CLI summaries, active plan/evidence/status checkpoint. No manifests/lock/CI/scripts/auth/runtime/desktop changes.

## Supported format and filesystem contract

Strict **not-YAML** frontmatter: UTF8 SKILL.md <=64KiB, header<=8KiB, exactly name+description literal single-line key: value. Name `[a-z][a-z0-9-]{0,63}`; description1..1024 UTF8bytes. Reject duplicates/unknown keys, controls/Unicode literal line separators U+2028/U+2029, empty values, leading YAML sigils/quotes, nesting/tags/aliases/anchors/multiline/requirements. Normal Unicode descriptions and multiline instruction body remain valid. Body must be nonblank.

Root SKILL.md plus references/*.md|txt or references/subfolder/*.md|txt only; <=16files,64KiB/file,256KiB total,64 visited entries,3 path components,64 ASCIIbytes/component. Subfolder names under references cannot contain dots in this subset. Literal Markdown links must refer to existing package-root-relative content or fragments; remote fetching/dependencies unsupported. Ordinary inert URLs may appear as text. No shell/scripts/executable-mode files, invalid UTF8 or special/hardlinked/symlink files. Bounded JSON artifact544KiB accounts for permitted UTF8 text escaping (at most2x raw256KiB) plus bounded16ASCII paths/envelope. Real escape-dense exact256KiB fixture passes; +1byte/17thfile fails separately.

Descriptor-relative NOFOLLOW/DIRECTORY/NONBLOCK/CLOEXEC ancestor/file opens; regular single-link files; bounded reads/readdir; held descriptors survive ancestor replacement without following new outside symlink. macOS/Linux atomic no-replace rename. New managed skills/ requires exclusive creation or exact existing package marker, private non-group/other-writable directory. OS try_lock serializes cooperating management commands; WouldBlock is busy, other lock errors remain actual IO errors. Source folder copied, never moved/deleted.

At most64installed packages. Stable installed:name identity, explicit owner_installed origin, workspace_installation scope (not project grant), sorted length-framed SHA256 content identity including paths/text. Existing builtin metadata origin builtin/scope packaged; no fifth runtime ID. Installed enabled:false. requirements:[] means unsupported declarations, not model compatibility proof. Exact repeated install returns same identity; differing bytes conflict; explicit exactdigest removal then reinstall creates new version. Immutable validated artifact, stage write/fsync and exclusive rename. Normal failed publication cleans only same inode/dev stage; no overwrite. Removal verifies current exact digest, moves with exclusive rename, revalidates moved bytes before deletion, attempts no-overwrite rollback on mismatch; never deliberately deletes mismatched owner content.

## Acceptance map

| Registry | D3a actual scope | Explicitly remaining |
|---|---|---|
| S01 | Real parser/service/CLI install/list/inspect/remove, actual reopen after original source deletion, stable metadata/digest; four built-ins/runtime unchanged | Actual model lazy loading, frozen catalogue/bundle/settings/composer/activity and restart runtime selection: D3b |
| S02 | Malformed shape/UTF8/byte bounds/unsupported files or requirements, names/builtin collision/traversal, unsafe ancestor/leaf/FIFO/hardlink/executable, actual held-descriptor replacement, no-overwrite, artifact tampering, OS-lock/concurrent differentdigest import | Other-OS/native/human qualification and process-kill crash recovery are not asserted |
| S03 | Immutable package version, repeated same identity, conflict, digest-bound remove, no caller-folder mutation | Active run frozen version/update/revocation fencing: D3b |
| S04 | Installation-only scope, other Workspace sees no install, no new authority or model tool; actual search/export/backup exclusion with positive canonical control | Canonical enablement/project injection/restore-unavailable host fences: D3b; connectors D4 |

## Required review attention and known limits

Installed package bytes are local **configuration**, neither canonical work nor rebuildable cache. Machine-readable included_in_backup/export:false and actual management help/docs notice make exclusion explicit; existing backup outputs/formats are untouched. App fixture installs actual synthetic example, reopens after deleting source, proves canonical records/receipts unchanged and searchable, package instruction sentinel absent in current/restored search and actual Markdown export, canonical Note/version/search preserved through real SQLite backup/restore while package unavailable. This is exclusion proof, not full D3 backup success. D3b principal must reconcile canonical selection recovery promises before acceptance.

Normal failures clean current stage by identity; abrupt process death can leave private .stage-/.removed- entries, preserved/ignored rather than broad cleanup deleting unknown owner files. Excess entries fail closed by bounded enumeration. No crash-kill recovery or arbitrary same-user direct-filesystem-tamper serialization claim. Direct cooperating commands are serialized by the actual OS lock; descriptor anchoring and moved-content recheck cover tested path replacement. No native GUI/Windows/all-Unix portability claim; platform helpers exercised on macOS, Linux source not executed here.

Check actual S split, retention notice visibility, strict parser and narrow core condition, collision/no-overwrite/removal semantics, descriptors/lock races, bounded encoding and the real app/CLI routes. Preserve all existing authority/read/source/summary/continuation contracts. No full D3, source acceptance, live quality, hosted/native-human/owner/full-program or integration claim.

## Verification and unsuccessful attempts

See README.md, self-checks.json and the raw hashed receipts/logs. Exact base existing-API behavioral before witnesses are separate from later WIP Unicode guard before witnesses. Initial5/3, incorrect225KiB aggregate fixture, first Unicode witness stopping at U+2028, both-separator witness, new README contract failure and carried PDF Unavailable are all retained. No original assertion skipped or weakened. Latest capability431/0/1 +affected strict all-target lint +fmt pass. Earlier full AI439/0/1/defaultworkspace build/lint and fullworkspace1163/2/6 belong their recorded WIP fingerprints, not a fabricated future SHA. Targeted README1/0 resolves only that new contract defect. Fullworkspace command remains exit101; no broad rerun solely for docs or reviewer pending status.

A new <=1800s changed-guard D3a mutation campaign may begin only after immutable pin; setup/list/copied passing baseline/controller/cleanup all count. Prior budgets are finalized. Mutation outcomes are evidence per exact copy/diff/ID, not acceptance or retroactive kills. Principal review can overlap read-only.
