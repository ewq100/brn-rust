# Proposed remaining Mac headless live qualification

**Preparation only; separate owner approval required.** No Mac access, model calls,
credential changes or native interaction occurred while preparing this handoff.
The latest task approves offline reliability work, not this proposed execution.
Do not infer permission from older campaign prompts or their former budgets.

## Already observed; do not repeat

The [pending-provenance qualification](../daily-followups-inbox-restore/mac-pending-provenance-qualification.md)
passed the duplicate-meeting case at exact `800d26bb`: two additional Runs,
thirteen cumulative historical Runs, one pending Action, both exact Source
versions, predecessor retirement and null completion evidence. Earlier smoke,
ambiguous discussion and unknown-owner observations remain dated evidence.
The narrow precise-edit Mac headless qualification from PR225 also remains
historical passing evidence; this proposal does not repeat it. None is Mac UI
or general live-model reliability acceptance.

Keep the earlier cloud **355 passed / 156 failed / 1 ignored** credential-boundary
observation intact. Do not bypass those guards or reinterpret this narrower
proposal as a full-suite pass.

## Proposed bounded selection

Use one normally offered Sol model at medium effort, helpers off, no matrix:

| Order | Existing case | New BRN Runs | Tool-round ceiling per Run |
|---|---|---:|---:|
| 1 | `meeting-changed-commitment-conflict` | 2 | 12 (production intake default) |
| 2 | `M1` grounded answer | 1 | 8 |
| 3 | `DW9` existing-topic update | 1 | 12 |
| 4 | `DW10` protected contradiction | 1 | 12 |

Proposed ceiling: **five additional Run launches**, twenty minutes from first
live reservation through review, 180 seconds per live command. Failed and
uncertain launches count. Historical thirteen plus this maximum would be
eighteen, not five total. No retries, helper/diagnostic Runs, `--resume`, extra
cases or automatic continuation after failure. New offline DW13/DW14 fixtures
are not included in this live allowance.

Stop at the first semantic, structural, grounding, safety, transport, timeout,
authentication or quota failure. Resolve an uncertain command by reading its
saved state, never by blindly relaunching. Kill only task-owned processes on
timeout. No new account, purchase, credentials, token copying, interactive login,
security/power changes, model download, Mac UI or native file picker.

## Freeze source and admission before launch

After separate approval, choose the exact reviewed integrated commit, record
`git rev-parse HEAD`, tree hash and clean/dirty state, and use a fresh isolated
checkout. Do not use the historical hash above as the new build target. Read its
AGENTS and current task instructions. Recheck normal installed login, supported
model listing and exposed allowance without executing a semantic smoke Run.
If admission requires interaction, stop. Cloud admission history is not current
Mac admission. Preserve the exact model ID and mark unavailable allowance data
unavailable.

From that checkout, using the existing pinned toolchain and cache:

```sh
cargo build -p brn --bin brn --locked --offline --jobs 1
cargo build -p brn-threads-app --example wiki_eval_seed --locked --offline --jobs 1
python3 scripts/evaluate-wiki-guidance.py --help
python3 scripts/evaluate_wiki_v2.py --help
```

Verify build logs identify this checkout's local crates. Freeze CLI and seeder
artifacts in the fresh evidence directory and hash them, the fixture files,
evaluator source and any driver. Verify current Stock/BRN helper identity through
the existing setup; no retired converter route. Use an explicit synthetic root
outside Git ancestors. `BIN`, `SEEDER`, `AUTH`, `EVIDENCE`, and helper paths below
mean verified absolute paths; `AUTH` remains an existing authorized path, never
copied into evidence. Redact paths/private identifiers from publishable receipts.

## Changed commitment: reuse the existing evaluator

`scripts/evaluate-wiki-guidance.py` has **no case selector or single-arm CLI**.
Do not run its default matrix. Use an evidence-local selection driver importing
that exact module and its unchanged `wiki-guidance-cases.json`. Select only
`meeting-changed-commitment-conflict`, assert exactly two intake steps, and invoke
`run_arm(binary, root, folder, case, selection, args)` once. Call `create_root`
for a fresh evaluation-owned root; use a fresh child folder. Selection is
`{'provider':'Codex','model':MODEL,'effort':'medium'}`. Its Namespace is:

```python
argparse.Namespace(live=True, provider='Codex', auth_file=Path(AUTH),
    credentials_dir=None, helper=None, stock_helper=Path(STOCK_HELPER),
    brn_helper=Path(BRN_HELPER), timeout=180, filler=0)
```

