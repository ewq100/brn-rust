# Public backup interruption: independent actual-result review

**Qualified for the selected single synthetic frozen-CLI backup interruption and one public retry. No outstanding qualification finding.** Reviewed actual `/private/tmp/brn-public-backup-interruption-20261022T0444/receipt.json` (run ID `067c4a5a-142d-4967-9a98-e90d50054f02`) and `/tmp/brn-public-backup-interruption/run.log`, the exact approved driver, artifact/build receipt and main208 whole-tree proof. This is independent post-execution assessment, not a rerun. The lead reports actual process exit0; the saved run summary and receipt both report `qualified_partial_backup_retry_and_exact_data_checks_passed` with receipt persistence successful, consistent with the approved driver's zero-return path.

The reviewer authored neither driver nor execution, and invoked no BRN CLI, workload, build/test, network/CI/account/provider/native action or nested helper. Existing synthetic files were read/hashed and SQLite was queried read-only with `mode=ro&immutable=1`; the source WAL was first confirmed empty, so those independent reads consume only the retained main DB bytes. An entire-root file bytes/modes inventory was equal before/after the independent validation. This report is the sole write; no repair, checkpoint, SQL mutation, new fixture or repeated interruption occurred.

Actual original receipt SHA-256: `e58b6c9d94778a3445f01f9e5080c5366632b76a1bc92a6b876c1af1985582c5`; run-log SHA-256: `31f916f873cc454b287034f8726be385ee32ffbea9fa9cce52ac294bbbaf8983`.

## Exact identity

- Driver still hashes to approved **`9f515e706cca1204c7ff5de0a65160b594f647f2a5b3965fe2552e3df24be675`**. Reused the full independent source disposition in `/tmp/brn-public-backup-interruption/final-review.md`; its three original P2s remain valid/fixed.
- Frozen ordinary CLI actually hashes to **`edf780009231fb887ef9db15d9e34e9bb1557caa914976e1675b8f80de412301`**. Receipt SHA remains **`6416208acd32804d8f76bf38daf501b2723e8efb22ca419f2b90a3e185ea3a70`**, binding source **`7dde6a41a4fbc66468e0400d2cc3c859f04c8ef5`**, the exact artifact path, 71,484,016 bytes and the successful pinned ordinary offline build.
- The actual run source proof records that exact source/clean runtime closure/tree and binary hash, plus reviewed imported harness SHA **`17fa8390811d961db2940dd95582b3e16cb597b57aad97195a069905d64f4cfd`**.
- Re-read lead `pr208-integration.json` and independently compared local Git trees: source7dde and actual main208 **`11c48f9b91cb0d4e9deba9e0f45b43ef284b63e1`** both name tree **`0a3e4464a9e9e1f413177dd2c740f82bcd36e2aa`**, with empty whole-tree diff. This establishes source equivalence to the recorded merge, not live hosted/resulting-main check status.

## Actual interruption and public retry

The receipt contains exactly five public commands in order: test init, Note creation, earlier complete backup, one interrupted backup, one retry to the same requested destination. The four ordinary commands report exit0, no timeout and complete bounded capture. There is one interruption record, not repeated observation attempts or fixture scaling.

The interrupted child's recorded observed and post-kill stage are the **same path**:

`backup-output/.brn-backup-stage-3bbdcf10-1524-4407-b24f-7980f2931f33`

Both samples contain `snapshot.sqlite3` **1,634,304 bytes**, `.brn-stage` 20 bytes and rollback journal 1,024 bytes; no stage WAL is listed. Recorded copy bytes therefore come from the DB itself, not a journal-size proxy. The complete baseline is **34,222,080 bytes**. The retained post-kill copy is greater than 1 MiB and strictly smaller than that complete snapshot. This is actual post-kill partial-copy evidence, not a timing-only inference.

The owned child records one signal attempt, signal9, **actual returncode -9**, **reaped:true**, elapsed0.075 seconds, and complete empty stdout/stderr drains. The approved control flow requires the requested final path absent after reap and retained same-stage partial proof before reaching the sole public retry. That absent-final gate is control-flow evidence from the exact approved driver; the reviewer did not witness the transient filesystem state live. The post-kill stage is now absent, as expected after the successful public retry; there was no driver-private sweep or stage setup between kill and retry in the approved source.

