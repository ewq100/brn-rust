# Combined B/A native session on the MacBook

Current execution location: MacBook `EMB-60VVL9KC`, arm64/macOS26.5. B owns the
combined session using A's recorded handoff; no new A execution dependency.
This derives from the unchanged [original checklist](../alpha-a921e325/native-session.md).
All native cases below are **pending** until an observation is recorded.

## Stage 1: current-account functional testing

The owner's 7 November clarification in #253 authorizes this stage before a clean
OS account exists. Use the packaged desktop's supported explicit data argument;
ordinary Finder launch uses the user's fixed preview workspace and is reserved
for Stage 2. Keep the signed launcher unchanged and HOME unchanged.

Current task-owned locations, verified before launch:

- Root: `/Users/Shared/BRN-native-functional-253-20261107` (mode0700).
- Synthetic workspace: root's `workspace` directory.
- Copilot credentials: root's `copilot-credentials` (mode0700, initially empty).
- Fixtures, backups, logs and A/B installations: separate named directories under
  that root. Credentials are excluded from committed evidence.

The verified test CLI created the fresh test workspace and configured Copilot,
`gpt-6-luna`/high, this exact credentials folder, no Codex auth file, helper Off
and review-first before any desktop connection. Read canonical settings and the
native Models form again before clicking Connect. Confirm the running process's
`--data-dir`, executable and open database target. Preserve personal workspace
and sign-ins without opening, copying or resetting them.

The original transfer folder remains unavailable; rebuilds from exacta921e325
are permitted by the existing task and receive separate MacBook identities.
Never relabel a rebuilt ZIP as an original Mac mini ZIP. Verify package signatures,
receipts, helper/updater identities and platform before starting its binary:

```sh
"/Users/Shared/BRN-native-functional-253-20261107/install-A/BRN Threads.app/Contents/Resources/bin/brn-desktop" \
  --data-dir /Users/Shared/BRN-native-functional-253-20261107/workspace
```

Current observations are in the [separate native evidence](../macbook-native-20261107/README.md).
Automation could not bind the unregistered direct process. A distinct signed
`BRN Functional 253.app` test wrapper uses unchanged packaged executable bytes,
unique bundle ID and explicit task-owned data/log paths. The original release app
and signed launcher are unchanged. This remains functional test scaffolding.

Use the following as a functional checklist, recording actual native results;
all cases start pending. Offline checks can proceed before tester device sign-in.

1. Inspect the isolated empty/new synthetic workspace, Settings and exact Models
   credential folder. Save/inspect settings without touching the normal profile.
   Import public synthetic fixtures with refinement Off; inspect content/assets,
   edit and search notes, archive disclosure at900×600, history and ordinary Undo.
2. Connect Copilot through normal device flow only after target verification.
   Ask the tester for the required browser/device-login action when the prompt is
   present. Observe cancellation then successful connection, model catalogue,
   exact offered model/effort and any accurate unavailable states. No existing
   Copilot credentials are copied; no Codex route is configured.
3. Stream chat, invoke BRN tools, continue and Stop. Require real cited native web
   search with web allowed; test Web Off and late-result/cancellation behavior.
   Import harbor.docx with refinement On through Copilot Luna/high as well as Off.
   Inspect quantities, exceptions, source links, figure assets and provenance.
4. Consolidate overlapping/contradictory synthetic notes: inspect candidate, Apply
   and Undo. Referenced writing: Discard; Apply/Undo; prepare again, edit its target,
   then verify stale refusal keeps the newer text. Check protection/cancellation.
5. Create identifiable notes/project links, chat/history, figure asset and draft.
   Save model/document settings. Make a native backup outside the workspace;
   quit normally, reopen the same binary with the same explicit data argument,
   verify preservation and continue. Reconnect using only the test-owned folder.
   Account-switch/sign-out checks require verified test-owned targets; unavailable
   second-account/quota/policy/no-Luna cases remain honestly unobserved.
6. Restore only into a separate fresh task-owned workspace after confirming the
   actual restore target and backup source. Inspect useful state and drafts;
   old editor tokens are revoked. A manual switch to separately verified local B
   after normal quit can check functional app replacement/preservation, recorded
   separately from the real stock HTTPS update. Never open a migrated store with
   an old binary or silently replace owner data.

This is **alternate-data-directory functional testing**, not ordinary Finder
first-launch or clean-user acceptance. The host has system-wide Xcode/Rust;
record that environment rather than claiming a fresh machine without dev tools.
D/E current threads are complete and no competing cargo/rustc/BRN run was observed
before B's new build. Coordinate any new actual host/account overlap.

## Stage 2: ordinary launch and clean-user acceptance — pending

When Stage 1 is complete, the straightforward remaining clean-user method is one
standard disposable macOS account with no BRN data or credentials. Sign into it,
make the verified package/public fixtures available, install into its writable
`~/Applications` and open through Finder using the unchanged signed launcher.
Observe fresh per-user data setup, standard-user permissions and normal Copilot
sign-in. Be present for ordinary per-app Gatekeeper approval if macOS requests it.
A fresh OS account does not remove system-wide development tools.

For full browser/Gatekeeper delivery acceptance, use the actual approved published
ZIP through the browser. A copied/rebuilt ZIP or direct-binary launch is recorded
by its actual route and does not satisfy that published-download leg. Creating an
OS account does not block Stage 1. Do not fake HOME or edit the signed launcher.

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