Reuse the [historical dispatch-gate pattern](../daily-followups-inbox-restore/evidence/mac-pending-provenance-20261026/driver.py.txt),
but **do not execute that sanitized historical driver verbatim**: it admits only
the already-passed duplicate case, has historical placeholders, and checks
absence of accepted Notes that would reject this case's legitimate protected
seed. Preserve launch reservation before dispatch, timeout/deadline checks,
helper-off configuration, actual returned terminal and durable observation.

Before allowing the second intake, inspect `public_snapshot(snapshot(cli))`,
run unchanged `action_checks` and `protection_preserved` plus existing
`action_source_checks([0], after)` for the observed first-source prefix. After
intake two run the full unchanged `structural_checks(case, before, after)`;
the canonical fixture now declares `source_steps:[0,1]`, so both source links
and exact Current inputs are enforced automatically. Do not run the two-source
oracle prematurely before the second source exists. Do not weaken pending-versus-
saved facts or `evidence:null` to obtain a pass. The preexisting protected Note
must remain exact, not be mistaken for an unauthorized new accepted note. At
each prefix gate compare accepted Action IDs/payloads against the before
snapshot: no new accepted Action or unauthorized accepted-state mutation is
permitted; reviewable pending proposals are evaluated separately. Do not use a
blanket requirement that accepted records be empty.

At each gate independently inspect actual text and bytes: the protected Mari
commitment and 10 March delivery remain exact; an unapproved Leena/12 March
proposal is not accepted authority; one logical obligation remains; no sending,
completion or reminder is claimed. Read the same public state in a fresh CLI
process. A missing answer or incomplete state is unverified, not a pass. Do not
Apply a pending candidate to repair a failing observation. Save the actual review
verdict before allowing the next Run. The driver is orchestration around the
existing oracle, not another evaluator.

## Remaining v2 commands, one at a time

Only after the prior case has passed structural and independent semantic review:

```sh
python3 scripts/evaluate_wiki_v2.py --arm "B=$BIN" --seeder "$SEEDER" --cases M1 --repetitions 1 --root "$EVIDENCE/qa-M1" --live --provider Codex --model "$MODEL" --effort medium --auth-file "$AUTH" --timeout 180
python3 scripts/evaluate_wiki_v2.py --arm "B=$BIN" --seeder "$SEEDER" --cases DW9 --repetitions 1 --root "$EVIDENCE/maintenance-DW9" --live --provider Codex --model "$MODEL" --effort medium --auth-file "$AUTH" --timeout 180
python3 scripts/evaluate_wiki_v2.py --arm "B=$BIN" --seeder "$SEEDER" --cases DW10 --repetitions 1 --root "$EVIDENCE/maintenance-DW10" --live --provider Codex --model "$MODEL" --effort medium --auth-file "$AUTH" --timeout 180
```

Pass `--stock-helper` and `--brn-helper` if verified helper artifacts are not BIN
siblings. Do not add `--scale`: M1 selects 1,000 fillers, DW9/DW10 select 100;
`--scale 100` is unsupported and `--scale 1000` would omit the maintenance cases.
Verify one nonzero sample per command and no diagnostic samples before dispatch.
No embedding-model download: report dense retrieval unqualified. The v2 runner
has a one-Run case boundary, so review each receipt before the next command.

## Receipt and acceptance

Report correctness, grounding, safety and recovery separately; lexical marker
checks and completed tools are not semantic proof. Preserve source bytes,
answers, traces, exact inputs, saved/pending records, operation lineage and
applicable Undo observations. Do not equate a source citation with completion
evidence. Record actual BRN launches separately from model turns/tool rounds;
record exposed input/output/cache usage, and actual cost if available, otherwise
unavailable. Do not invent cost from elapsed time or a rounded quota display.

Record all failed attempts, source and binary hashes, commands, admission and
stop conditions. No merge, release or deployment. Remaining Mac native visual,
focus, accessibility, picker and owner acceptance belong to a separately approved
session, not this headless proposal.

Command/source inspection at base `bc699c41`: guidance `run_arm`, structural and
source-check APIs; both fixture files; v2 parser/planner; CLI intake default;
historical dispatch driver and precise-edit scenario. All three evaluator/scenario
`--help` invocations succeeded locally without model or Mac execution. Reconfirm
these contracts at the final selected head before any separately approved run.