The retry reports exit0 in0.361 seconds, complete capture, no remaining backup stages, and a published regular output. Independent current inspection found no `.brn-backup-stage-*` entries. The receipt's final outcome is reachable only after these interruption gates and the following exact validations.

## Independent retained-data validation

Independent read-only validation passed integrity checks and exact whole-snapshot equality for source, earlier complete backup and retried backup. All non-internal schema and **all rows of 11 tables** match without normalization. Table counts: operations3, receipts3, records3, revisions3, workspace_profile1; candidate_targets, chat_drafts, derivation_inputs, edit_session_bindings, edit_sessions and receipt_actors are empty. Actual workspace purpose remains `test`.

All three independently recomputed logical digests equal the recorded **`f8dff2e93a69e24faee89af7e78fdeabe4207ae1834773fe38e29f90244f6e43`**. The unique titled Note is exactly the deterministic multilingual pattern, **8,388,548 UTF-8 bytes**, SHA **`914e092cae630d083d2fe506e834e4a095ac8d443874188d2001c7d6728c1f1d`**. This checks actual content and historical/operation rows, rather than relying only on the receipt's Boolean success fields.

Both current backups are **34,222,080 bytes**, with equal actual hash **`ec3dbb901d76f0be09b2f336f297dc767f1954a29cf38b11a65945190c207274`**. The earlier backup hash equals its pre-interruption baseline. Both have SQLite DELETE-format read/write header bytes and no companion WAL/SHM/journal. The actual receipt additionally records read-only `journal_mode=delete` and no companions before/after each backup validation, including the earlier backup recheck. Independent immutable reads confirm all logical data comes from the main files themselves.

Source main DB size/hash equals both recorded before/after identities: **34,222,080 bytes**, **`a092ad150e6af1ef5d2f74d41ea96d90e2393eb714ed43a9e56dba36e5dcb28d`**. The workspace marker hash, sibling sentinel hash and unrelated directory's retained file bytes/mode/hash all match their baselines. The receipt records equality of the complete unrelated tree, and current inspection contains its expected `keep.txt` only. Source WAL is currently zero bytes; SHM is present as recorded. Sidecar identity is diagnostic rather than an acceptance requirement. The temporary Note input is absent as intended.

## Capture and bounds

All ten stdout/stderr prefix files exist with exactly their recorded saved lengths, each at most64 KiB. All streams record complete:true, thread_alive:false and command-level output_complete:true. Full retained streams' hashes were independently recomputed. The Note-create response emitted8,512,594 bytes but retained only65,536; its full-stream count/hash is driver drain evidence and cannot be independently recomputed from the deliberately bounded prefix. No large body was printed during this review.

The receipt records102,766,061 task-root bytes before writing its own12,505-byte receipt; independent current total including the receipt is **102,778,566 bytes**, below512 MiB. Both outputs are below128 MiB, Note below8 MiB, all ordinary commands below60 seconds and the single observation/kill below ten seconds. The initial2 GiB free-space admission is recorded by the source gate, not independently reconstructible after execution. These are accepted-size/checkpoint bounds, not a hard live OS quota.

## Limits and disposition

Retain the exact original receipt, run log and bounded captures. This qualifies one actual killed partial Online Backup and one public same-destination abandoned-stage sweep/retry using the frozen source-equivalent ordinary CLI, with exact retained data and sibling preservation. It supplies a meaningful distinct partial-copy witness alongside the existing full-stage process test; it does not prove every interruption offset or permit another attempt.

No power-loss/hardware reliability, native UI usefulness, owner acceptance, live provider/account behavior, current hosted/main status, release readiness or broader integration result is inferred. Transient SIGKILL/absent-final/stage observations are supported by the exact reviewed driver's receipt and gates, not a second external observer. The retained artifacts and independent read-only comparisons support the bounded qualified verdict above.
