# Kiro Opus 5.5 code review, X2 first HTML helper slice, round 1

Verdict: **CHANGES_REQUIRED** for the first HTML component at `bbf107e3`.

The design is sound and the slice stays within its allocation. The required changes are three bounded code/test fixes in `documents.rs` and `documents.rs` tests, plus narrow corrections to the evidence records. No scope change and no new requirement is involved.

I worked read-only. I ran no Cargo, build, test, fetch, network or GUI step and delegated nothing.

## Binding

- I confirmed the worktree is clean at `bbf107e3`. Tree `df3887f1`, crates `f230d3a2`, one commit on base `781c34dc`, 77 paths.
- All nine source hashes in `check-index.json`, including lock `8b7a8ac0…b8dc`, match the committed files byte for byte.
- The diff touches only allocated paths: brn-intake manifest/helper/documents/lib/tests/fixture, the lock and X2 docs. Host, core, AI, CLI, prepared HTML and CI have no hunks. Lock identities: 965 → 1011, with exactly 52 added and the 6 expected/approved removals.

## Blocking (in priority order)

**B1. The link integrity check is quadratic.** In `documents.rs` `integrity()`, each admitted href runs `plain.contains(href)` over the whole output.
- Admission allows up to `MAX_LINKS` = 4096 links (64 KiB of attributes) after about 8 MiB of text. Worst case is about 4096 × 8 MiB ≈ 3.4e10 byte comparisons.
- On aarch64, `str::contains` uses the Two-Way searcher with no SIMD path. My unmeasured estimate is 10–30 s, which can hit `RLIMIT_CPU` 10 and kill the helper with SIGXCPU on an input that passed every quota. No quota bounds the product links × output.
- Minimal fix: search hrefs with a monotonic cursor in source order, as the word loop already does (links render in order), or otherwise make total work O(output). Add a test with many links placed after a large text block.

**B2. Admitted text and code containing a backslash followed by ASCII punctuation are falsely refused.** `unescape()` drops every `\` before punctuation, but only on the Markdown side.
- Text: `a\*b` becomes `a\\\*b` after upstream `escape_misc` (`text.rs` `escape_into` escapes `\`), then unescapes to `a*b`. The source word is still `a\*b`, so the check refuses.
- Code: fenced/inline code is not escaped, so `printf("a\"b")` unescapes to `"a"b"` and also refuses. Regex text, `\\` and `\'` hit the same path.
- The refusal happens after a full backend run. It fails closed, but it rejects constructs the profile admits, and code is in the profile.
- Minimal fix: apply one symmetric normalization to both the admitted words and the Markdown, or unescape in pairs with awareness of code spans. Add exact tests for text `C:\x\*y`, code `"a\"b\\n"` and `\.`.

**B3. The astral-tl differential coverage is below the allocated Q1 criterion.** `differential_astral_tl_structure_equals_admitted_structure` compares only 4 admitted documents, and only tag names, depth and element counts.
- It does not compare text nodes or attributes.
- It does not cover each allowed parent/child pair in `ELEMENTS`, each void, each entity form, `<a>` without href, href with `&` or parentheses, whitespace-only text in `html`/`ul`/`ol`, CRLF, or a void at depth 32.
- The refused-divergence witnesses cover repeated `p`, `< img` and `</DIV>`. There is no tl witness for repeated `li` or a raw-text element.
- Minimal fix: a deterministic enumerated corpus over the grammar that compares tl's tag sequence, parent relation and raw text spans with the scan tokens, and runs the real Xberg call plus `integrity` on each case. Add the two missing divergence witnesses. A generic fuzzer is not required.

**B4. Required evidence corrections (documentation only).**
- **a. The association statement is too broad.** The `check-index` claim "Source bytes listed were the compiled inputs of every Cargo command… no source edit during or after them" is false for `helper-build-1`, whose log shows the `lt_count` dead-field warning on pre-fix bytes, and for the pre-escape runtime/test run.
  - Narrow it: `helper-build-final`, `documents-tests-after-escape-pin` and every later command ran on the listed hashes. Mark the earlier runs as compiled from earlier sources whose hashes were not recorded.
  - The final checks stay valid, because the recorded hashes equal `bbf107e3`.
