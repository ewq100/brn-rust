# Document conversion settings — owner feature brief

18 October 2026. The owner's decisions below define selected behavior, not implementation or verification claims.

## Feature

Use stock, pinned Xberg as the default converter. Offer Xberg + BRN layer as the second choice. An independent AI conversion On/Off setting works with both and defaults Off. No separate comparison screen or old-converter third mode.

## Problem

The previous approach tried to resolve every conversion discrepancy before replacing the readers. The owner now wants normal Xberg conversion first and optional improvements they can judge on their own documents.

## User outcome and core stories

- Import with normal Xberg output and no conversion model call by default.
- Select Xberg + BRN layer to use our completed adaptations and fixes.
- Turn AI On with either converter, or Off for its direct output.
- Reopen BRN with both preferences retained and existing notes unchanged.

## Expected behavior

Settings exposes **Converter: Xberg / Xberg + BRN layer** and **AI conversion: Off / On**. The defaults are Xberg and Off. All four combinations are real execution paths across the eight current kinds: PDF, DOCX, PPTX, XLSX, EML, HTML, Text and Markdown.

Stock mode uses the pinned unpatched library's normal output. Common admission, isolated execution, cancellation, provenance and checked save are always present; these wrappers are not optional content enhancements. Stock must not silently inherit our parser patches or content rewriting.

Optional mode uses an explicit versioned bundle of implemented, verified BRN adaptations. Incomplete work is preserved separately. A format need not have a visible enhancement before the two options are available; the product must not imply an improvement where none is implemented.

AI Off makes no conversion auth/provider request and retains the selected converter's direct output. AI On applies the existing dedicated conversion stage after the selected converter, using normal checked auth. It does not change general chat settings, grant tools or enable unrelated remote services.

Result provenance identifies the actual converter, optional bundle and AI stage. A failed mode never silently switches converter or AI setting.

## Important concepts and state

The two settings are independent owner preferences. Each import snapshots both. Source content cannot select a mode or grant authority. Changing a preference does not itself convert a document or call a provider.

## Lifecycle

Both preferences survive reopen and existing configuration backup/restore. A change affects subsequent imports; in-flight work retains its captured choices and already-sent processing cannot be undone. Previously saved notes are not rewritten. Existing import review, source identity, version checks, replay and Undo remain authoritative.

## User decisions

Confirmed: all eight kinds; exactly two converter modes; stock pinned Xberg default; BRN adaptations/fixes second; no old-reader third mode; Settings is sufficient; independent AI switch for either; AI Off by default. No new live-provider qualification window is activated.

## Trust, approval and autonomy

Documents remain untrusted. Both modes keep the shared checked import/save boundary, protection and scope. AI Off cannot require credentials. AI On cannot follow source instructions to change access. The setting does not authorize changes to existing notes. Existing transient-original and retained-content boundaries remain.

## Failure and uncertainty

Report the selected path's actual safe error; do not silently fall back. AI failure does not save a substitute result automatically. Failure or cancellation leaves canonical content unchanged and reaps owned work. Stock fidelity differences and library limitations remain honestly documented; normal output is not advertised as meeting the superseded complete custom-fidelity target.

## Relationships and examples

This changes document conversion, not other BRN AI/chat or useful-information workflows. Xberg + AI Off imports normal output. Changing only AI On adds dedicated AI conversion. Changing only the converter uses the adapted path with the same AI setting. The owner compares through the existing import workflow; no new side-by-side interface.

## Acceptance scenarios

1. Fresh settings show Xberg and AI Off; changing either persists independently and survives reopen.
2. Both engines process representative synthetic inputs of all eight kinds with AI Off and zero auth/provider calls, with truthful mode provenance.
3. Controlled provider fixtures prove AI On receives the output from either selected engine and the result uses shared checked validation. No live provider is required for routing proof.
4. Stock output matches the pinned stock API; the stock build cannot inherit the adapted patch bundle.
5. Reimport/replay cannot mislabel a prior mode's output as a new result. Reusing an operation key with a materially different request cannot silently return unrelated output.
6. Cancellation, missing helper, conversion failure and AI failure do not change existing notes, fall back or leave owned children.

## Non-goals

No comparison screen, legacy-reader third mode, automatic AI, added document families, new OCR/model downloads, credential inspection, blanket fidelity repair, new encrypted-PDF/password implementation or resumption of provider-screened tasks. Custom PDF/refusal and broader raw-cell/MIME repair work are not prerequisites for offering this reduced feature. Preserve their source, evidence and limits. This brief does not claim PR or program readiness.

## Open product questions

None for the reduced feature. Resolve engineering constraints without silently changing these decisions.

## Implementation considerations and handoff

Reuse canonical settings and existing app/CLI/native import services. Keep stock and adapted builds reproducible and isolated; global patches must not contaminate stock. Reuse the helper protocol and Luna boundary where compatible. The principal owns the plan, interfaces/dependency/schema allocations and integration. A bounded read-only check is establishing packaging facts, not rewriting this brief. Continue through implementation, meaningful synthetic verification, independent review and final integration under the owner's existing authority.
