# X2 first HTML slice: implementation report (Kiro Opus 5.5)

17–18 October 2026. Branch `work/x2-html-20261017`, created with `git switch -c` at `781c34dc466d6a6a210cdae4cf75e1096521c70d` in worktree `t3-ad771af6`. It was clean at start, and the crates tree equaled production `bdc0a22a`.

This is a candidate for a helper-protocol capability only. It is not user-facing HTML, it does not adopt Xberg for all documents, it has had no independent review and it has not been merged. Xberg is not done.

## Status

The first gate run stopped compilation correctly. Four toml-family companion moves were outside the expected set. That record is preserved in [graph-gate.json](graph-gate.json). The principal then approved exactly those four moves on 18 October as mandatory closure of toml 1.1.8. All other existing-version movement stays a STOP.

After that approval:
- The helper built first.
- All brn-intake helper targets pass.
- The CI-equivalent workspace, capability, native-UI and native-retrieval lanes ran. The only unexplained-free failure left is the carried `Pdf preparation failed: Unavailable` (local Poppler `pdftoppm`/`pdfinfo` absent).

Every command, exit code, test count, log hash and source/binary hash is in [check-index.json](check-index.json). Raw logs are in [logs/](logs/).

## Graph

- **Metadata resolution.** One offline `cargo metadata` run after the manifest hunk. Lock `65d2aeee…1a1c` (965 identities) became `8b7a8ac0…b8dc` (1011): 52 added, 6 removed. Every added identity equals the source1b ledger and its cache archive. The approved moves are serde_json 1.0.154, toml 1.1.8, serde_spanned 1.1.2, toml_datetime 1.1.2, toml_parser 1.1.5 and toml_writer 1.1.3.
- **Actual helper graph** (`cargo tree --features helper` with features; [log](logs/helper-tree.txt.gz)):
  - xberg `[html]` only, with no Tokio;
  - serde_json with `preserve_order`;
  - image `[bmp,gif,jpeg,png,webp]`, which adds gif, bmp and webp decode surface (BRN asset admission stays PNG/JPEG, and the profile refuses all image syntax);
  - toml 1.1.8.
- **Default workspace graph.** It already had serde_json `preserve_order` through rig-core. toml 1.1.8, serde_spanned and toml_writer are absent from the macOS default graph. toml_parser and toml_datetime appear only at build time, through `toml_edit` → `proc-macro-crate` → `rig-derive`.
- **Build scripts.** The 8 added build scripts hash-equal the source1b inventory.

## Code

| File | Role |
|---|---|
| `src/helper/documents/html_admission.rs` | Std-only byte-level agreement language `html-textual-1` (details below). |
| `src/helper/documents.rs` | html5ever 0.27 token-sequence corroboration, one pinned `xberg::extract` per request on a private park/unpark executor, result checks, ordered word/link integrity, schema-1 projection. |
| `src/helper.rs` | Standalone `html` arm (`source_base == 0` only; nested EML unchanged), with the existing finalization and usage accounting. |
| `src/lib.rs` | `HTML_CONVERTER`; the existing `CONVERTER` is unchanged. |

Admission covers:
- lowercase allowlisted tags immediately after `<` and exact `</name>` closes;
- explicit closes and restricted content models;
- only an exact `href` on `a`, limited to http(s), mailto or `#`;
- six named entities plus validated numeric references;
- refusal of comments, declarations, raw-text/head elements, resources, tables and data URIs;
- size of 8 MiB or less, then NUL and strict UTF-8 checks, then `2*count('<')+1 <= 200000`, then depth of 32 or less, then text/entity/attribute/link quotas.

Xberg configuration pinned as granted:
- extraction: no timeout, cache off, quality off, max concurrent 1, no content filter, Markdown output, `ConcurrencyConfig.max_threads=Some(1)`;
- `html_options`: preprocessing off, 8 MiB input limit, `max_depth` 128, document structure on;
- additionally: `strict_parse=true` (no plain-text recovery), post-processing/OCR/images/captioning/QR off, file inputs and URIs off, metadata off.

**Correction found at runtime.** The actual helper rendered the literal source text `*star*` as Markdown emphasis. I pinned upstream `escape_asterisks`, `escape_underscores` and `escape_misc`, and added an exact entity/literal-text test.

**What the integrity check proves.** The every-word subsequence check is a defense against truncation and content dropping. It is not a full fidelity proof, so the exact ordered fact, entity and link tests are kept.

## Upstream warnings that cannot be observed

Xberg drops nested html-to-markdown warnings, so checking top-level `processing_warnings` does not cover them. Each warning class is made unreachable by admission and config instead:

