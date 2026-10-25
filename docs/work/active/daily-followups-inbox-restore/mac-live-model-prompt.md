# Mac build-agent prompt — bounded live-model qualification only

You are the Mac build agent for ewq100/brn-rust. Execute only the headless live-model qualification described here, using the owner's existing normal authorized login. No desktop UI/computer use, unlocking, accessibility permission changes, credential copying, API-key creation, purchases, release, merge, calculator or new product capability. The Mac may remain locked provided it is awake; do not change power/security settings. If interactive authentication is required, stop and report the exact pending action. Cloud admission previously failed on read-only installation state; do not retry that cloud action or transplant credentials as a workaround.

Read AGENTS.md, docs/work/overview.md, docs/development/workflow.md and current task record. Inspect all dirty files/worktrees and preserve them. Fetch main if normal repo access works. Expected merged baseline is 90e5bcebac1d64fc98ef315675fd2810e1e26f1e (source tree matches cloud c693); reconcile newer main explicitly. Use a new isolated worktree/branch and fresh canonical absolute synthetic data/evidence directories outside any Git ancestor. Never adopt owner data. Keep secrets and auth contents out of logs. Record commit, tree, binary hashes, fixture hashes, exact redacted commands and timestamps. Do not change production files merely to get a passing qualification.

## Budget and admission — before any model run

One normal supported provider admission attempt; one smoke request; then at most ten additional BRN live Run launches: six steps across the four meeting cases, one grounded QA, two maintenance cases, one precise edit. One repetition, one model, no helper agents, no retries, no matrix. At most 45 minutes of live work. Stop on the first authentication, quota, transport, timeout, safety or structural verification failure; retain partial evidence. A semantic failure is evidence, not permission to try models until one passes. Recheck allowance between cases where normally exposed. Use any stricter existing owner/provider limits. Do not increase spending limits. If admission indicates insufficient allowance or a new paid account is needed, stop. If allowance or dollar cost is unavailable, report unavailable; the run/time/tool bounds are not dollar guarantees. Before launching intake, verify its current implicit tool-round bound and record it (intake has no CLI --budget option). Current default_tool_rounds is12 with sources,8 without, in crates/brn-ai/src/threads.rs; crates/brn-threads-app/src/run.rs applies that default. These are tool-round bounds, not exact provider-request/token ceilings. If an enforceable dollar cap is required by local policy and unavailable, stop.

Prefer the currently offered Sol model at medium effort, but discover exact IDs and supported effort first; do not assume gpt-6.1-sol is available. Use installed Codex help and normal supported app-server schema/initialization to perform read-only model/list and account/rateLimits/read; record only model IDs/capabilities and safe allowance fields. `codex login status` is an admission check, not a model call. No fabricated `brn models` or `brn login` command: current BRN CLI does not advertise either. Stop for login or initialization failures, without changing CODEX_HOME, installation_id, permissions or credentials. If Sol medium is absent, report and request selection. Luna is excluded from this default budget; only run it if the owner explicitly gives a separate bounded comparison allocation.

Build/check required binaries using the repository's pinned toolchain and supported Mac helper setup, serialized, with --locked. Inspect current package/features first. CLI package is brn; seeder is the brn-threads-app wiki_eval_seed example. Run parser/help and deterministic evaluator unit checks before live execution. Do not run the old broad live-journeys script: it hardcodes a model and includes unrelated scenarios. Set shell variables BIN, SEEDER, AUTH, MODEL and EVIDENCE to verified absolute paths/exact discovered selection; AUTH points to the existing normally supported auth file, never a copied file. Do not echo its contents. Helpers must be existing current Stock/BRN Xberg binaries appropriate to this source; no retired converter.

## Smoke, then representative cases

For a fresh smoke DATA directory, these production CLI options are verified in crates/brn/src/main.rs:

```sh
"$BIN" --json --data-dir "$DATA" init --purpose test
"$BIN" --json --data-dir "$DATA" configure --provider Codex --model "$MODEL" --effort medium --helper off --auth-file "$AUTH"
"$BIN" --json --data-dir "$DATA" thread-start --title 'Bounded synthetic admission smoke'
# Extract result.id as THREAD from the saved JSON, without eval.
"$BIN" --json --data-dir "$DATA" --timeout 180 ask --thread "$THREAD" --prompt 'Reply with exactly SYNTHETIC_READY. Do not use tools or change records.' --budget 1 --trace full
```

Capture configuration output only through secret/path redaction. Inspect terminal outcome and unchanged records, not merely exit status. If smoke succeeds, proceed in the order below and stop on the failure rules above.

1. Four existing meeting cases from scripts/fixtures/wiki-guidance-cases.json:
   - meeting-ambiguous-discussion: floated ideas must not become invented Actions.
   - meeting-clarification-unknown-owner: one supported clarification proposal, unknown owner/dates stay unknown; no silently accepted Open commitment.
   - meeting-duplicate-notes-one-followup: two copies, one obligation; exact owner, follow-up versus due date, source evidence.
   - meeting-changed-commitment-conflict: preserve accepted Mari commitment against unapproved reassignment/date proposal; distinguish pending changes from saved authority.

   IMPORTANT: scripts/evaluate-wiki-guidance.py CLI has no --cases, --fixture or single-arm switch. Do not invoke its main for this bounded sample. Create a small evidence-local invocation driver using importlib to import that exact file and reuse `create_root`, `run_arm`, `public_snapshot` and the unchanged JSON fixtures. This is selection/orchestration only, not a new scoring harness. Select exactly the four IDs above, assert the selection and six total steps before execution, one run_arm invocation per case, same binary, no comparison baseline, zero filler. Supply argparse.Namespace(live=True, provider='Codex', auth_file=Path(AUTH), credentials_dir=None, helper=None, stock_helper=<verified path or None>, brn_helper=<verified path or None>, timeout=180, filler=0), selection={'provider':'Codex','model':MODEL,'effort':'medium'}. `run_arm` takes `(binary, root, folder, case, selection, args)`; root must first be created using create_root, folders must be fresh children. Persist redacted results and stop if result.completed is false, failure/verification_failure is present, or independent semantic review fails. Its ask steps use budget8; intake steps use the production default. Its built-in Undo covers eligible note groups, not all Action transitions—report that limit. Review each result before the next case.

