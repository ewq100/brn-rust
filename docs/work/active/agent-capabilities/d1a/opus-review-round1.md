**Verdict: CHANGES_REQUIRED at `202bf2d7`.** I found one High and two Medium defects, plus two Low notes. I reviewed the fixed refs only, made no edits and ran no Cargo commands. The source, whole and crates tree hashes match the ones you gave. The working tree is not clean: it has four untracked `d1a/*-build.{json,log}` files.

## Findings

**1. HIGH: The new 8192-token output cap is actually sent to Copilot, where it was never sent before.**
- **Where:** `threads.rs:3278` and `threads_research.rs:423`, both calling `.max_tokens(context_budget::RESPONSE)`.
- **Evidence:**
  - Rig drops `max_output_tokens` only for the Codex contract (`rig-core-0.44.0/src/providers/openai/responses_api/mod.rs:509-513`), and only ChatGPT uses that contract (`chatgpt/mod.rs:69`).
  - On Copilot, Responses requests now carry `max_output_tokens: 8192`, which counts reasoning tokens. Chat requests carry `max_tokens` or `max_completion_tokens` (`openai/wire.rs` `OutputCap`).
  - `chat.rs:179-187` fails a run on any finish other than Stop, including `length` and `incomplete:max_output_tokens`.
  - Before this change, Copilot requests had no output cap. The README describes 8192 as a "response allowance", not a cap sent to the provider.
  - No test catches this, because every wire-level test uses the ChatGPT dialect, which removes the field.
- **Fix:** Remove both calls and keep `RESPONSE` only as space reserved in the estimate. Add captured-HTTP Copilot tests for both Responses and Chat that assert no output-cap field is sent.

**2. MEDIUM: The cap for models with an unknown limit is smaller than requests BRN already allows.**
- **Where:** `threads_context_budget.rs:11` and `:14`.
- **Evidence:**
  - `UNKNOWN_REQUEST_CAP` is 128 KiB. That is smaller than `RESULT` (132 KiB) and smaller than a valid `read_note` with `max_bytes` 131072 (`threads.rs:438`).
  - It also cannot hold a near-maximum prompt that `prepare_run` accepts (128 KiB, `threads.rs:2336`), once fixed overhead and the 12 KiB reserve are added. The fixed overhead includes BRN.md (about 12.7 KB) and the tool catalog.
  - `for_scope` treats every Copilot run as unknown. So every maximum-size read, and every accepted prompt above roughly 80–90 KiB, now fails on Copilot. These requests were sent to the provider before this change.
  - The failure then tells the user "The context limit was reached", although no model limit is known or reached.
- **Fix:** Set the unknown cap so it fits the fixed overhead, the maximum accepted prompt, one maximum admitted result, the history window and the reserve. Alternatively, clamp result admission to the remaining budget. Make the wording for this refusal say a BRN request-size estimate was exceeded and the model limit is unknown. Add a Copilot test where a maximum-size read completes.

**3. MEDIUM: Copilot's own context-overflow error is not treated as a context limit.**
- **Where:** `error.rs` `is_context_limit`.
- **Evidence:**
  - Copilot reports overflow as `model_max_prompt_tokens_exceeded` (public example: github.com/orgs/community/discussions/167397). `provider_code` already keeps this code.
  - On Copilot, the provider's error is the only real signal of overflow. Today it shows as "The operation did not complete.", and the helper outcome is `failed` instead of `budget`.
- **Fix:** Recognise this code too, keeping the error kind as `Other`. Add parent and helper tests that use a Copilot 400 response.

**4. LOW: The model-limit fixture leaves out two listed models without explanation.**
- I confirmed the fixture's source hash (`943ca7fe…`) and its six entries against the pinned public catalog.
- That catalog also lists `gpt-6-astra` and `gpt-5.6-terra` with `context_window` 272000. Both fall back to the stricter unknown cap.
- **Fix:** Add both models or document how entries were selected.

**5. LOW (not blocking): Building history does unbounded work before the cut.**
- `history_input` scans every prior Run's operation writes and runs `records.iter().any(record_in_chat…)` for each, across the whole chat history.
- **Fix:** Build an index from record ID to record once.

## Checked and correct
- **Core fence (`830b5e3f`):** It applies only to Runs that were not already archived and are not Working. Working Runs still advance exactly once, in the existing live loop, which reuses the archived write. Restore matches this. The Store guard is unchanged and overflow refusal is atomic. The tests are direct and meaningful.
- **History selection:** `TokenWindowMemory` is applied explicitly once. It keeps the newest whole chunks, and the chunk boundaries do not move as history grows. Coverage metadata and the mandatory input come after the raw history. The prefix test checks actual HTTP bytes.
- **Within a run:** `without_memory`, no trimming, a stop before any over-limit dispatch, and refusal of oversized results.
- **`read_message`:** It uses the same core visibility check as `read_records`. The owner/assistant role whitelist already excludes all reserved writing roles. It requires the exact current version and valid character boundaries, and it refuses inside helper and writing scopes.
- **Mandatory content:** The writing v2 snapshot stays intact. An oversized request fails before any provider or credential HTTP request. ChatGPT's `context_length_exceeded` gets the same specific outcome, with safe diagnostics.
- **Helper route:** The helper uses the budget for its own chosen route.
- **Dependency:** The `rig-memory` addition is minimal and needed.
- **Old instructions:** Text from earlier discussion can't forge new instructions and can't apply protected edits.

## Residual
The ChatGPT byte estimate against the 272000-token figure is roughly 3–4 times conservative. It is labelled as an estimate, so I'm not counting it as a defect. Mutation and normal runs are still pending. The cache-share figures are synthetic only. Nothing here amounts to live, native or owner acceptance.