| Warning class | Why it cannot fire |
|---|---|
| Depth | Admitted depth is at most 32; the astral-tl differential shows actual DOM depth equals scan depth, and depth 33/127/128/129 refuses before the backend. Upstream `max_depth` is 128. |
| Head repair / `TruncatedInput` | Head, title, meta and raw-text are refused. |
| Lossy decode | Strict UTF-8 check runs first. |
| Image / second pass | No image syntax; images, captioning and OCR are off. |
| MathML / table-caption raw-HTML recovery | `math`, `table`, `caption` and `img` are refused. |
| Preprocessing removal | Preprocessing is off. |
| Tier-1 path | Forced document structure makes the router always select tier 2. |

## Checks

| Check | Result |
|---|---|
| Genuine before, actual old helper | exit 1, `unsupported intake kind` |
| Std-only scanner (pre-grant `rustc --test`) | 5/5 pass; first attempt had a wrong expected count, a test error |
| Helper build | exit 0; first build had 1 dead-field warning, fixed |
| `cargo fmt --check`, intake strict Clippy | pass |
| brn-intake helper all targets | 83 pass, 0 fail (lib 42, includes 15 X2 HTML tests, real Xberg, astral-tl differential, 31/32 vs 33/127/128/129 backend-count tests) |
| threads-intake all targets (prepared rich HTML regressions) | 35 pass, 1 fail: carried PDF `Unavailable` |
| Retirement checkers, script tests | pass |
| Workspace build, workspace strict all-target Clippy | pass |
| Workspace tests, run 1 | 1252/7/7. Six `SymlinkRefused` failures because my TMPDIR was under the `/tmp` symlink, plus carried PDF. Retained as a failed run. |
| Workspace tests, run 2 (TMPDIR under `/private/tmp`) | 1258/1/7; only carried PDF |
| Capability tests / Clippy | 469/0/1, pass |
| CLI help, e2e fixtures, desktop headless | pass |
| Native UI build/Clippy/tests; native widgets Clippy/tests | pass; 95/0/0; 312/0/1 (no claim about the historical 314/1/1 cause) |
| Native retrieval and combined lanes, run 1 | ORT link failure: `CARGO_NET_OFFLINE` without explicit `ORT_LIB_PATH`. Retained. |
| Same lanes, previously qualified cached ORT (`d68759bf…38b3`), no network | retrieval 35/0, app 248/0/1, CLI and combined build/Clippy pass, combined tests 95/0 |
| Restricted helper runtime | fixture 0.01 s, max RSS 12.9 MB; 6.0 MiB joined-thread input 0.37 s, max RSS 218 MB, max sampled threads 3; process-group SIGTERM mid-conversion reaped |
| Sandbox-log canary | No evidence either way: the positive control also logged nothing. No-file/network/child for this branch is source/config-based plus the unchanged pre-input restriction. |

## Serialization compatibility

serde_json 1.0.151 → 1.0.154 source diff:
- map hasher type change (insertion order unchanged);
- `as_f32` non-finite filter;
- `-0` parse change only under `arbitrary_precision`, which is off;
- error positions;
- no serializer output change found.

BRN does not use `as_f32` or `arbitrary_precision` and has no TOML call sites. The existing workspace persistence/replay/backup/hash tests pass (241 by name in run 2), as does the historical `Extraction` byte/hash round-trip. This is a static diff plus existing tests, not a byte comparison of every saved record type. I rewrote no saved record or receipt.

## NF-6 docs correction

I converted 82 machine-local links in `source-audit/api.md` and `resources.md` to inline code, keeping the exact path:line. The full link check went from 82 failures to 0 ([before](linkcheck-before.log), [after](linkcheck-after.log)). Those logs are a historical receipt of the 17 October bytes.

The immutable `source-audit/evidence-hashes.json` is not rewritten. Its `resources.md` hash describes the `cd2da52c` bytes, `137726e1` later changed the file, and this task changed it again (api.md `eb4087f9…`, resources.md `62c204f8…`).

## Limits and next step

Open:
- full rich-HTML fidelity (tables, figures, spans) and prepared-HTML cutover;
- host converter wiring;
- PDF resource STOP;
- Office/XLSX/MIME families, EML through Xberg and helper-local Tokio for Excel/PDF;
- a measured OS thread cap and hard RSS bound (neither is claimed);
- a syscall-level runtime canary for the HTML branch.

Next: a fresh independent Kiro review of the actual diff, then the bounded DOCX/PPTX evidence slice from [source1b/next-allocation.md](../source1b/next-allocation.md).
