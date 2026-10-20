# Saved proposal stale-apply CLI preflight

Read-only preflight completed in `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust`.

Checkout observed: branch `codex/agent-first-followthrough`, HEAD `d760eb00e5c65729f43dd959133e5463d3ad67ea`; existing dirty changes are `docs/work/active/vacation-campaign/campaign.json` and `docs/work/overview.md`, preserved. This differs from the assigned expected integrated commit label `integrated4c50b614`; lead should verify whether this is the intended source before execution.

## Supported route

The production CLI supports the full requested sequence; no SQL/library bypass is needed:

1. On the fresh restored clone, inspect `candidates` and `operation --id OP` to establish exact prepared candidate identity/state and its expected Note version, then `read --id NOTE` for the current version. Candidate list fields are documented in `crates/brn/README.md`; operation exposes prepared/applied state, writes and input versions.
2. Use `begin-edit --note NOTE --base CURRENT_VERSION --token-out /tmp/...` (absolute, fresh token file) to open the CLI caller-owned edit. It returns the session and token file path. Then `update-buffer --session SESSION --generation 1 --text-file /tmp/later-edit.md --token-file /tmp/...`, followed by `save --session SESSION --generation 1 --token-file /tmp/...`.
3. Attempt the original candidate exactly once with `apply --operation OP`.

Machine mode invocation form: `brn --json --data-dir FRESH_CLONE COMMAND ...`; README says `--json` must be first and global options can otherwise appear in option-name positions. Save/update require `--token-file`; there is no argv token form. `--text-file` accepts absolute path or `-`, and preserves bytes exactly. Token output path must not already exist and has 0600 mode.

## Guards and expected outcome

`begin-edit` requires no existing open edit session for the Note. It binds to the requested Note base version, but a later save is a real accepted-version write. The returned caller token exclusively authorizes that session; owner typing adopts the session. Sessions do not expire. `reclaim-edit` is test-workspace-only; do not use it for this sequence. Save uses latest buffer generation and caller token; stale save is a separate failure and should not be auto-rebased.

Apply is explicitly an owner decision. It rechecks all Current/ChatContext inputs against the transaction prestate; when the target Note/input version changed, core returns `ApplyOutcome::Stale { records }` before grouped writes. CLI JSON maps Stale to `status:"stale"`, exit 3, `outcome:"Stale"`, with conflict IDs/versions. No Apply receipt should be created; proposal and Run should remain, and later Note text/version should remain exact. `apply` is replay-class, so the lead's requested single attempt is appropriate; do not blindly retry.

Potential alternate guards: an open edit causes Apply to return Deferred (exit 4), so close the caller's own saved session as part of save (save closes it), and verify recovery has no remaining session. A Working Run may also guard writes and defer; inspect `runs` and do not start/recover a Run as part of this test. Candidate may be retired/applied already; proceed only if operation inspection confirms `prepared` original candidate. The MAC-09 evidence identifies candidate operation `7571d9e7-71f7-432f-9d9c-d3fad71589a0`, but MAC-09 describes Apply and Undo on its backup clone. Confirm state in the actual selected source/clone before choosing it.

## Evidence read

`crates/brn/README.md` sections “Text input,” “Recovery recipes,” operation/candidates result schemas, and owner decisions; `crates/brn/src/help.rs` command signatures and save/begin-edit notes; `crates/brn/src/main.rs` begin-edit/update-buffer/save/apply routing and stale mapping; `crates/brn/src/spec.rs` command option requirements; `crates/brn-threads-core/src/store.rs` prestate checks; `docs/architecture/threads-behavior.md` caller-held token and workspace-purpose contract; `docs/work/active/vacation-campaign/campaign.json` selected qualification; and `docs/work/active/vacation-campaign/evidence/native-mac-session-20261020.json` MAC-09 operation identity.

No builds, tests, native interaction, workspace access, source fixture reads, or provider/account work were done. No supported-route ambiguity found. One source-control caveat: the checkout HEAD differs from the supplied integrated commit description; lead should confirm intended commit before running the qualification.

Lead source disposition: `d760eb00` is the intended progress branch and contains integrated`4c50b614`; the complete `crates`, `scripts`, `tools`, `patches`, root manifest and lock trees compare identical. Its later commits change campaign documentation only. Immutable CLI was rebuilt from those matching production sources. The apparent HEAD difference is not a product-source conflict. Original report remains at its named temporary source.
