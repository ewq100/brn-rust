# MAC-13 corrected runner review

**CHANGES REQUIRED: two remaining P2 bound inconsistencies. Do not execute this identity yet.**

Reviewed the entire actual 1,015-line runner SHA-256 `ae66bc3b9f289862c4168b7818ede962ee6cf23840922dfc764f359ab97f1dc7` and corrected implementation report `843f5f259c8ac73d3bf25759487208b10135de7776f0012c61fbd95d0aedc30f`. The original `eb837f36` candidate and `/tmp/brn-mac13-preparation/runner-review.md` remain preserved; the complete original source/API/fixture/artifact assessment is explicitly reused where unchanged. Reviewer authored neither runner nor correction and performed only read-only inspection. No runner/CLI/build/workspace/test/fixture/signal/native/account/network/helper action occurred. This report is the sole write.

## Prior finding dispositions

- **Environment ordering: valid/fixed.** Fixed allowlist is assigned before manifest names are evaluated (`196–220`).
- **Nested Settings admission: valid/fixed.** `598–616` requires exact ready/data, a nested Settings object, explicitly present null credential sources and explicit Xberg/AI-false fields. No missing-key success defaults remain.
- **Leader-independent cleanup: original escape valid/fixed; lifetime accounting remains incomplete below.** Every created CLI/Git process enters cleanup regardless of leader return code. Individual operational errors no longer prevent independent KILL/reap/final-probe attempts; errors/unknown state cannot qualify success.
- **Receipt failure success: valid/fixed.** Main exceptions set local failure status; secret cleanup errors fail; final persistence is required for zero exit, and fallback persistence retains failed status (`953–1011`). Unique atomic temporaries avoid a prior failed temporary blocking the failure receipt.
- **Unbounded Git subprocess lifetime: original calls valid/fixed; accepted-read handling remains incomplete below.** All Git calls now use the owned-group five-second/remaining-budget runner. No `subprocess.run` remains.
- **Advisory addressed:** initial general Notes IDs are compared with the expected set (`679–681`); rehearsal history retains every exact old receipt and permits only the one Save addition (`809–816`). These strengthen the selected claims without product tests or a new framework.

## Remaining required corrections

**[P2] Reserve the actual cleanup budget and avoid repeating completed cleanup — `105–179,239–246,306–309,421–464`.** The helper can wait TERM 10 seconds, KILL 10, reap 1, independent final KILL 10 and reap 1: up to 32 seconds before reader joins. Callers may invoke the complete sequence again from `except` after an already completed failed cleanup; CLI readers add further bounded joins. `ensure_budget` still reserves only 21 seconds. A command admitted near the overall limit can therefore outlive the declared 600-second preparation cap despite following every branch normally. The error paths are exactly where this bound is meant to apply. Use one shared absolute cleanup deadline that all cleanup/reap/join attempts consume, or reserve the actual finite worst case and avoid rerunning a completed sequence. Keep independent best-effort cleanup/error recording; no framework or new product test is needed.

**[P2] Do not fully read rejected oversized Git output — `run_git:285–305`.** The size guard detects combined output above 1 MiB and marks failure, but then `read_bytes()` loads both complete files unconditionally. Consequently an overflow can still cause an unrestricted memory read before rejection. Known source queries are normally small; that does not make their implemented overflow branch bounded. Reject before reading, or retain only a bounded prefix for decoding while recording actual sizes/overflow and hashing by streaming if needed. Regular-file output plus sampled stopping is not a hard live disk quota; keep that limitation explicit. Adding explicit `--no-ext-diff`/`--no-textconv` to the relevant Git diff invocations is a small deterministic-query refinement, not a separate product finding or reason for broad redesign.

## Complete-candidate scope assessment

The corrected runner otherwise preserves the approved four-root plan, canonical exclusive synthetic parent, independently hashed input bytes, primary/rehearsal separation, exact Note metadata and unrelated-record preservation, supported token-held checked writes, source/CLI/helper binding and ordinary JSON shapes. Frozen artifact/build receipt evidence from the initial review remains applicable and is not reinterpreted as native qualification. No production/dependency/configuration change, owner-data root, provider/Run/native action, direct SQL, old-manifest rewrite or automatic retry was introduced.

After these bounded corrections, freeze the exact complete identity and refresh affected review before one selected preparation run. Preserve failed/partial receipts. Actual native opening, stale-banner reachability, SaveCopy refusal/public success, general search/export and owner acceptance remain pending regardless of headless preparation.
