# CLI agent contract: handoff
Written 16 October 2026, when the owner asked to wrap up and merge the reviewed work. This file lists what is done, what is left, and the decisions the next agent must respect. The authoritative records are the [owner decisions](owner-decisions.md), the [plan](plan.md) and the [evidence](evidence.md). The machine contract itself is in [`crates/brn/README.md`, "Agents and scripts"](../../../../crates/brn/README.md#agents-and-scripts).
## Done
- **Round 1:** the `--json` machine contract (one envelope, exit/status table, stable error codes), `--NAME-file PATH|-` text input, and `--key` retry identity with per-command retry classes. It also added `crates/brn/tests/agent_contract.rs` and the maintained journey runner `scripts/cli_journey.py`.
- **Round 2 (contract 2):**
  - paged inspection: `history`, `records`, `revisions`, `revision`, `runs`, `candidates` and `operation --id|--key`;
  - SIGTERM (exit 143), `--timeout` and the `busy` code;
  - per-command help, with global options accepted anywhere;
  - `missing_workspace`;
  - `invalid_request.details.reason`;
  - more keys (`topics-create`, `knowledge-check --seed-chat`, keyed intake that replays before conversion);
  - an offline loopback stub for `ask` outcomes;
  - owner items B2 (no default credentials in CLI-created workspaces), B4 (no-op converges return `Unchanged` with no receipt) and B3 part 1 (`edit_session_open` without the session ID, documented stale-save recipe).
- **Round 2b (still contract 2; see evidence "Round 2b"):** paged `notes`, `threads`, `needs-you`, `recovery` and `projects`, a paged `messages --thread`, `--hops`/`--budget`/`--trash` checked before opening, the hard stop after an earlier SIGINT, `busy` for `begin-edit` lock contention, the README usage row, and signal tests isolated from the caller's signal mask. The merge of `main` `9b47b285` replaced the round-2b `SQLITE_CONSTRAINT_UNIQUE` inference with the typed `Error::EditSessionOpen` arm (evidence "Merge of main 9b47b285").
- Rounds 1, 2 and 2b were independently reviewed by two reviewers, and both approved. They merged to `main` as PR #156 (`fb1b2100`). Shared-service changes are additive: no `brn-desktop` or `brn-ai` change, and no `ApplyOutcome` variant.
- **Round 3 (contract 3, branch `work/cli-agent-round3`; see [plan](plan.md#round-3-caller-bound-edit-sessions-and-workspace-purpose-phase-1) and [evidence](evidence.md#round-3)):**
  - caller-bound edit sessions (owner B3 part 2): one-time tokens, only their hash stored; owner-host desktop functions unchanged, and owner typing adopts a caller session; `reclaim-edit` works in test workspaces only;
  - B1 phase 1: `init --purpose test`, `--actor` on receipts, `authority {…, enforced: false}` in every envelope; every in-repo CLI test, script and witness that writes uses a test workspace;
  - schema 6 with upgrade option B: the CLI refuses older stores with `upgrade_required`, and only `upgrade --backup-directory` upgrades;
  - PR #156 LOW findings 1–4 fixed and tested.
## Left for the next agent, in priority order
1. **Workspace-scoped CLI authority, phases 2–3 (owner B1).** This is a separate assignment and needs a desktop review UI. In owner workspaces, CLI writes become exact-candidate proposals and owner-decision commands are refused (phase 2). The owner issues scoped, expiring grants from the desktop (phase 3). This guards against agent overreach. It does not protect against code running as the same OS user. Phase 2 also gives `set_workspace_purpose` a caller and an audit receipt.
2. **Still open, lower priority:**
   - `--key` for `ask` and useful intake (`brn-ai` mints the run identity);
   - cooperative cancellation inside an already-started backup, export, restore, upgrade or index build;
   - `NeedsReview` and `Superseded` cannot be reached from the CLI (they arrive with B1 phase 2);
   - receipts written inside a Run carry no actor (the Run record attributes them);
   - CI has no schema-5 binary, so the CLI upgrade process tests skip there; the core tests cover the upgrade.
3. **Desktop follow-ups (for a UI agent):**
   - desktop no-op archive/link still writes receipts; the core `*_converging` functions exist if the same behavior is wanted;
   - a CLI-created workspace has no default sign-in, so the desktop must ask for configuration;
   - de-flake `writing_tests::an_edit_already_open_elsewhere_offers_compare_and_discard` and `chat_tests::a_saved_working_rewrite_offers_the_same_confirmed_stop_in_the_note` (`native-ui,native-test-support`). They fail intermittently on `main` too, and only in parallel runs, which points to interference between tests (evidence "Round 3 final verification").
## Owner questions not yet answered
- **Security, predates this branch:** a writable Copilot cache can redirect Copilot traffic to plain HTTP. The recommendation is that `validate_record` require HTTPS, as a small separate task in `brn-ai`.
- `brn project-delete` of a project whose chat has a Run fails with "run control changes must advance the fence". This also happens on `main`, and it needs its own slice.
- Should chat intake into a project be refused? It is currently marked as an owner decision, and the recommendation is to keep the mark until B1 phase 2.
## Environment
- **PDF:** on 16 October the owner said *"no poppler forge abut the pdf stuff for now"*. The Poppler-only intake test stays an environment gap: report it as such and don't install Poppler.
- **PTY test:** run `cargo test -p brn` with the pseudo-terminal test enabled outside a sandbox that blocks `posix_openpt`.
