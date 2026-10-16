# One shared intake instruction, reused on re-supply (round 5 slice E)

Slice E (`dd2c8ce8`, on `b037140c`) stops a reused intake chat from collecting duplicate owner instructions. The desktop's existing wording moves verbatim into `brn_threads_app::SOURCE_INTAKE_INSTRUCTION`, and the CLI's useful-intake `ask(..)` call now passes that constant. A single runtime rule, `brn_ai::threads::reusable_intake_instruction`, returns the chat's latest owner Message when three things hold: its text equals the prompt, it has no newer owner text after it, and a non-archived Run in that chat that it instructed received every supplied source (slice B's `matching_sources`). `begin()` applies the rule. The desktop applies it through `Workspace::post_intake_instruction`, which skips `post_message` when the rule matches. Review covered the diff, the slice spec, the slice E evidence and consumers of `Run.instruction`. Suites were not re-run.

Watch for: verification's Limitations entry for a desktop instruction whose Run never started is wrong for the desktop host. `post_intake_instruction` posts a second identical instruction there even when the orphan is the chat's last Message (confirmed by reading, non-blocking, pre-existing).

**Verdict**: APPROVED

## High-level view

Both hosts send the same prompt, and both decisions come from one function. The desktop asks first through a read-only store wrapper, and then `begin()` decides again. In normal use the two decisions agree: both sides canonicalize the locator with the same function (`prepare_run` before `begin()`, and the wrapper itself), and both use the constant. Reuse does not change the model's input. `retained` is built from the Messages that existed before the start write, so a freshly posted copy was never part of it.

The shared checked service is still the only write path. The desktop posts with `Workspace::post_message`, and `begin()` writes the Run and its `instruction` in its one checked start group. The rule only reads. Reuse is limited to the chat: Source matching covers any scope, but only Runs with `run.thread == chat` count. `source_intake_chat` is untouched, so project scoping is unchanged. Edit-note, decision and rewrite requests are excluded explicitly, which keeps `owner content instruction` and `owner decision instruction` authority bound to their own Message. Queued, Steer and Continue Runs pass no sources, so the rule never matches them. In production, only useful intake supplies sources.

The desktop text is byte-identical to the literal at `dd2c8ce8^`. The CLI wording change is noted in the commit and in verification. It drops "Reconcile current notes and suggested Actions", "leave raw text external" and "ask for review of protected decisions". The quoted replacements exist in `agent/skills/intake/SKILL.md` (lines 17, 25) and `agent/skills/maintain-notes/SKILL.md` (line 27), and `threads_guides::preloaded(true)` loads both skills for every source Run. Protection is enforced by the host regardless of the prompt. No reply or delivery wording changes.

Ownership matches the plan, apart from rustfmt reflow, which is noted in both places. The desktop production change is the `useful_intake` call plumbing: 7 lines, with the `Ok(_)` arm unchanged. `main.rs` has one statement that rustfmt reflowed to 8 lines once the long literal was gone. Cargo files, `Cargo.lock`, `docs/status.md` and `docs/architecture/*` are unchanged, and the proposed `threads-behavior.md` sentence is an integration item. All 10 new or extended tests have a recorded failure before the fix. Each failure line matches its assertion in the committed sources. The deltas agree with slice D's totals: app 11→16, CLI `source_locator` 1→2, desktop 154→157, workspace 766→772. Both cross-binary evidence stores exist under `/private/tmp/brn-dwr-data/`.

<details>
<summary>Issues (1)</summary>

1. **Desktop orphan-instruction limitation is misstated** — Verification's first Limitations bullet says `begin()` reuses an unrun instruction that is still the chat's last Message. For the desktop, `post_intake_instruction` first posts a second identical copy, because the rule needs a Run that named the Message. Either restrict that sentence to the CLI, or have `post_intake_instruction` also skip when the chat's last Message is an identical owner instruction; `begin()`'s `last_is_prompt` would then bind it. Non-blocking; not a regression.

</details>

<details>
<summary>Details</summary>

### Two decision points over one rule

```
desktop useful_intake                         CLI intake
  source_intake_chat(project, locator)          source_intake_chat(project, locator)
  post_intake_instruction(chat, locator)        ask(.., SOURCE_INTAKE_INSTRUCTION, [source])
    intake_instruction_to_reuse  ──┐               │
      (canonical_locator, records,  │               ▼
       receipts → chat_messages)    │          prepare_run (canonicalizes) → begin()
    Some → write nothing            ├── reusable_intake_instruction ──┤
    None → post_message(constant)   │                                 │
  start_run(constant, [source]) ────┘──► prepare_run → begin() ───────┘
                                          last_is_prompt? → last Message
                                          else reuse rule? → latest owner Message
                                          else → new Message in the start group
```

