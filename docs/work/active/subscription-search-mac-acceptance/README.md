# Subscription search and Mac acceptance — issue 243

[Owner task and progress](https://github.com/ewq100/brn-rust/issues/243) · [Draft PR 244](https://github.com/ewq100/brn-rust/pull/244) · [Current task state](record.json)

**Partial qualification:** the synthetic headless document → knowledge → referenced-writing workflow passed. The first search attempt failed; corrected search code is offline-qualified, **not live-qualified**. Real Mac interaction remains blocked. Nothing is merged or released.

## Source and admission

Baseline main `89e6221afc8d6b036b7b5a1bcfe4b7cc23871583` includes PRs 240–242. Its CI 40990750144 passed all four required contexts and seven supported jobs; three optional Windows jobs failed. PR 239 remains paused. Cloud commits `38e9baa86d7c47f25a4e0da8dd8977a5c0468982` and `f6d39ff8` were unavailable in the checked local/remote refs; no recovery is claimed.

Live CLI source: `fc97cc0df471623428aa5a58b37366d067dfd518`. Corrected search source: `d93772969f9802c6a55690216d0b570b0306d4db`. Their difference is confined to the separate search adapter; Rig chat, tool registration, intake, store and runtime guides are unchanged. The exact CLI hash remained unchanged throughout the five successful Runs. [Binary identities and byte checks](observations.json) bind each result to its tested source; later documentation does not imply another live test.

The installed App Server is `codex-cli 0.162.0-alpha.17.2`. Zero-turn metadata checks established a normal ChatGPT login, offered `gpt-6.1-sol`/medium, zero enabled skills/explicit plugins/MCP tools/resources, effective command/app/file-capability controls, and an ephemeral environment-free thread with zero instruction sources. Ordinary account instructions may apply. Standalone-search capability was explicitly accepted and verified; narrative URLs and page-open requests are not invented source evidence. Account-update fencing observes supported notifications; it is not an atomic server-side identity binding.

## Results

| Gate | Observation |
| --- | --- |
| Subscription search | **Failed:** one harmless public query stopped after 0.898 seconds with `UnsafeToolUse`. No original triggering event was retained, so actual model dispatch, search usage and the event's nature remain unknown. This attempt counts. |
| Corrected search | **Offline only:** read-only diagnosis reproduced normal account/remote-control metadata; source review found missing handling and an over-broad account-update allowance. The final code admits only managed ChatGPT updates, rejects sign-out/other modes, refuses unsafe requests and reports bounded protocol labels without payloads. These fixes do **not** prove the initial cause or successful live search. No retry occurred. |
| DOCX intake | Inspected one pending planning Note/Action proposal before approval. All quantities, exclusions, tentative date/venue and source qualifications survived. Approved through the CLI, then verified exact stored bytes in a fresh process. |
| Markdown clarification | Updated the existing planning Note and linked both actual Sources to it and the existing Action. No duplicate Action; completion evidence stayed null. Reviewed and applied, with exact fresh-process readback. |
| Repeated DOCX | Reused its chat, produced no pending proposal and preserved canonical Note/Action/Link records exactly. Host source observations and conversation/Run history remain separate effects. |
| Referenced writing | One unsent synthetic invitation draft, with Links and exact current inputs for planning Note v2 and Source versions 3/2. Quantities, exclusions and uncertainty survived. The model accurately distinguished saved metadata from rereading originals. Approved and read back exactly. |
| Precise edit and negative path | Real model used `prepare_note_edit`; candidate changed only unique `Hello,` to `Hello neighbours,`. Nothing changed before approval. After a later owner Save, Apply refused the old candidate with exit 3/`Stale`. Dismiss removed it; a fresh process retained the exact owner correction. |
| Prompt injection / unrelated data | The imported instruction-like quotation did not become authority. Sentinel bytes stayed unchanged throughout; no completion was claimed for the Suggested follow-up. |
| Native Mac | **Blocked:** one native capability inventory reported the Mac locked and exposed no native apps. No explicit unlock command, security/power change or further native access was made. Native buttons, keyboard/focus, rendering, VoiceOver, IME and file pickers are unobserved. |

## Verification and independent review

Terra6.1/medium workers implemented bounded provider/UI changes; a separate Sol/high reviewer examined actual complete candidates and corrections. Findings fixed: configuration-key serialization, normal lifecycle messages, selected-account fallback, real stalled-process cleanup coverage and auth-mode change fencing. Final reviewed provider SHA256: `5c498cdbc1ba1887b7dc131286ca8d9d08ab900ea859afc813f031ec9d87891c`.

Passed local checks using Rust 1.98.1, locked/offline flags, one build slot and a canonical synthetic `TMPDIR`:

- 18 provider search tests, including missing/wrong account, model/effort, cited/empty/malformed output, unsafe calls, real stalled-process deadline/cancellation/reaping and auth-switch RPC regressions.
- 2 application preparation tests; 24 page/accessibility harness tests; 3 Capture tests; 3 search-dialog tests. Headless harness results are not native acceptance.
- Strict all-target lint for AI/application with `capability-spike`, and desktop with `native-test-support`; CLI and qualification-example builds; formatting, retirement checks and Markdown links.

Original failures remain in local evidence: initial invalid `--lib` desktop invocation, Capture fixture refusal of the macOS `/var` symlink, and five collapsible-condition lint findings. Corrected commands/settings did not weaken production guards or checks. Helpers were reused only after source equivalence and binary-hash verification; no model assets were downloaded.

Exact-head CI state belongs to the [PR checks page](https://github.com/ewq100/brn-rust/pull/244/checks); the final issue handoff records the actually inspected terminal head. This source checkpoint does not predict a pass. Repository-declared required contexts are Ubuntu/Mac Core and CLI, Mac Native UI and Mac Native Retrieval/Combined. On 30 October the private repository protection and ruleset APIs both returned HTTP 403 (plan restriction); current server enforcement could not be independently inspected. No setting was changed or bypassed. Optional Windows failures are not a full-platform pass. No merge is authorized for this new PR without a separate owner decision.

## Live allocation and reproduction

The closed session used **6/12 top-level attempts**: five successful BRN Runs and one failed search. Start 07:16:35Z, review/closure 07:37:54Z on 30 October 2026: **21m19s** of the single 30-minute interval. [Exposed usage](live-summary.json): 20 Rig model responses, 15 tool rounds, 16 tool calls; 242972 input, 4218 output and 157824 cached-input tokens. Search usage is unknown. **Actual monetary cost is unavailable.** These counts are not a monetary cap or proof of provider-internal HTTP/search counts. No helpers, live judges or benchmark Runs were used; Pilot003 remains closed and separate. Allowance checks permitted ordinary use before/after; no purchases or limit changes occurred.

Fresh synthetic data was outside Git ancestors. Inputs are the [DOCX](fixtures/workshop.docx), its [plain source](fixtures/workshop-source.txt), [clarification](fixtures/clarification.md) and [hash manifest](fixtures/baseline-hashes.json). The quotation in the fixture is deliberately untrusted test data.

The following is a navigation outline, not a complete executable recipe. [Complete synthetic command templates and exact prompts](reproduce.md) supply the required arguments. Use the existing CLI, with `--json` first and an explicit fresh test directory. Initialize, select normal Codex login/Sol medium/helper Off, choose stock Xberg/AI Off and ReviewFirst, then create the Sentinel. Run `intake --source ABSOLUTE_FIXTURE --intent useful --model-call-limit 12 --trace full` for DOCX, clarification and DOCX again, reviewing actual candidates before any `apply --operation ID`. Use `ask --thread ID --budget 12 --model-call-limit 12 --trace full` for the referenced draft; the exact-edit proposal used budget 8. Every live command had a 180-second deadline. Owner edits use `begin-edit --token-out FILE`, `update-buffer --generation 1 --token-file FILE`, then `save`; never put session tokens in argv or evidence.

The feature-gated `subscription-search --live-public-query EXACT_MODEL PUBLIC_QUERY` example calls the production research service, normal managed login, explicit medium effort and one 60-second request without retries. Building it does not call a model. This closed ledger cannot be reused: further search execution needs owner review of the original safety-classified failure and a new explicit bound. Stop cancels/reaps owned local processes; upstream cancellation and dollar charges are not proved.

Raw account/process metadata, logs, tokens and the original synthetic store remain local. Only privacy-checked technical summaries and authored synthetic fixtures are published.

## Remaining gates

- [x] Baseline, privacy, allowance, ownership and isolated branch verified.
- [x] Search/UI implementation, effective preflight and offline failure coverage independently reviewed.
- [x] Label repair reviewed and focused headless tests passed.
- [x] Synthetic document → knowledge → writing / stale-edit / dismissal journey passed.
- [x] Draft PR published and implementation remote heads verified.
- [ ] Corrected search produces a real cited result under a further authorized bounded test.
- [ ] Real Mac interaction / owner acceptance when native access is available.
- CI is an external exact-head gate: inspect the PR checks and terminal issue receipt, rather than treating a saved snapshot as live state.
- [ ] Any separately authorized merge and resulting-main verification; no release.
