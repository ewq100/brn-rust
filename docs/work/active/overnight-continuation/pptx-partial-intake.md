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

Workflow routes explicit EML/PPTX names and preserves the existing bounded DOCX
byte-validation attempt for other binary names. Existing valid DOCX fixtures
intentionally use arbitrary labels; removing that compatibility would be unrelated
behavior change. Names select parsers, never establish package validity. Unsupported
bytes still refuse recoverably. Native file chooser/help accepts these three formats. All clients retain the existing workflow and Source approval path. Native original
inspection supports the retained PPTX package through Quick Look; selected slide,
notes or shape evidence follows only exact parent IDs to its package, refusing
broken ancestry. No original preview is launched during overnight qualification.

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


## Qualification and independent review — 8 October 21:58 UTC

Initial final intake suite38 passed (17 maintained,9 PPTX,9 protocol,3 restricted
process), helper all-target Clippy/shipping passed. Workflow3 parent tests passed
plus exact helper-unavailable replay child invoked and asserted by parent; native
workflow all-target Clippy, CLI1 focused test and shipping native CLI passed.
Initial workflow fixture placed credentials inside data and correctly received
UnsafeCredentials; test configuration corrected to sibling directory, guard kept.
All failed attempt logs are retained under nightly receipts.

Independent complete baseline deca5d5 → candidate dcb51f1 + all staged/unstaged
files, binary diff SHA2a45ac3fdb7ee6da19bad60e41bdcf9890c656722a354b4bcfc57a6c9f0eda08,
found two valid P2 defects with actual restricted-helper synthetic witnesses:

- Generic application/octet-stream named PPTX converted but retained generic MIME,
  making exact package/slide original inspection unavailable. Successful typed
  parsing must preserve its validated package type while raw declared MIME stays
  retained in exact EML; no MIME claim alone can establish a valid package.
- Distinct-ID duplicate notesSlide relationships silently selected first notes.
  Reject ambiguous consumed singleton joins before projection, using the actual
  pinned root/notes/layout/master selector semantics; preserve valid collection
  families resolved by IDs. Add unrelated distinct-ID ambiguity witnesses.

Intake helper owns the sole Cargo slot for red regressions, minimal corrections,
final focused/full/Clippy/build. Lead will validate findings/fixes and refresh
independent review on the final complete candidate. No affected merge or live
trial before these correctness gaps are resolved. Final default/native/shipping,
actual CLI retained-case, required hosted CI and morning acceptance remain pending.


### Final corrected candidate — 8 October22:03UTC

Both P2 findings reproduced red and fixed. Full intake41tests passed (17maintained,
12PPTX,9protocol,3restricted process), all-target helper Clippy/shipping passed.
Final independent complete review clean at binary patch
2b12fbbc60e82f92f6552d902569107bcab08ea9afc27bae6c428b4311b1e8bc;
helper SHA7f058c359a092559cda55b07c3880b44e637257818ad60c725da8b1d99a9f8e5.
Native exact PPTX parent inspection and existing selected-attachment regression
passed; workflow3parents/1intentional child ignore and CLI focused witness passed
again with the corrected helper. No original preview/GUI was launched.

Actual retained CLI case `/private/tmp/brn-overnight-20261008/pptx-retained-case`
contains Harbor Draft Source5eeb31f3-7837-4674-b889-4c77cf0d8f13, Quay Applied
Sourcebbbfcd8f-5215-4890-afea-ac4415be5e51 with two exact assets, and repeated
attachments Draft313bd19e-107d-4d16-9fc4-5cd8334b6f97. One attachment declared
canonical MIME, the other generic; both preserve validated package ancestry and
exact originals. Quay fresh-process approval/restart/exact replay passed with
unchanged Original/installed file identities. Zero inference. Initial harness used
unsupported approve --file; resumed retained Draft using explicit UUID,
--review-version and --operation, without re-import/reconversion/reset/inference.

PR97 session controls now merged main441120ce7f6ee47de3e4fa0f784355e2fc0b3257
after all four required checks/docs in37849061646; merge tree equals checkedhead.
Next merge main into this independent candidate, preserve current docs/retained
cases, qualify final combined default/native/shipping/fixtures, then required CI
and protected integration. GUI and personal acceptance remain pending.