The two reads are separate. An interleaved owner write can make the desktop skip its post, and `begin()` then posts a fresh instruction. The result is an extra instruction, never a missing one. Verification records this. When the desktop does post, `begin()` binds the new Message through `last_is_prompt`. Without a concurrent write, the two paths cannot bind different Messages.

### Orphan instruction on the desktop

`reusable_intake_instruction` matches only a Message that some Run named, and `post_message` does no deduplication. Two kinds of failure leave the desktop's posted instruction with no Run naming it:

- `start_run` returns before `prepare_run`: no provider chosen, a Copilot model not offered, an unsupported effort, or a worker that started while the file was converting.
- `prepare_run` refuses.

On the next supply of that source, `source_intake_chat` reuses the chat only if an earlier Run there received the source. In that case `post_intake_instruction` posts an identical instruction directly after the orphan, and `begin()` binds the new copy. This is narrow: it needs an earlier Run for the source in that chat, a reason for the first post (such as owner text in between), and a failed start. The pre-slice code posted unconditionally, so this is not a regression. It does contradict the last sentence of verification's first Limitations bullet, "If the instruction is still the chat's last Message, `begin()`'s existing rule reuses it", which holds only for the CLI host. With no earlier Run, the re-supply gets a new chat, and the orphan stays alone in the old one. That is also pre-existing.

### Run.instruction can now precede earlier replies

Before this slice, a Run's `instruction` was the explicit `request_message` or the chat's last Message at start. Now it can be an owner Message followed by assistant replies from earlier Runs. No consumer depends on position:

- Queue binding and completion in `chat.rs` compare exact IDs, and queued Runs carry their item's `request_message`.
- The writing tile in `threads_workbench.rs` matches its writing request.
- The core immutability check compares a Run's old and new values.
- Nothing in desktop chat rendering anchors on `instruction`.

The visible effect is that the transcript shows instruction, reply 1, reply 2, with no owner turn between the replies. That is the slice's intent.

### Test coverage

The "rule disabled" mutation (`if true || references.is_empty()`) reproduces the pre-fix behavior: an unconditional desktop post, and `begin()` reusing only the last Message. E-T1, E-T2, E-D1 and E-D2 failed under it. The old CLI literal made E-C1 and the cross-binary E-D4 fail. The guards E-T3, E-T4, E-T5 and E-D3 failed under targeted mutations of the latest-owner check, the per-source check and the text check.

Not tested:

- A desktop re-supply after an assistant reply through the desktop harness. Every desktop leg fails offline, so the instruction stays last. The app-level E-T1 covers this shape.
- Exclusion of archived Runs and archived instructions.
- A source Run that carries `owner_request`. This is unreachable in production today.
- The orphan-instruction case above.

The count part of E-C1 has no fail-before, because an offline CLI failure leaves the instruction last. Verification states this. Verification also discloses one unexplained `:18` exit of `brn` on the old-wording tree, which did not recur in 6 further mutated runs or 11 runs of the fixed tree.

</details>

<details>
<summary>File map</summary>

- `crates/brn-ai/src/threads.rs`: `chat_messages` extracted from `begin()`; new `reusable_intake_instruction` and `intake_instruction_to_reuse`; `begin()` applies the rule.
- `crates/brn-threads-app/src/intake.rs`: `SOURCE_INTAKE_INSTRUCTION` and `Workspace::post_intake_instruction`.
- `crates/brn-threads-app/src/lib.rs`: re-exports the constant.
- `crates/brn-desktop/src/threads_ui.rs`: `useful_intake` uses the constant and `post_intake_instruction`.
- `crates/brn/src/main.rs`: CLI useful intake passes the constant (one statement, reflowed by rustfmt).
- `crates/brn-threads-app/tests/source_intake_chat.rs`: E-T1 to E-T5.
- `crates/brn-desktop/src/threads_intake_tests.rs`: E-D1 to E-D3; E-D4 extended.
- `crates/brn/tests/source_locator.rs`: E-C1 and an offline-store helper.
- `docs/work/active/daily-workflow-reliability/round5/verification.md`: slice E section.

Full diff: `git -C /Users/evokessler/repos/brn-rust/.worktrees/daily-workflow-reliability show dd2c8ce8`.

</details>
