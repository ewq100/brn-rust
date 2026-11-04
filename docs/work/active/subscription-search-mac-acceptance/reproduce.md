# Synthetic reproduction reference

This is a manual reference, **not authorization or an automatic runner**. The recorded session is closed. A new live session, including another search, requires a fresh explicit owner bound and admission; do not reuse its allowance. Stop on the applicable failure or capacity boundary. Never replay against an existing vault.

Use the [source and binary identities](observations.json), pinned toolchain/lockfiles and qualified sibling document helpers. Commands below were checked against the tested CLI contract. Replace placeholders only with IDs returned by the new synthetic workspace. Every Apply is a separate review decision; do not automate it from model summaries.

## Preparation (no model call)

```sh
BRN_REPRO_CLI=/absolute/path/to/qualified/brn
BRN_REPRO_DATA=$(mktemp -d /private/tmp/brn-issue243-repro.XXXXXX)
BRN_REPRO_FIXTURES=/absolute/path/to/fresh/copied/synthetic/fixtures
BRN_REPRO_AUTH="${CODEX_HOME:-$HOME/.codex}/auth.json"
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" init --purpose test
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" configure --provider Codex --model gpt-6.1-sol --effort medium --helper off --auth-file "$BRN_REPRO_AUTH"
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" document-conversion --converter xberg --ai off
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" delegation --choice review-first
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" note-create --title Sentinel --text 'Synthetic sentinel — Café Ω. Preserve these exact bytes.
'
```

Copy only the authored [fixtures](fixtures/baseline-hashes.json) outside Git ancestors and compare their hashes. The auth variable locates the normal managed login; never read, copy or publish its contents. Check normal supported login/model/allowance metadata without a model turn before any new authorized dispatch.

## Intakes and reviewed readback

Run each command separately, inspect its actual candidate bytes/inputs/Links and fresh-process records before advancing. The failed search is not a prerequisite replaced by these separate Rig workflow Runs.

```sh
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" --timeout 180 intake --source "$BRN_REPRO_FIXTURES/workshop.docx" --intent useful --title 'Willow workshop planning brief' --model-call-limit 12 --trace full
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" candidates
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" candidates # Inspect the matching operation in the returned request bytes.
# Only after reviewing the first proposal:
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" apply --operation FIRST_PROPOSAL_ID
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" read --id PLANNING_NOTE_ID

"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" --timeout 180 intake --source "$BRN_REPRO_FIXTURES/clarification.md" --intent useful --title 'Willow workshop clarification' --model-call-limit 12 --trace full
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" candidates
# Inspect the second proposal and both Sources before approval:
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" candidates # Inspect the matching operation in the returned request bytes.
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" apply --operation SECOND_PROPOSAL_ID
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" read --id PLANNING_NOTE_ID

"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" --timeout 180 intake --source "$BRN_REPRO_FIXTURES/workshop.docx" --intent useful --title 'Willow workshop planning brief' --model-call-limit 12 --trace full
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" records
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" candidates
```

Compare canonical Note/Action/Link values before and after the repeat; conversation/Run/source-observation metadata is separate. Check Sentinel bytes and null Action completion evidence after each step. Each CLI invocation is a fresh process.

## Referenced draft and exact edit

Save this exact synthetic draft prompt to a fresh UTF-8 file, replacing only `PLANNING_NOTE_ID`:

```text
Create exactly one UNSENT draft Note titled 'Willow workshop — draft invitation' using the current saved planning Note PLANNING_NOTE_ID. First read its current content and inspect its linked Source records. Do not claim to have reread original documents when only saved metadata or knowledge is available. Include traceable source Links to the planning Note and its two actual Sources. Preserve all material quantities, tentative date and venue status, room and repair exclusions, kit delivery status, and the named follow-up owner/date without claiming completion. Make the synthetic nature clear. Keep this a draft for review, never send or publish it. Do not change the planning Note, existing Action, or Sentinel. Prepare the draft for owner review under ReviewFirst.
```

```sh
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" --timeout 180 ask --thread INTAKE_THREAD_ID --prompt-file /absolute/fresh/draft-prompt.txt --budget 12 --model-call-limit 12 --trace full
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" candidates # Inspect the matching operation in the returned request bytes.
# Review exact facts, source Links and versioned inputs, then:
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" apply --operation DRAFT_PROPOSAL_ID
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" read --id DRAFT_NOTE_ID
```

Before the edit, verify the new draft has exactly one standalone `Hello,` greeting at version 1. A fresh model result may differ; if that precondition fails, stop this recipe without spending another call or inventing a match. Save the following edit prompt to a fresh UTF-8 file, replacing only `DRAFT_NOTE_ID`. Use ordinary `ask`, without an `--edit-note` write grant:

```text
Prepare one exact passage edit for ordinary draft Note DRAFT_NOTE_ID at its current version 1: replace the standalone greeting 'Hello,' with 'Hello neighbours,'. Use prepare_note_edit for only that unique passage. Preserve every other byte, title, source Links, provenance and tentative facts. This is a proposal only: do not apply it, send anything, or change any other record. The owner will review and decide separately.
```

```sh
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" --timeout 180 ask --thread INTAKE_THREAD_ID --prompt-file /absolute/fresh/edit-prompt.txt --budget 8 --model-call-limit 12 --trace full
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" candidates # Inspect the matching operation in the returned request bytes.
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" read --id DRAFT_NOTE_ID
```

Verify the unique replacement and every other byte before any mutation. To reproduce the negative path, write the exact saved draft plus `\nOwner correction: drafting is paused pending venue confirmation.\n` to a fresh UTF-8 file. Then perform the explicit synthetic owner Save:

```sh
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" begin-edit --note DRAFT_NOTE_ID --base 1 --token-out /absolute/fresh/new-session-token
# Use result.id as EDIT_SESSION_ID; never print the token or put it in argv.
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" update-buffer --session EDIT_SESSION_ID --generation 1 --text-file /absolute/fresh/owner-correction.txt --token-file /absolute/fresh/new-session-token
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" save --session EDIT_SESSION_ID --generation 1 --token-file /absolute/fresh/new-session-token
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" apply --operation EDIT_PROPOSAL_ID
# Expected exit 3 / Stale: do not repair or force it to pass.
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" dismiss --operation EDIT_PROPOSAL_ID
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" read --id DRAFT_NOTE_ID
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" read --id SENTINEL_ID
"$BRN_REPRO_CLI" --json --data-dir "$BRN_REPRO_DATA" read --id FOLLOWUP_ACTION_ID
```

Require exact owner content, unchanged Sentinel and null completion evidence. Do not treat these headless checks as native acceptance.

## Separate search reference

After a **new explicit search authorization**, the existing feature-gated example accepts:

```sh
/absolute/path/to/qualified/subscription-search --live-public-query gpt-6.1-sol 'Search the official SQLite documentation: what does atomicity mean for a transaction? Answer in two sentences and cite an official sqlite.org source.'
```

It uses medium effort, a 60-second total deadline and no automatic retry. The recorded first attempt failed with an unknown triggering event; the corrected binary has not been live-qualified. Building or documenting it makes no model call and does not reopen the closed receipt.
