# Next settled specifications

> Current planning context, 2026-10-07: the [proposed whole-system plan](../architecture-reassessment/plan.md) supersedes the old next-task order, custom-converter expansion and mandatory incidental-compatibility rules for reassessment. Existing observations/tests remain evidence; implementation is paused. Actual EML is required V1 and is not implemented by pasted Email text. Partial conversion may enter qualified draft review; original cleanup remains a separate protected operation. AI may recommend resolutions and revise unapproved work. No new design is accepted solely by this notice.

Written 2026-10-06; **Build-ready for explicit selection only**. Baseline is integrated main `a8deb9d8e94665b1731034675b490fb134aae091`, with unchanged product at PR77 `c75803832f3140347192bb08f2fdf13bb5fba1d4`. Recheck remote and candidate before work. No implementation was performed here.

Parent outcome: reduce duplicated general mechanisms while preserving observable BRN behavior. These are independent maintenance slices, not a prerequisite chain for V1. [H1–H5](../v1-handoff.md#ordered-task-queue) remain the tracker; this parent supplies detailed H1/H2 contracts rather than new duplicate tickets. [Reuse rule](https://github.com/ewq100/brn-rust/blob/af9239c741c7ab0983e62f0253e607b9607727e6/docs/development/workflow.md#compatible-reuse), [reuse decision](../../../architecture/decisions/2026-10-06-compatible-reuse.md), [invariants](https://github.com/ewq100/brn-rust/blob/af9239c741c7ab0983e62f0253e607b9607727e6/docs/architecture/invariants.md) apply.

## H1 — titles from saved Markdown

**State:** implemented and independently reviewed as a candidate; verification, acceptance and integration are recorded separately in the [H1 record](../h1-library-titles/evidence.md).

**Kind/readiness:** implementation, ready. Outcome: a fenced pseudo-heading cannot become a library title. Reuse Markdown1.0.0 AST and Store `note_identity::body_start`; build only the title-policy adapter. No new dependency, generic Markdown framework or note write.

**Files:** `crates/brn-workflow/src/library.rs`, `crates/brn-workflow/tests/library.rs`; examples of existing position-based parsing in `src/knowledge/links/extract.rs`; body contract in `crates/brn-store/src/note_identity.rs`. Current `title` scans raw lines and misses fenced blocks and `...` header closure. Existing library tests fix unmanaged, unclosed, BOM/CRLF and empty-heading behavior.

Required policy:

- Count the first50 physical saved lines, including header lines; line50 is eligible, line51 is not. The limit is not50 body lines. Preserve filename-stem fallback, Unicode and exact saved bytes/hash.
- Skip a complete supported leading header (`---` open, `---` or `...` close), after an optional BOM. A header ending beyond the window yields fallback. Reuse Store's offset; keep title derivation separate from metadata validity and query admission.
- In the body, choose the first nonempty level1 AST heading whose original physical line starts exactly `# `. Keep the existing trimmed **raw line suffix**: inline markup, links, escapes and closing hashes remain literal title text. Do not render/normalize AST children.
- Setext, indented headings, `#NoSpace`, `#` plus tab and lower-level headings do not gain title status. Fake headings in fenced code or HTML cannot qualify as AST headings. Empty eligible headings are skipped.
- Unclosed unmanaged headers continue to expose their body as ordinary Markdown, including the existing `Unclosed Title` case. Unsupported/malformed managed layout must not newly remove a note, alter metadata classification or abort refresh. Preserve prior eligible-input behavior with a narrow compatibility adapter if the Store offset reader refuses; use filename fallback on actual Markdown parse failure, without promoting unsafe metadata. If that adapter needs a second frontmatter interpreter, stop and document the gap.

**Acceptance:** through `Library::refresh/notes`, exercise backtick/tilde fences, an unclosed fence, HTML pseudo-heading, real/fallback, all raw-inline rules, BOM/CRLF, complete `...`, malformed/unclosed unmanaged input and lines50/51. Keep existing tests. Verify full note bytes/hash unchanged and metadata/scope behavior unchanged. Add one shared search/list witness where title consumption matters, using existing seams. No UI redesign or retrieval ranking change.

**Checks:** `cargo test -p brn-workflow --test library --locked --offline`; affected formatting and strict Clippy per verification; shared gate once for the resulting product change. macOS filesystem fixtures, pinned toolchain, disposable owned TMPDIR and separate Cargo target; no credentials/network/model. GUI acceptance is inapplicable unless display behavior beyond corrected title changes.

**Ownership/dependencies:** one implementation owner; no queue dependency. Lead owns shared status/decision/handoff updates and final integration. H4/H5 can evaluate independently. Stop on a required product-policy change or inability to preserve eligibility; record evidence, do not broaden into metadata refactoring. Completion requires exact candidate, relevant checks, independent review and finding dispositions, applicable CI/integration and accurate gate state.

## H2 — one compatible Action schema

**State:** implemented and independently reviewed as a candidate; verification, acceptance and integration are recorded separately in the [H2 record](../h2-action-schema/evidence.md).

**Kind/readiness:** implementation, ready with an explicit equivalence gate and non-adoption stop. No unresolved product requirement. Reuse already locked Schemars1.2.2 for structural derivation; adapt only required-nullable/provider representation. Existing manual schema remains the compatibility oracle until evidence supports removal. No H1/H3 dependency.

**Files:** `crates/brn-ai/src/action_candidates.rs`, `proposal_tools.rs`, `provider_formats_tests.rs`, `crates/brn-ai/Cargo.toml`, lockfile and pinned Schemars source. Current Serde data has14 required fields and tagged closed enums; tool schema independently duplicates them. Domain validation and deterministic Workflow authority remain unchanged.

Executable slice:

1. Capture the current tool/schema contract in behavioral equivalence witnesses before replacing it. Missing versus explicit null, closed nested shapes, Create/Replace tags, ActionRef variants and complete checked replacement refs are the seam.
2. Derive Action data/ref/enum structure using pinned1.2.2. Add a direct exact-version dependency only in this selected task if required. A required-nullable wrapper or bounded schema adjustment is allowed; `schemars(required)` alone is insufficient because it can remove nullability.
3. Preserve all existing structural constraints (required fields, enum/null unions, unknown-field rejection, item/count/number/string formats and bounds). Preserve Rust UTF-8 **byte** validation and Workflow UUID/date/full-record/approval checks; JSON Schema string length is not a byte validator.
4. Prove the emitted schema works in existing synthetic ChatGPT Codex Responses, Copilot Responses and Copilot Chat tool request formats. Inline `$defs`/references through existing facilities if needed; unproven provider reference support is not acceptable. Tool descriptions and capability/route selection stay as they are.
5. Remove only duplication actually replaced. Retain manual sections and publish a concrete non-adoption result if adaptation costs exceed the removed duplication. Other tool schemas, Rewrite and visual structured output remain outside scope.

**Acceptance:** every14-field deletion rejects while explicit null round-trips; nested unknown fields/tags/types reject; required-nullable generated schema preserves null; known valid/invalid boundary matrix matches previous schemas and Rust parsing/validation; each synthetic route carries the intended schema. Domain UUID minting, exact before records, approval, retries/cancellation and tool policy are unchanged. Evidence counts adapters/dependency changes as well as deleted duplication.

**Checks/environment:** `cargo test -p brn-ai --lib --locked --offline`, fmt/strict affected Clippy and shared gate for product changes. Pinned Rust with synthetic transports only; no account, live provider, model or GUI required. A schema's offline equivalence does not establish live endpoint support beyond existing qualified tool routes.

**Ownership/conflicts/stop:** one owner; serialize with H3 because both touch AI provider/schema tests and contracts. Lead reconciles shared records. Stop at a demonstrated equivalence/route blocker or disproportionate adapter; do not change accepted inputs to force adoption. Completion evidence and integration requirements are the same as H1; non-adoption closes the bounded task only when independently reviewed and recorded.