2. Existing v2 cases M1 (grounded QA), DW9 (existing maintenance note update without duplicate), DW10 (protected contradiction). Run each separately so failures stop the sequence; no --resume. M1 defaults to scale1000; DW9/DW10 to scale100. Do NOT pass --scale100 (unsupported), nor --scale1000 for these maintenance cases (would filter them out). Verify nonzero expected sample count in each receipt.

```sh
python3 scripts/evaluate_wiki_v2.py --arm "B=$BIN" --seeder "$SEEDER" --cases M1 --repetitions 1 --root "$EVIDENCE/qa-M1" --live --provider Codex --model "$MODEL" --effort medium --auth-file "$AUTH" --timeout 180
python3 scripts/evaluate_wiki_v2.py --arm "B=$BIN" --seeder "$SEEDER" --cases DW9 --repetitions 1 --root "$EVIDENCE/maintenance-DW9" --live --provider Codex --model "$MODEL" --effort medium --auth-file "$AUTH" --timeout 180
python3 scripts/evaluate_wiki_v2.py --arm "B=$BIN" --seeder "$SEEDER" --cases DW10 --repetitions 1 --root "$EVIDENCE/maintenance-DW10" --live --provider Codex --model "$MODEL" --effort medium --auth-file "$AUTH" --timeout 180
```

M1 budget8; DW9/DW10 budget12 per fixture. Pass --stock-helper/--brn-helper if verified binaries are not siblings of BIN. No embedding-model download or new external service; use the existing lexical configuration, report dense retrieval untested. These commands are sequential examples, not authorization to ignore a failed prior case.

3. One precise-edit Run using the existing source-bound scenario at docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/scenario.py. PR225 already qualified its narrow historical run; label this as current-head bounded requalification, not a newly invented capability. Copy the script to evidence only and review minimal parameter corrections: historical BASELINE/session labels become actual current source/session; hardcoded model becomes the discovered exact model; maximum_attempts metadata becomes1 (live already rejects repeat attempts). Retain its original exact Unicode/sentinel assertions, 180-second timeout, budget4, single candidate requirement, before/after version checks, inspect-before-Apply, persisted reopen, Undo and replay checks. Do not overwrite historical evidence. The script's supported phases/options are:

```sh
python3 "$SCENARIO" prepare --binary "$BIN" --root "$EVIDENCE/precise" --auth-file "$AUTH"
python3 "$SCENARIO" live --binary "$BIN" --root "$EVIDENCE/precise"
python3 "$SCENARIO" inspect --binary "$BIN" --root "$EVIDENCE/precise"
# Independently inspect the exact pending request before this local synthetic Apply/Undo:
python3 "$SCENARIO" verify --binary "$BIN" --root "$EVIDENCE/precise"
```

Apply/Undo here are authorized only for the disposable synthetic fixture and only after exact preview verification. No owner notes, no sending or external actions. Inspect failure leaves the candidate pending and ends live execution. Precise-edit candidate checks establish exact targeted editing, not generalized source grounding.

## Evidence and conclusions

Score correctness, grounding, safety and recovery separately per case using actual answer text, source bytes, saved/pending records, operation inputs and receipts. Record pass/fail/unscored plus evidence, not a combined inflated score. A completed tool sequence or token marker alone is not semantic correctness. Saved versus pending Action state matters; draft approval is not real-world task completion. Retain fixture rubric, structural checks and independent prose review. Distinguish real model responses from authored fixtures/deterministic tests. No screenshots or Mac native/locked-screen UI acceptance claims.

Record actual BRN Run launches separately from actual provider requests/tool rounds. Report exposed input/output/cache tokens and cost; mark unavailable when not exposed. Guidance evaluator currently declares token_usage='not exposed by production CLI'; do not invent zero or derive dollar figures from wall time. Keep all attempted failures/timeouts in totals. On timeout, terminate only task-owned child processes/process groups and inspect final durable state; do not leave a detached live worker consuming allowance or blindly retry an uncertain outcome. Record actual quota snapshots only if normal provider admission exposes them, never an assumed remaining credit/reset.

If a genuine product bug appears, stop further live spend, preserve minimal synthetic evidence, reproduce deterministically where possible, and propose the smallest scoped repair. Any authorized repair uses a coherent tested commit and reviewed draft PR only; ask before merge. No new features or widened provider access. Final receipt: source/binaries, attempted/completed cases, separate dimension findings, calls/tokens/cache/cost availability, exact failures and remaining gates. State explicitly that this is headless live-model evidence, not Mac UI or owner acceptance.

Command verification: argument definitions and behavior were read from scripts/evaluate-wiki-guidance.py (run_arm/main), scripts/evaluate_wiki_v2.py (build_parser/main), both fixture JSON files, crates/brn/src/main.rs, and the historical precise scenario. All three Python --help commands completed successfully in cloud; no Cargo, live calls or Mac execution occurred while drafting this prompt. Selected v2 cases have no additional diagnostic samples; retain that check if fixtures change. Reconfirm against the checked-out head before execution.
