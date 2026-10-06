# Compatible reuse within frozen V1 — 2026-10-06

**Status:** accepted development rule, explicitly requested by the owner on
2026-10-06. The [workflow rule](../../development/workflow.md#compatible-reuse)
is canonical. Component replacements below are separately classified; they are
not all accepted or implemented. Source inspected at merged PR77
`c75803832f3140347192bb08f2fdf13bb5fba1d4`; earlier main76/PR77 heads were only
reference points. This closeout changes documentation, not product behavior.

## Implemented mechanisms and authority

Rust/macOS/Rig and the six existing crates remain V1's direction. Rig0.43.0
(including the existing narrow vendored agent safety patch) supplies agent/runtime
and provider transport; static `brn-ai::behavior` supplies task instructions and
capabilities, while workflow supplies BRN context and deterministic authority.
Markdown1.0.0 already parses saved links. Rusqlite0.40.2 supplies SQLite; WorkStore
owns operational integrity/recovery and the retrieval database stays disposable.
GPUI supplies native UI infrastructure; clients retain the shared AppWorker seam.

DOCX already reuses zip8.6.0, flate2 1.1.10, quick-xml0.41.0,
roxmltree0.21.1, png0.18.1 and crc32fast1.5.2. These are library decoders.
BRN's custom work is package admission, meaningful-content interpretation/refusal,
bounds and cancellation, exact original/asset proofs and Markdown rendering.
Saved-state scope, identity, freshness, provenance, exact comparisons, proposal
validation/approval, filesystem effects, indexing, recovery and Undo remain Rust
responsibilities. Library parse success and AI confidence grant no authority.
No Pi migration or new framework/datastore follows from this reassessment.

## A — CLI parsing: evaluated, not adopted (H5)

Requirement: remove generic scanner/help maintenance while preserving BRN commands,
JSON errors, exact-token help precedence and validation before workspace access.
Current `crates/brn/src/cli/mod.rs` manually scans globals/options, rejects duplicates
and unknowns, and selects subcommands. Domain execution is a separate responsibility.
[clap4.6.7](https://docs.rs/crate/clap/4.6.7) is a published candidate, absent from
the lockfile. Its [Command API](https://docs.rs/clap/latest/clap/struct.Command.html)
offers non-exiting `try_get_matches_from` and configurable help/version flags.

Generic parsing is a plausible replacement; BRN's early exact `--help`/`--json`
handling and error-envelope adapter still belong locally. Adoption is unresolved
until one representative nested command, duplicate/unknown/global option matrix
and current help precedence prove compatibility. Count the new dependency graph,
help/error adapters and command declarations. Stop that evaluation with an adoption
decision and migration boundary; do not migrate the entire CLI during evaluation.

**Result, 2026-10-06 (build: keep the local scanner).** The
[H5 evaluator](../../../experiments/h5-clap-cli/README.md) ran a clap4.6.7 candidate
for globals, `status`, `search`, `notes show`, nested `inbox add|list|show`,
`findings close` and `models download` against the real `brn` binary at
`main@450eaa2` over 104 synthetic argv cases. 95 matched exactly. The other 9 were
usage errors with the same exit code and envelope command but a different message.
Every run stopped at parsing or admission and created nothing in the data directory.
To get there, clap had to be configured against its defaults: BRN's exact-token help
pre-scan and static help stay, globals are declared per level because propagated
globals overwrite silently, hyphen values are enabled and re-checked, `--` is
pre-rejected, and argv is re-walked because clap errors carry no subcommand path.
The candidate needs about 255 lines of tree builder and adapter to replace about 130
lines of generic scanning, adds `clap`, `clap_builder` and `clap_lex`, and leaves the
command tables unchanged. There is no migration boundary. Reopen only if the owner
accepts clap-generated help, wording or ordering, or a new requirement such as shell
completion outgrows the local scanner.

## B — tool-schema structure: accepted later, task H2

Requirement: a single structural source for typed arguments and provider schemas.
Schemars1.2.2 is already locked transitively through Rig; BRN has no direct schema
dependency. `action_candidates.rs` supplies Serde tagged enums and required nullable
fields; `proposal_tools.rs` repeats their object/enum structure manually.
Accept using the existing Schemars version for compatible structural derivation
in a later narrow change, contingent on focused equivalence tests. Keep tool
descriptions beside their implementations and domain/UTF-8 byte limits in Rust.

Pinned `schemars_derive::schema_exprs` chooses optionality by generation contract;
the default deserialize contract treats `Option<T>` as optional despite BRN's
`deserialize_with = required_nullable`. Merely adding `schemars(required)` removes
nullability (`Option`'s non-optional schema delegates to `T`), so it is not a drop-in
fix. Test an explicit required-nullable wrapper or small generated-schema adjustment,
including every14-field Action and nested enum. `deny_unknown_fields` can generate
`additionalProperties:false`; provider `$defs`/references and enum unions still need
route tests. Keep matching manual schema sections if the adapter costs more than
the duplication removed. Evidence: pinned1.2.2 sources and
[Schemars attributes](https://docs.rs/schemars/1.2.2/schemars/derive.JsonSchema.html).

## C — Rig structured output: unresolved, bounded task H3

Requirement: schema-constrained candidates without weakening streamed completion,
cancellation, limits or the selected provider/model and retry policy. Rewrite and
VisualInterpretation now share the static typed behavior boundary but still request
JSON through instructions. Pinned `vendor/rig-agent/src/agent/builder.rs` exposes
`output_schema`, `output_schema_raw` and `output_mode`; no new runtime is needed.

Rig0.43.0 maps native schemas to Responses `text.format` and Chat Completions
`response_format`. BRN uses ChatGPT's Codex Responses dialect and Copilot's
model-selected Responses/Chat routes. Source encoding does not prove live endpoint
acceptance. `OutputMode::Auto` can choose a synthetic final-answer tool;
`Tool` would alter the current no-tools visual behavior. Start with explicit Native
mode on the small description/uncertainty response in synthetic transports, checking
the actual three route shapes. Preserve current strict response parsing, known
Stop finish, bounded streaming, cancellation, no retry/provider fallback and separate
exact approval. Evaluate Rewrite composition afterwards; no live qualification is
authorized. Record whether offline adoption is safe or endpoint support is a blocker,
and stop without speculative fallback or a parallel prompt architecture.

## D — Markdown title/body mechanisms: accepted later, task H1

Requirement: select the intended first nonempty level1 title in the first50 saved
lines outside supported leading frontmatter, with filename fallback and unmanaged
notes supported. `library.rs::title` currently checks literal `# ` lines and scans
only `---` delimiters. It does not distinguish fenced code. Reuse the already
pinned Markdown1.0.0 AST and Store's public `note_identity::body_start`, already
used by saved-link/evidence paths. No new dependency is necessary.

The later change must preserve the policy rather than automatically accepting all
CommonMark headings: explicitly decide/test ATX vs setext, raw inline text, BOM/CRLF,
`---`/`...`, malformed frontmatter,50-line boundary and parser/refusal/fallback for
unmanaged notes. Store's body reader checks managed layouts and the1MiB bound;
do not silently propagate stricter eligibility into previously indexable notes.
AST positions and the exact original body stay authoritative; no note bytes change.

## E — DOCX reader: unresolved, bounded task H4

Requirement: replace meaningful custom OOXML interpretation while preserving complete
meaningful content or explicit refusal, bounds, originals and exact assets.
[docx-rs0.4.22](https://docs.rs/docx-rs/0.4.22/docx_rs/fn.read_docx.html) provides a
published `read_docx(&[u8]) -> Result<Docx, ReaderError>` reader; it is not installed
or locked here. Reader structure alone does not establish completeness or resource
safety. Compare its read paths and retained representation against current synthetic
supported/refused fixtures before expanding Office support. BRN package caps,
provenance and recoverable proposal/application remain necessary even if adopted.
The official reader source at inspected `main` commit
`4ff72dc8ba5f33ca40c121f30664d12e4fe6cdb2` supplies concrete concerns:
[document.rs](https://github.com/bokuweb/docx-rs/blob/4ff72dc8ba5f33ca40c121f30664d12e4fe6cdb2/docx-core/src/reader/document.rs)
leaves unmatched elements without refusal and tolerates some nested parse failures;
[read_zip.rs](https://github.com/bokuweb/docx-rs/blob/4ff72dc8ba5f33ca40c121f30664d12e4fe6cdb2/docx-core/src/reader/read_zip.rs)
allocates from advertised sizes without BRN caps;
[read_docx.rs](https://github.com/bokuweb/docx-rs/blob/4ff72dc8ba5f33ca40c121f30664d12e4fe6cdb2/docx-core/src/reader/read_docx.rs)
can omit failed media/header/footer reads. Disabling previews exposes original
image data and relationship/path, a promising asset seam. These are source
observations, not executed loss witnesses or release-qualified0.4.22 conclusions:
release-source retrieval failed. The three inspected files were then fetched at the
immutable main revision and their hashes retained; H4 still must qualify the
published0.4.22 release itself. Determine whether a small bounded admission/mapping adapter covers these
gaps; stop on irrecoverable silent loss, inaccessible exact assets, unbounded
allocation or the need for a second full interpreter. No replacement is integrated.

## F — streaming UTF-8: keep existing for this checkpoint

Requirement: validate inflated PNG metadata without retaining the complete decoded
text, including UTF-8 scalars split between buffers. `docx/image.rs::Utf8Check` is
a small state machine with explicit overlong/surrogate/out-of-range rejection;
flate2 already performs decompression. A `std::str::from_utf8` adapter needs a
carry of up to3 trailing bytes plus distinction between invalid and incomplete
input, and a terminal-incomplete check. Its total simplicity advantage has not been
demonstrated. Retain the qualified code now; reconsider only when naturally touched
with every scalar split and malformed sequence tests. It is not a priority V1 task.

## Consequences and next action

Existing completed tests/reviews remain useful; no reuse refactor was performed
during closeout. These decisions reduce future ad-hoc implementation without
changing the frozen delivery sequence. [Handoff tasks](../../work/active/v1-handoff.md#ordered-task-queue)
bound the unresolved questions and accepted maintenance work. The owner must select
a task before continuation; native/live/owner qualification remains separate.