- **b. The pre-escape failure was not retained.** The run where `*star*` rendered as emphasis has no raw log, in the repo or in `/tmp/brn-x2-html-20261017/logs`; `attempts.log` lists only the build warning. Record it as an unretained attempt. Do not reconstruct it.
- **c. The build targets are gone.** `check-index` still names `target/` and `target-before/`, but the scratch directory holds neither. The helper, `.d` and `brn`/desktop binaries now survive only as hashes. State this, and do not rebuild to recreate them.
- **d. Some runtime evidence is prose only.** The SIGTERM-cancel check has no raw log. `sandbox-denies.log` exists only in `/tmp` scratch. Commit it or mark it unretained.
- **e. The tier-routing claim is incomplete.** Both the module doc and the report say tier 1 is unreachable, so tier 2 always runs. In fact `convert_inner` takes `fast_text_only` for any input without `<` before tier 2 (`html-to-markdown-rs` `convert_api.rs:696`), with no document structure. The output is still integrity-checked. Correct the claim and add a backend positive test for tagless input.
- **f. Record blake3's native code.** blake3 1.8.7's build script compiles NEON C on aarch64 (asm/AVX-512 on x86_64) through `cc` into the helper. It is reachable from Xberg `types/internal.rs`. Record it as emitted native code. Remove or explain the stray empty `diff-.txt.gz`.

## Axis findings

**Q1: admission (`html_admission.rs`)**
- The grammar is sound against the WHATWG tree-builder repairs I checked:
  - `p` closed by a block start in button scope;
  - heading inside heading;
  - the `li` scope algorithm;
  - nested `a` and adoption-agency misnests (only exact pop-matching closes are accepted);
  - the `</br>` remap;
  - dropping the newline after `<pre>`;
  - whitespace before `<html>`;
  - text beside a root `html`.
- The node bound holds: elements ≤ `lt` and text runs ≤ `lt + 1`. It is checked before any tag is interpreted.
- Scanning is linear. Decoding never expands bytes, so the decoded text quota is applied per run and needs no allocation beyond the input size.
- Numeric entities refuse controls, surrogates and noncharacters. A lone CR is refused and CRLF is folded, matching the tokenizer.
- html5ever corroboration is real: any parse error, comment, doctype or self-closing tag is a disagreement, and a limit bounds its memory.
- **A1.** `<html><body>a</body><body>b</body></html>` is admitted. The WHATWG builder merges the two bodies, tl builds two. Content survives. The fix is one line (allow at most one `body`).

**Q2: config, warnings and literal escaping**
- I confirmed upstream source facts:
  - `include_document_structure` forces Tier2 in `router.rs`;
  - `convert_single_pass` drops nested html-to-markdown warnings;
  - `needs_image_data()` is false, and the test asserts it;
  - a NUL refusal makes the UTF-16 detection path unreachable;
  - with head, title and script refused, the head and script repairs are unreachable;
  - preprocessing is off;
  - inputs ≥ 512 KiB run on a 16 MiB thread that is joined inside the poll, so the future never pends;
  - the Rayon global pool is initialized once with 1 thread.
