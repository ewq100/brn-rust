# Text/email Inbox — Stage 7

Baseline: actual PR43 merge d70ec0ac644ff3283ec89425b12ae1013dd94ada, reviewed
tree c2229a49e301031850cc5c5e3c6e91b8c43845fd. Stage6 implementation is integrated;
its native/live acceptance remains pending. Frozen vision§7/§23, architecture,
invariants and the development workflow govern this work.

## First deliverable: checked original-copy catalog

WorkStore V12 adds a narrow immutable Inbox catalog. Each nonnil item UUID binds
its complete kind (text, Markdown, email or Teams copy), bounded exact label/
optional original filename and whole ordinary-file identity/length/SHA256 proof.
The catalog stores operational metadata only, never original payload text or
current knowledge. Workflow will supply freshly checked durable owned-copy proof;
Store cannot verify filesystem durability. Identical creation replay preserves
original metadata/time; changed payload/proof refuses. Distinct explicit copies
remain distinct even when bytes match. Bounded ascending received-time/UUID pages
provide stable FIFO inventory. Reads, startup and restore validate full semantic
records, hashes/index bindings and the exact owned schema before corruption
recovery can discard readable work. Additive migrations preserve existing work.

This storage checkpoint does not yet expose intake to clients, process AI work,
convert content, delete originals or apply knowledge. No new datastore/framework.
All upcoming client capabilities go through AppWorker; the desktop owns no
private parsing, queue or authority logic.

Acceptance: V11 upgrade/validated backup restoration preserve exact work; whole
copy proofs/metadata survive restart and backup; replay never changes timestamps;
FIFO pages tie-break and resume without duplicates; invalid input and readable
semantic/schema damage refuse without replacing database or making backup;
physical corruption still restores a valid catalog backup.

Checks: meaningful Store tests, fresh storage/shared format/build/all-target
Clippy/workspace tests+52 offline fixtures, optional native workflow/UI paths and
shipping startup/schema12, independent read-only review, exact-head applicable
Mac/Ubuntu CI, normal merge and post-merge witnesses. Use synthetic explicit
directories; no live accounts, original/private files or downloads.

## Next deliverables

1. Owned exact text/email copies with crash-safe acknowledgement, shared
   AppWorker capture/read/list and CLI; retain originals and flag uncertainty.
2. A small owned bounded processing queue, individual/batch admission and joined
   cancellation/restart; faithful source conversion and full existing proposals.
3. Separate grouped consequences, source provenance, suggested identities/
   relationships/Actions/replacements and explicit conflicts/uncertainty.
4. Native Inbox/review with reproducible owner scenarios and safe original
   screenshot captures. Delete copies only after meaningful conversion and exact
   source approval; incomplete/unapproved/rejected work retains originals.

Saved Markdown source proof is distinct from raw intake. Action references to
pending sibling proposals cannot count as approved saved authority. Resolve that
concrete sequencing within the existing proposal boundary before implementation;
do not bypass proof rules or combine unrelated consequences into one proposal.

Pending external qualification carries forward in current status. Pinned Rig
invalid-streaming-tool stderr output needs a narrow correction before packaging.

## Catalog checkpoint — 2026-10-05

Implemented; independent whole read-only review is clean (all14Rust hashes pinned).
The first runtime witness failed on baseline V11 instead of required V12; fresh
9focused tests now pass/0failed/0ignored, including direct and validated-backup
V11 upgrades retaining complete Action/proposal/apply/completion and exact
unfinished work. Test-only Box/explicit-clock compilation assumptions were
corrected; they are not product runtime failures. Readable record/schema damage
refuses current reads/startup without replacing bytes or making backups; original
synthetic copies stay exact. Physical corruption restores a valid catalog backup
and skips a malformed newer candidate without changing that retained candidate.

Fresh pinned Rust1.98.1/macOS arm64: storage/shared format/build/all-target Clippy,
1160workspace tests/0failed/6ignored,52offline fixtures/retirement,
207nativeWorkflowModels/0failed/5ignored,257nativeDesktop/0failed/0ignored, both
native Clippy modes, shipping Desktop/CLI builds and2shipping startup/restarts
passed,V12/exact BOM-CRLF-Unicode/zero credentials.95affected/new/moved doc links
and diff check passed. Ignored subprocess helpers are exercised by parent crash
witnesses; native resource/model/GUI qualification is not claimed. Known upstream
block0.1.6 future-compiler warning remains. No accounts/downloads/private data.
CI/publication/integration follow; this is a storage deliverable, not completed
Stage7 user intake. Next: shared owned-copy capture/read/list, then processing.
