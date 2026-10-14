# Rust Luna intake evidence

Candidate starts from `1d1555a364ec0f7d8a7e260a0420e9fa587784fe` in the isolated `codex/rust-luna-intake` checkout. Rust 1.98.1 and the existing lockfile remain authoritative. Synthetic fixture inputs only; the original Python research checkout is unchanged. Raw local checks and synthetic provider receipts are in the ignored `artifacts/` folder; final commands/results and integration receipts will be recorded here.

## Implementation and review

Schema-1 helper behavior is preserved beside schema-2 prepared evidence. Parser graphs remain restricted to the isolated helper; no ZIP/XML format dependencies enter SQLite core. Source preparation uses BetterOffice 0.3.0, mail-parser 0.11.8, inert html5ever/rcdom and new pinned Calamine 0.36.1 with bounded spreadsheet metadata. Poppler remains the explicit PDF facility, adding `pdftoppm` page previews. Existing hard quotas and native helper restrictions are unchanged.

The host formats structured sources and checks exact normalized source text, cell coordinates and metadata roles, parsed Markdown heading/list/footnote/table structure, image destinations/occurrences and source-preview page receipts. Luna returns a typed plan for table inspection, PDF page Markdown and crops; it cannot rewrite structured-file values or binding grammar. Crops use checked source-page coordinates, incremental asset/pixel/encoded-byte budgets and a separate local checking deadline. Only linked figures/crops are retained; full-page previews and originals are transient. Coverage remains unverified/partial without an independent inventory. Source bindings describe pre-persistence evidence; the checked service rewrites image destinations to canonical asset IDs.

The dedicated production model is fixed `gpt-6-luna` at high effort with explicit existing ChatGPT credentials, no canonical tools/history or general settings mutations. Its independent profile bounds the prompt to 256 KiB, candidate text to 1 MiB, images to 32/16 MiB under decoded-pixel quotas, and one request to 180 seconds. Rig's subscription Responses route does not support `max_output_tokens`; no generation-token cap is claimed. Refusal, malformed/truncated/incomplete response, unavailable account/model and excessive input fail visibly. Markdown/text do not require provider access.

Independent initial review accepted plan/authority boundaries and identified substring numeric-value corruption, unbound trailing prose, crop budget timing and inline figure ordering. These findings were fixed with targeted checks; final review remains separate from initial inspection.

## Verification in progress

- Prepared-source focused tests: 26 passed (`cargo test -p brn-intake --features helper prepared:: --locked --offline`). Includes same-coordinate attachment footnotes, sparse far-column cells, precise identifiers, blank comments, host-owned plan formatting, binding corruption and nested lists.
- Provider mock tests: 8 passed, including fixed model/high/no tools, image/hash metadata, refusal/truncation/EOF/oversized output, timeout/Stop, payload-free schema diagnostics and provider-free Markdown.
- CLI tests: 10 passed. Shared import tests: 2 passed, including differing candidate wording on a repeated source, dedup and Undo. Native queued-completion Stop test: 1 passed without saving Note/Source/Asset records or changing settings.
- Strict default workspace lint and complete storage gate passed before subsequent plan/font-runtime changes. The native state gate passed 147 tests. Final affected-tree gates remain pending; unsuccessful earlier lint log retained.
- Renderer restriction witnesses passed: system-font config contains no includes, only two host-owned environment variables reach the renderer, external synthetic source reads and loopback network remain denied, writes stay inside scratch, cancellation kills owned descendants. Legacy helper restrictions still use an empty environment.
- First live production CLI probe: nested HTML candidate returned in 8.498 seconds but failed candidate validation; no note was saved. Diagnosing the failed binding with synthetic-only candidate receipts. This is unsuccessful evidence, not a passing quality/speed claim.

## Live typed-plan qualification

Early full-Markdown model responses failed binding/list/escaping checks. Switching structured files to the prewritten Rust formatter removed copying and helper generation from the model. A single request now returns only bounded inspection decisions. Accepted final structured-fixture runs are below; these are measured qualification runs, not a fresh Luna-versus-Sol benchmark or a general latency promise.

| Synthetic fixture | End-to-end seconds | Source QA |
| --- | ---: | --- |
| nested HTML | 4.036 | Three list depths, literal ordinals, two occurrences of the same figure |
| footnotes DOCX | 4.782 | Scoped footnotes/endnote, human section labels, preserved chart values, explicit unresolved cached NOTEREF gap |
| metadata EML | 5.612 | Eleven named headers, unique body alternatives, exact blank/zero table values and dispatch figure |
| blank-comments XLSX | 3.970 | Source cell coordinates, leading zeros, formula/cache roles and four blank-cell comments |

Each passed prepared/plan/candidate/final-result validation and returned Partial coverage without an independent inventory. Independent Sol source QA confirmed these four outputs. Accepted receipts are `artifacts/qualify-plan-02-summary.json`; failing earlier receipts are retained separately.

PDF qualification remains in progress. Page preparation now takes about 3.7–4.8 seconds with one bounded native Poppler renderer and a host-written system-font config. Earlier trials failed native-wrapper selection/font startup, then typed schema parsing. The latest plan correctly identifies three previews and a figure crop but was rejected on numbered-list text fidelity; independent visual QA also found heading/list nesting and native extraction reading-order issues. These are being corrected before accepting the PDF output.

Full native observation, final broad checks, stable independent approval, PR CI, merge and merged-main verification remain pending.