- Upstream `escape_misc` covers `\ & < > [ ] ~ # = + | -` and digit `.`/`)`, which defuses Markdown/HTML injection from text. Code fences use the longest backtick run + 1. Link destinations balance parentheses.
- **A2.** Word integrity matches substrings, not word boundaries. A dropped short word can be masked by a later word or URL (`a` inside `banana`), and href checks are not tied to position. This is acceptable only because it is labeled a truncation defense, not a fidelity proof.
- **A6.** No tests cover literal block-start text (`# `, `- `, `1. `, `> `, ``` ``` ``` inside a paragraph), backtick runs in code, `<a>` without href, `mailto:`/`#` outputs, or NBSP.

**Axis 3: protocol and DTO**
- The `"html" if source_base == 0` guard is redundant but correct: EML only nests docx/pptx.
- `HTML_CONVERTER` is distinct, and the host compares against `CONVERTER` exactly (`threads-intake lib.rs:279`). `convert(Html)` is still `Unavailable` (`:213`).
- `sources[0].text = markdown` satisfies `validate`. Error labels are static. Usage finalization reuses the existing code.
- **A4.** `expanded_bytes` counts decoded source text, while DOCX counts package bytes. The output limit applies only after conversion (the `append` 8 MiB check and the protocol budget), and upstream output can reach about 2× before that refusal. It fails closed, but the claim of output quotas before expensive work is overstated.
- **A3.** The check `elements > lt_count` is a tautology, added to silence the dead-field warning, and its refusal is mislabeled CORROBORATION. Remove it or make the field test-only.

**Axis 4: executor and restriction**
- `block_on` is bounded: 16 polls with `park_timeout(1 s)`. **A5:** the Pending path has no test.
- `restrictions()` is unchanged and runs before stdin is read; `sandbox_init` is process-wide and irreversible. The existing `--self-test-restrictions` canary (`file_denied`/`network_denied`) shows that same binary is restricted before it reads input. Together with the env-cleared HTML tests running on that binary, that is enough gating evidence for this engineering component.
- The gap is acknowledged: the sandbox-log query was silent for the positive control too, so the logs prove nothing either way. Nothing in this branch witnesses an attempted access during HTML conversion.
- The profile allows `process-fork`, but exec is denied by default. I found no `Command` or `fork` on the Xberg or html-to-markdown HTML paths.
- The RSS, latency and thread figures are measurements, not caps.

**Axis 5: graph and shared serialization**
- The 52 added identities and the 6 replacements match the claims.
- I independently confirmed that serde_spanned, toml_datetime and toml_writer have identical `src/` trees; their manifests differ only in version, include list and lints. So the empty `.gz` diffs are genuine src-only diffs, not missing evidence.
- serde_json 1.0.151 → 1.0.154 changes only `de.rs`, `lexical/math.rs`, `lib.rs`, `map.rs` (hasher type) and `number.rs`/`read.rs`; neither `ser.rs` nor `value/ser.rs` is in the diff. The helper builds `json!` maps only for the error object and self-test output.
- The default workspace already had `preserve_order`.
- No precise historical byte witness is needed beyond the existing persistence, replay and hash tests.
- The tree confirms xberg `[html]` with no Tokio, and image gif/bmp/webp compiled into the helper but unreachable (all image syntax refused before backend).

**Axis 6: verification evidence**
- The raw logs agree with the reported counts:
  - helper all-targets: 83 passed, 0 failed (42/0/17/12/6/6);
  - threads-intake: 35/1/3;
  - workspace run 1: 1252/7/7, six `SymlinkRefused` failures plus the PDF case;
  - workspace run 2: 1258/1/7, only `Pdf preparation failed: Unavailable` at `qualification.rs:470`;
  - fixture conversion: real Xberg Markdown.
- In the carried failing test, PDF is the last fixture, so the prepared HTML fixture ran and passed first.
- Workspace runs do not include the helper-feature tests; those come from the helper lane. The failed ORT run-1 lanes are retained.

**Axis 7: NF-6 link fix**
- All 82 absolute link targets are preserved as inline code, 0 remain as links, and every changed line is a link-bearing line.
- The checker reports 0 failures afterwards. CI and the link checker are unchanged.
- The immutable `evidence-hashes.json` ledger is left alone, with its dated association kept.

## What the checks prove, and what they don't

They prove:
- The real restricted helper, under the unchanged sandbox and rlimits, converts the tested textual subset through real `xberg::extract`.
- Refusals happen before the backend, with backend-call counts at depths 31/32 versus 33/127/128/129.
- The before-state was a genuine `unsupported intake kind`.
- The shared lock passes the macOS workspace, capability, native and ORT-cached lanes.

They do not prove:
- full HTML fidelity;
- that Xberg made no syscall attempts;
- hard RSS or thread caps;
- Linux or Windows behavior;
- the preserved build artifacts.

## Shared lock

I accept the six dispositioned moves within this candidate. Remaining qualifications:
- Linux and Windows CI. On Windows, embed-resource now resolves toml 1.1.8, which was not run locally.
- CI `cargo fetch` of the 52 new crates.
- The macos-15 helper lane running the new HTML tests.
- The Poppler-dependent PDF test in CI.

## Scope

- This is a helper-protocol engineering component only. User-facing HTML is unchanged, and host `convert(Html)` is still `Unavailable`.
- The PDF resource STOP, Office/XLSX/MIME, helper-local Tokio, the rich-HTML cutover, owner all-document adoption and final integration are all still pending.
- The owner's all-document objective stands.
- Publish and merge nothing early. This review grants no source or owner acceptance.
- The parent owns the corrections and the next fresh review.
