# Rig 0.45 migration

Qualified local candidate on `codex/rig045-migration-20261023`. Implementation:
`870bb93640c80a8e81ab4b3d96b7a9cc85767746`; subsequent evidence changes do not alter
product source. Stacked on maintenance `8f7143f34191ad53e4c7e74f7c061e8dce5e6e35`,
with UI `125f8ef053895e9dac1a54ecc3e8243188ccbc6c` preserved. Main was fetched
unchanged at `7b416591`. No push, PR, merge or release has occurred for this stack.
Publication remains unauthorized. No live accounts, credential transfer, new
persistent access, Mac access or native-picker retry occurred.

## Compatibility and implementation

Official release: https://github.com/0xPlaygrounds/rig/releases/tag/v0.45.0
(released 2026-10-19). Its tagged MIGRATING.md stops at 0.44. The current official
guide covers 0.44→0.45; every used API was cross-checked against released crates:
https://github.com/0xPlaygrounds/rig/blob/main/MIGRATING.md

- Rig family 0.44.0 → 0.45.0; its required rmcp/macros 3.5.0 replace 2.2.0.
  Darling 0.24.1 and existing Syn 3 are required by those macros. No unrelated
  version chase: Tokio 1.53.3, Schemars 1.2.3 and both helper graphs are unchanged.
  [Exact lock delta](evidence/lock-delta.json).
- Updated AgentBuilder, Arc-owned messages, RegisteredTool, StreamRecord,
  middleware Exchange and provider construction through official APIs.
  Accepted-final ownership, tool validation, cancellation, correction bounds,
  approval/replay fences and ordered schema behavior remain covered by tests.
- A private Rig TokenStore adapter preserves existing ChatGPT JSON and separate
  Copilot bootstrap/session files. Copilot records combine only in memory.
  Selected Codex tokens use read-only MemoryTokenStore. Existing provider locks,
  checked paths and finalizers remain; auth resolves before constructing clients,
  with no later request-time refresh outside those fences. No new lockfile,
  credential location, canonical DB migration or irreversible data rewrite.
- Recent history uses Rig TokenBudget/view. The removed Compactor trait wrapper
  is retired; BRN retains its existing bounded Rig model call and Semantic/Entry
  DTOs. The new generic Summarizer trims conversational text, which would change
  BRN's exact coverage/source-ID/JSON provenance contract. No alternate model
  runtime or replacement OAuth implementation was introduced.
- MCP uses McpConnection and DynamicTool::run_with. Sessionless/replayed GET,
  continuation fields and task RPCs remain refused. New protocol capabilities
  cannot broaden a frozen approved invocation. Disconnect retires before SDK
  shutdown. New regressions cover those boundaries and credential compatibility,
  partial login, read-only Codex tokens and unsafe-destination zero-write behavior.

## Qualification

Rust 1.98.1, Debian 13.6 x86_64; existing Xvfb :98/Openbox, software Vulkan and
AT-SPI. Serial shared-target builds. [Commands and fixture setup](reproduce.md).
[Source binding](evidence/source.json), [log hashes/counts](evidence/log-receipts.json),
[independent review](evidence/review.json) and [persistence comparison](evidence/persistence-comparison.json).

| Check | Result | Source / limit |
|---|---|---|
| Workspace all-target check, workspace build | PASS | Final build on 870bb936 |
| Workspace tests | 1,313 passed, 0 failed, 5 ignored | 1fc6e45f, before boxed-client layout correction |
| Provider/capability suite | 505 passed, 0 failed, 1 ignored | Repeated on final 870bb936; synthetic only |
| Strict workspace, capability, scripted-document and native Clippy | PASS | Final corrected source, no warning suppression |
| Native state tests | 352 passed, 0 failed, 1 ignored | 870bb936; separate from actual window |
| Production native build | PASS | Binary SHA-256 c9d23831f7df978c24e0a9b32e8da2bab52d0032acf5998dd3a449c69b482306 |
| End-to-end fixture journeys | PASS | 108 steps, 1,266 checks; final binary |
| Python tooling | 191 run, 1 skipped, no failures | Final source |
| Formatting, links, overview consistency, diff | PASS | Final evidence gate |
| Independent source/evidence review | No blocking findings | Exact 1fc6e45f plus boxed-client correction; logs, counts, snapshots and limits checked |
| Real native window | PASS | Old-store comparison, Unicode save v3, menus, quit/reopen/close |
| CI, live accounts, Mac-specific acceptance | NOT RUN | Unpublished branch; outside this local qualification |
| Scripted document journey module | NOT RUN on Linux | macOS-only; feature Clippy passed, not a journey pass |

The full workspace test result is not relabeled as an exact-final-head rerun.
The only later product change boxes the unified Copilot client like ChatGPT;
independent review found it layout-only. Final provider regression, native state,
real-window, rebuilt CLI end-to-end and all strict lint gates cover that correction.

## Actual native and persistence evidence

A preserved Rig 0.44 maintenance CLI opened the synthetic baseline and recorded
[records/history/recovery/candidates](evidence/rig044-snapshot.json). The Rig 0.45
CLI reopened a disposable copy with an exactly equal full snapshot before edits.
The native app edited exact Unicode text, saved version 3, activated a menu through
AT-SPI, exercised repeated Return/Space/Escape and pointer paths, quit, reopened,
matched exact clipboard bytes and the full saved snapshot, then closed normally.
The unrelated sentinel remained byte-for-byte unchanged.

[Saved window](evidence/native-saved.png), [reopened window](evidence/native-reopened.png),
[observed result](evidence/native-result.json), [persisted saved state](evidence/native-saved.json),
[reopened state](evidence/native-reopened.json), [AT-SPI actions](evidence/native-ax-actions.jsonl).
The [exact local harness](evidence/native-smoke.py) and existing cloud driver are
reproducible in this saved environment; use a new fixture root for another run.

Disabled Rewrite exposes no action while dirty, but Linux still incorrectly reports
`enabled`/`sensitive`: [actual AT-SPI observation](evidence/native-disabled-observation.json).
This known baseline caveat remains unpassed. Native picker access was previously
denied and was not retried; GUI Restore remains absent. Mac close/menu/accessibility
acceptance remains separate. No live-model quality, release readiness or full
platform acceptance is claimed.

## Failures and corrections retained

Initial compiler failures identified removed APIs/shared ownership and were fixed
without weakening behavior assertions. First provider run: 499 passed, 3 failed,
1 ignored. Two assertions expected the old user-agent version. The third expected
Timeout after a raw-stream byte refusal; released rmcp 3.5 now emits
`transport_closed` for the originating unfinished RPC (streamable_http_client.rs
1767–1790), whereas 2.2 only warned. The corrected test expects Transport and
strengthens prompt refusal timing, retaining raw-byte-cap, one-call and zero-redirect
assertions. No production workaround was needed. Both corrected provider runs pass.

Initial formatting found an app test assertion needing rustfmt; corrected check
passes. Initial strict Clippy found the larger unified client variant; boxing
Copilot fixed the issue without suppressing the check. Original failed logs remain
under `/workspace/brn-environment/logs/rig045-migration`, with receipt hashes here.

Next: publication approval for the preserved stack, followed by ordinary draft PR
review/CI. Keep live-provider qualification and remaining platform UI gaps separate.
[Prepared draft PR description](draft-pr.md).
