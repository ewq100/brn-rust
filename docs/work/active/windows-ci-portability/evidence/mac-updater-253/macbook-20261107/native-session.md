# Combined B/A native session on the MacBook

Current execution location: MacBook `EMB-60VVL9KC`, arm64/macOS26.5. B owns the
combined session using A's recorded handoff; no new A execution dependency.
This derives from the unchanged [original checklist](../alpha-a921e325/native-session.md).
All native cases below are **pending** until an observation is recorded.

## Inputs and owner participation

1. Copy the complete Mac mini `/Users/Shared/BRN-alpha-a921e325-acceptance` folder
   to that same path on this MacBook. Transfer release artifacts and public
   fixtures only. B verifies original A alpha.1/build3 and B alpha.2/build4 against
   committed hashes/signatures before use. Keep originals unchanged.
2. Create and sign into a disposable standard macOS account with no BRN workspace,
   Copilot or Codex credentials. Keep the existing personal/managed profiles
   untouched. Make the Shared artifacts accessible to the test account. B uses
   only fresh synthetic data in this profile.
3. Remain present for ordinary per-app Gatekeeper approval if shown and the
   tester's normal Copilot browser/device login. Never copy credentials from the
   Mac mini or another app. A second account is optional for live switching;
   record unavailable quota/policy/no-Luna/account-switch cases as unobserved,
   retaining A's simulated evidence separately.

This MacBook has Xcode and the pinned Rust toolchain installed. Record that
environment; use the app and stock updater through ordinary recipient paths
without invoking development tools. Preserve the installed tools.

Before starting, coordinate any actual D/E heavy build, timing run, native session
or shared-account conflict. There is no standing reservation or wait for their
experiments. Record package/source/version/build, OS, delivery route, account type
without identity/secrets, exact model and requested/effective effort where known.

## Available copied-package leg

1. Extract verified A into a fresh directory; install `BRN Threads.app` in the
   clean profile's writable `~/Applications`. Open normally, observe first launch
   and empty workspace. Follow ordinary per-app approval if macOS requests it.
   Record if no prompt appears. No security-setting or quarantine workaround.
   This copied-artifact leg cannot satisfy browser-download Gatekeeper acceptance.
2. Connect GitHub Copilot in BRN via normal device flow. Observe cancellation of
   sign-in then successful sign-in, connection status and account model catalogue.
   Select an offered qualified model, preferably A's `gpt-6-luna`, and record the
   exact identifier and effort. No Codex account is configured.
3. Stream a reply, invoke BRN read tools, continue across turns and Stop an active
   run. With web allowed, require provider-native search with inspectable returned
   source URLs. Observe Web Off/no search and cancellation/late-result behavior.
   Preserve clear provider/network errors and drafts rather than retry blindly.
4. Import supplied synthetic `harbor.docx` with refinement Off, then On through
   Copilot Luna/high. Inspect retained quantities, exceptions, structure, source
   references and figure assets. Record actual provider/model/provenance; Off
   must work without model processing. Exercise useful knowledge/local search and
   an evidence-based answer plus a question whose information is absent.
5. Prepare consolidation of overlapping/contradictory synthetic notes, inspect
   exact candidates, Apply, then Undo. For referenced writing: Discard; Apply and
   Undo; prepare another candidate, edit the target note, then verify stale Apply
   refusal keeps the newer owner text. Check cancellation/protected content.
6. Create identifiable synthetic notes/project links, chat/history, figure asset
   and draft; save provider/model/document settings. Check compact archive
   disclosure at900×600 and relevant remaining #243 native journeys. Create a
   normal backup outside the workspace. Quit through normal prompts and reopen A;
   inspect data/settings and continue chat. Sign out/reconnect normally and
   continue. If a second account exists, verify started work refuses the changed
   account and fresh work uses it. Restore the backup into a separate fresh
   workspace; inspect useful state and retained drafts. Old editor tokens should
   be revoked on restore. Never test an old app against a migrated store.

## Published-artifact leg, after explicit approval

1. Finish available local acceptance before the release decision. Present exact
   alpha.1/build3 manifest/notes, fixed runtimea921e325 and actual candidate head,
   pending hosted source identity and remaining limitations. The machine move
   does not approve publication. Preserve any later explicit owner decision.
2. After authorized normal merge and required resulting-main CI, let the existing
   immutable release workflow build/publish actual approved A. Record its full
   source SHA and hosted ZIP/manifest/checksum identities separately. Download
   that actual A in the browser on the clean profile, install/open normally and
   observe Gatekeeper. Do not overwrite the copied-leg installation/data blindly;
   use a separate clean installation/session as needed and record it.
3. Use the separately approved increasing B manifest/source and normal workflow.
   Once both are published, run existing `prepare-published-macos-update.py` and
   `test_macos_updater.py --qualify-update --stock-transport` with a verified,
   matching test CLI and absent task-owned output paths. Retain preparation,
   source/receipt and unchanged updater evidence. Preparation alone is no pass.
4. Quit published A normally and run its unchanged command in Terminal:

   ```sh
   "$HOME/Applications/BRN Threads.app/Contents/Resources/bin/brn-update" v0.2.0-alpha.2
   ```

   Observe real fixed-repository HTTPS replacement, recovery/previous-app path,
   ordinary approval if shown, and normal reopening of B. Inspect notes, project,
   chat/history, draft, figure asset and settings. Check Copilot continuity or
   normal reconnect, chat/tools, cited web search, Luna refinement and a separate
   native restore. A manual B copy or substituted transport cannot satisfy this.
5. Record native observations and failures in this MacBook evidence directory and
   update the existing task, PR268 and #253/#252. Owner acceptance and permission
   for invited distribution remain separate from upload, checks and native use.
