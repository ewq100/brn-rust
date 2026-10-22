# MAC-13 final runner disposition

**APPROVED exact runner SHA-256 `59e9bd0d8a20095f51693899589bd8873f2170a5345a6f3168cab9b1a0a77397` for one selected bounded preparation attempt. No remaining actionable findings.** This is independent static approval, not execution or fixture qualification.

Read the complete actual delta from preserved `ae66bc3b9f289862c4168b7818ede962ee6cf23840922dfc764f359ab97f1dc7`; verified that preserved gzip decompresses to that identity. Delta text SHA-256 is `6f540cb3ddfe8f74d415011c64f8e64d24886261bf92a90e9bbb50419dffa157`. Reviewed complete affected cleanup, admission, CLI/Git exception/output paths and their integration with unchanged finalization. The final refinement from `aefa0347` is only the reserve changing 25→30 and its corresponding explanatory comment; normalization independently reconstructs that preceding hash.

Both remaining P2s are **valid/fixed**:

- Each created Popen stores one absolute `_mac13_cleanup_deadline`, established once with a 22-second budget. TERM/KILL/final-KILL observation and both reap attempts consume its remaining time; later calls cannot reset that budget. Polling reaps the leader while checking descendants. Operational errors remain recorded and unknown/unresolved ownership fails. The 30-second admission reserve now covers 22 seconds of shared cleanup, both possible reader-join passes (four seconds), the short post-leader observation and margin. CLI and Git overall checks use the same reserve.
- Git output sizes are checked after cleanup before reading. Overflow retains size/ownership/error metadata and raw files but raises before read/hash/decode. Defensive reads are capped at MAX_OUTPUT+1 per stream with a combined postcheck before decoding/hashing. Explicit `--no-ext-diff`/`--no-textconv` prevent those extension paths. Regular-file capture uses sampled stopping; it does not establish a hard live disk quota.

The complete initial review and corrected-candidate review remain preserved and are explicitly reused for unchanged CLI/service/artifact/fixture evidence. All five original P2s and the general-projection/history advisory now have supported dispositions: environment ordering, actual nested fail-closed Settings, leader-independent cleanup, persistence-dependent success, finite Git checks, exact general ID sets and retained prior receipts. No unchanged service/build qualification was repeated here, and no further abstraction/test matrix is needed.

Execution must retain the approved fresh canonical four-root ownership, frozen CLI/helper/source verification, primary-versus-rehearsal separation, task-local token secrecy, no automatic retry, durable failure capture and stop on unresolved cleanup. Only public checked CLI setup/rehearsal is approved. Original MAC-01 roots/manifest and shared lead facts remain preserved. Actual native opening, stale-banner reachability, SaveCopy behavior, search/export and owner acceptance remain pending.

Reviewer performed read-only source/delta/hash inspection only. No runner/CLI/build/workspace/test/fixture/signal/native/account/network action or helper occurred. Only this report was written.
