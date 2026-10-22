# Independent audit-continuation adapter review

**CHANGES REQUIRED before the selected launch: one P2 input-provenance gap.** No adapter/audit/build/workload was executed. Reviewer authored neither adapter nor configuration. This review does not alter the approved product candidate or claim any remaining scan result.

Exact full files inspected:

- `/private/tmp/brn-advisory-audit-20261022/continuation-adapter.py`, 586 lines, SHA256 `35d617b1561435a6b9d743289a87ed1807f87bd1ae02b82d5d9131290808de44`.
- `continuation-config.json`, SHA256 `da95fa53e685ee0df8b06e4e754714538599044b3619177d3dd4d185cabb2d91`.
- Author's `continuation-adapter-review.md`, prior packet/source rationale, actual pinned cleanup helper and retained evidence.

## Required correction

**[P2] Bind the actual copied audit inputs to the proven hashes — adapter lines 513–519 and 542–554.** `source_proof` validates bytes from one read, but main performs a separate unchecked `read_bytes()` to write the snapshot later passed to `cargo-audit --file`. The snapshot itself is never compared with the recorded proof before or after auditing. Final lines 558–563 check only source checkout files, then assert `source_lock_hashes_unchanged`.

A source changed between the verified read and the copied read, then restored before final source checks, can therefore yield a successful same-count audit of different bytes while the result claims the pinned original input hash. The source freeze reduces that race's likelihood; it does not bind the actual `--file` bytes. This is a concrete gap in the expressly selected exact-input provenance contract, not a demonstrated corrupted current fixture.

Small correction: reuse the already verified bytes, or compare the copied bytes with the proof's exact size/hash, then verify the saved snapshot's exact size/hash immediately before and after each corresponding scan. Keep the actual snapshot hash in the result/postcheck evidence. Stop on mismatch; do not silently replace or repair a snapshot. No new framework or repeat baseline is needed.

## Other reviewed behavior

The fixed input identities currently match actual files: final candidate `234956cb171beb025acb1dc17b41be98992bddf0`, root lock SHA256 `f6d7419899f81f4e28eaae198475e69067ac34d174c5293a0f9d13e012fbc01a`, 249738 bytes/943 packages; stock/adapted helper hashes `4d375d6c65cc56a573203fa838f0d42d49e9d6764487bf56adb5b05006e32f36` / `9e3fa02dfa4533233ffdba0338829d01a1ebe767ccdb47d01da4d581a040f508`, 409 packages each. The three explicit inputs are the only audit loop; baseline is parsed in place, never dispatched. Superseded 003 remains a distinct preserved/unscanned input. Output is exclusive `packet-continuation`; it did not exist during inspection.

The current task DB is clean at `59ef7abf8ce56ea80fe7eba97ba3835449009fa4`. Independently enumerated its 1349 tracked record files and confirmed all current bytes match their Git blob IDs and filesystem path set. Adapter's initial/before/after DB inventory algorithm checks HEAD, clean status, exact tracked paths/types and per-record SHA256/size/Git blob; all seven inventories must agree. Parsed advisory count 1348 is correctly distinct from record-file count 1349. A conflicting nonnull report commit fails; expected null metadata is externally bound. No record repair/fetch or shared DB mutation is performed.

Original baseline receipt hash `b0f58c7f769448d2e531b47ebfa4d9518c7004c6571a171a55fc9820e122b69d` matches the pinned retained failed receipt. Its actual stderr file is currently empty; stdout hash, seven findings/eight notices, dependency count, input and flags remain correctly validated. Advisory findings are retained, not retried or reclassified as operational failure. Original failed packet status remains unchanged. The new packet's `complete` means all three scans/capture/provenance/postchecks completed, not no vulnerabilities or native acceptance.

The existing binary hash/version and cleanup helper hash are pinned and currently match. Environment preserves inherited HOME/CODEX_HOME, redirects task-local Cargo state, uses explicit offline/no-fetch/no-yanked, and supplies no provider/account credentials or target/ignore/severity filters. Shared DB is read only to compare HEAD. No install/download/build or native launch exists in the adapter.

The command runner caps combined captured stdout/stderr at 1 MiB, records raw hashes/status before interpretation, owns each process group, invokes previously reviewed absolute 22-second cleanup across repeated calls, joins readers and rejects signals, unknown/live groups, capture failure, unexpected exits and malformed/inconsistent report summaries. Commands are at most 60 seconds (Git 15/20) within a 300-second admission/30-second cleanup-reserve budget and 2/1 GiB disk floors. Synchronous hashing/persistence is not an OS-enforced wall-time or disk quota; it processes the pinned small local record/lock set. Initialization or persistence failure cannot return success; final persistence failure enters a failed/nonzero path. Exit 2 is captured then rejected, not accepted as a completed scan. The prior whole-packet retry history is preserved.

After the located correction, refresh review of affected paths and new exact adapter hash before one explicit continuation launch. Product final qualification remains a separate lead-owned gate. Only this report was written; no source/shared facts, audit inputs, prior packets or account/native state were changed.
