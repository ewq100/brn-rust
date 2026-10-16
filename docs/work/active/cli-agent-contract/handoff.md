# CLI agent contract: handoff

Written 16 October 2026, when the owner asked to wrap up and merge the reviewed work. This file lists what is done, what is left, and the decisions the next agent must respect. The authoritative records are the [owner decisions](owner-decisions.md), the [plan](plan.md) and the [evidence](evidence.md). The machine contract itself is in [`crates/brn/README.md`, "Agents and scripts"](../../../../crates/brn/README.md#agents-and-scripts).

## Done and merged with this branch

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
- **Round 2b (still contract 2; see evidence "Round 2b"):** paged `notes`, `threads`, `needs-you`, `recovery` and `projects`, a paged `messages --thread`, `--hops`/`--budget`/`--trash` checked before opening, the hard stop after an earlier SIGINT, `busy` for `begin-edit` lock contention, the README usage row, and signal tests isolated from the caller's signal mask. At merge, delete the round-2b `SQLITE_CONSTRAINT_UNIQUE` inference in `begin_edit_error` together with `recovery_buffers()` and keep the typed arm (merge-resolution notes).
- All three rounds were independently reviewed by two reviewers, and both approved. Shared-service changes are additive: no `brn-desktop` or `brn-ai` change, and no `ApplyOutcome` variant.

## Left for the next agent, in priority order

1. **Round 3, caller-bound edit sessions (owner B3 part 2).** `begin-edit` returns a secret that buffer, save and discard writes must present. The design must:
   - store only a hash, and never show the secret in errors, listings (including `recovery`), receipts or logs;
   - keep the secret out of argv where possible;
   - explain how the desktop holds sessions, and how its recovery buffers reattach after a restart (in-process owner-host binding vs a stored secret);
   - define what a desktop-held session looks like to the CLI;
   - define lost-token reclaim (the owner through desktop Compare/Discard; the CLI only in a test workspace; expiry must not break invariant 8);
   - define how restart and backup/restore treat the secret hashes;
   - follow the schema-upgrade rules for the schema change.

   **Coordination:** the `work/ui-polish` lead asked for the proposed shared-seam API on the coordination board *before* the design is approved. Desktop U4 uses only `note` and `base_version` from `Error::EditSessionOpen`.
2. **Round 3, workspace-scoped CLI authority, phase 1 (owner B1).** Record a workspace purpose:
   - `brn init --purpose test` sets it, and only at creation. Existing and desktop workspaces are owner workspaces. The CLI has no command to change purpose; a desktop-only API can be defined later.
   - A caller-supplied actor is recorded on receipts. Define whether it affects the request hash and replay.
   - The effective authority is reported in `brn --json --help` and in the envelope, with `enforced: false`.
   - Move in-repo tests, scripts and witnesses to `--purpose test`.

   Phase 1 enforces nothing new. **Phases 2–3 are a separate assignment that needs desktop review UI:** CLI writes in owner workspaces become exact-candidate proposals, and the owner issues scoped, expiring grants. This guards against agent overreach. It does not protect against code running as the same OS user.
3. **Still open, lower priority:**
   - `--key` for `ask` and useful intake (`brn-ai` mints the run identity);
   - cooperative cancellation inside an already-started backup, export, restore, upgrade or index build;
   - `NeedsReview` and `Superseded` cannot be reached from the CLI (they arrive with B1 phase 2).
4. **Desktop follow-ups (for a UI agent):**
   - desktop no-op archive/link still writes receipts; the core `*_converging` functions exist if the same behavior is wanted;
   - a CLI-created workspace has no default sign-in, so the desktop must ask for configuration.

## Owner questions not yet answered

- **Security, predates this branch:** a writable Copilot cache can redirect Copilot traffic to plain HTTP. The recommendation is that `validate_record` require HTTPS, as a small separate task in `brn-ai`.
- `brn project-delete` of a project whose chat has a Run fails with "run control changes must advance the fence". This also happens on `main`, and it needs its own slice.
- Round 2b made `projects --trash` values other than `yes`/`no` an `invalid_value` usage error; they used to list active projects with exit 0 (README Compatibility 8). Confirm with the owner at merge.
- Should chat intake into a project be refused? It is currently marked as an owner decision, and the recommendation is to keep the mark until B1 phase 2.

## Environment

- **PDF:** on 16 October the owner said *"no poppler forge abut the pdf stuff for now"*. The Poppler-only intake test stays an environment gap: report it as such and don't install Poppler.
- **PTY test:** run `cargo test -p brn` with the pseudo-terminal test enabled outside a sandbox that blocks `posix_openpt`.
