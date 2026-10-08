# Partial PPTX intake with attributed slide and notes evidence

Baseline main `deca5d5ea3f6decbfb6ae2288db3934d4de694c4` (PR96). Selected next
ready V1 document outcome: inspect and retain useful presentation evidence in
the existing Source review journey, both directly and as email attachments.
Branch `codex/p5-pptx-partial-intake`; session PR97 remains independent.

Reuse/adapt BetterOffice `betteroffice-pptx-parse =0.3.0`, already evaluated in
P1. Its cached crate checksum matches the P1 lock; typed package/model and
slide_source/notes_source inventories expose the required parts and shapes.
Reuse existing OPC aggregate admission, restricted helper, image decoding/dedup,
Extraction schema 1, retained snapshot, exact Source/assets approval and recovery.
Build only the typed evidence projection and attachment attribution remapping.
No second XML interpreter, renderer, schema, model runtime or network fetch.

## Bounded profile and interfaces

Helper accepts kind `pptx`. Preserve presentation slide order, authored text
paragraphs/runs/soft breaks/field display, nested groups and ordinary table cells
with row/column locators. Presenter notes body has a distinct source node and
label. Embedded complete PNG/JPEG bytes are retained once; every picture has a
distinct attributed occurrence and exact Markdown interval. Source nodes retain
exact slide/notes part bytes with parentage and stable part/shape locators. Hidden
content is labelled; image cropping/rotation/layout is not rendered equivalence.

Every native chart, SmartArt, inherited master/layout, unsupported drawing/media,
missing/external image or unrepresented inventory element has a located gap.
Root and affected nodes remain partial. Chart data absent from the qualified
projection cannot become an extracted fact. No silent completeness claim.

Validate typed model/source-inventory identity and traversal correspondence;
ambiguous joins refuse instead of guessing. Admit OPC ZIP aggregate/member/byte
budgets before parsing; set explicit bounded parser XML/shape/run limits. Keep
existing 16MiB input/inflation, 512 members, 32 assets, 2048 sources/occurrences,
8MiB text, 32MiB retained-source, image and wall/process cancellation limits.
Hard RSS enforcement remains unqualified. No unbounded parser defaults.

Email attachment integration maps local root onto the retained attachment and
remaps every child/parent/occurrence source ID, preserving each attachment's
identity and offsets. Two attachments sharing bytes remain distinct sources.
Failed conversion retains the attachment unprocessed with no half-bound nodes,
assets or occurrence ranges. Existing DOCX and MIME behavior stays compatible.

Workflow routes explicit EML/DOCX/PPTX names; unknown binary formats refuse with
a clear unsupported-format outcome. Native file chooser/help accepts these three
formats. All clients retain the existing workflow and Source approval path.

## Acceptance and verification

1. Harbor P1 slides retain prerequisite, notes deadline, two separate picture
   occurrences sharing one PNG; native chart omission is explicit and its slot
   values do not appear as extracted evidence.
2. Unrelated synthetic deck covers reordered part names, duplicate wording,
   nested groups, multiline text, table cells, hidden content and notes-only fact;
   exact slide/notes quotes and part bytes remain attributable.
3. The same deck attached twice to an EML keeps distinct parents and correctly
   remapped slide/notes/image attribution. Missing/external/unsupported content
   yields precise gaps; hostile/quota/ambiguous packages refuse recoverably.
4. Actual CLI import/process/Source proposal/approval/restart retains evidence and
   assets; discovery/replay with helper unavailable requires no reconversion or
   inference. Changed originals/stale approvals refuse, recoverable work remains.
5. Relevant intake/protocol/workflow/CLI/native tests, existing DOCX/MIME and
   full-size recovery witnesses, applicable Clippy/shipping/fixtures, one complete
   independent read-only review, required CI and normal protected merge.
6. Add expected presentation inspection/approval/reopening to the single morning
   UI task. Interactive qualification and personal usefulness remain pending.

Lead retains selected model/effort, owns workflow/native/shared docs/integration.
One bounded helper owns intake adapter/dependency/lock/tests only; independent
review later, at most two active helpers and no recursive delegation. Cargo is
serialized across checkouts. Stop/reassess dependent work only for a concrete
integrity defect or consequential unresolved product choice; continue elsewhere.
No private data, GUI, live inference, paid fallback, optional model download,
release, port, user-data reset or unrelated work is authorized by this slice.
