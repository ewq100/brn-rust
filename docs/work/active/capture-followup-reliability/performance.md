# Measured startup, search and inbox recovery

The current bounded workloads do not justify a production optimization in this
slice. The existing 10,000-note witness passed, and the new opt-in profiles
preserved its correctness assertions while separating application open from
index preparation. Inbox replay retained exact accepted records without another
conversion. No production code, persistence checkpoint, privacy rule or recovery
guard changed.

Production baseline: `bc699c4164147354b96e433b6327e194275102b0` (merged PR229).
Measurement instrumentation: `e29e4776fe04d177640a935acd8b869f42160251`.
The [receipt](evidence/performance-20261026/receipt.json) contains exact commands,
source hashes, samples, environment and prior failures. These are cloud service
measurements, not Mac/native usability or live-model evidence.

## Results

Linux x86_64, AMD EPYC 9V74, four CPU quota cores, 16 GiB memory limit; Rust
1.98.1, repository test profile (`opt-level=1`, assertions/overflow checks retained,
SHA-256 at its existing `opt-level=3`), one Cargo build slot. Filesystem caches were
not flushed. Each row shows the range from two fresh synthetic datasets, not a
statistical confidence interval. “Cold index” means no pre-existing derived index,
not cold disk caches. No native embedding model was used.

| Notes | Workspace reopen | Index reopen | Cold index build | One-note sync | Query p95 |
| --- | --- | --- | --- | --- | --- |
| 1,000 | 4.46–4.77 ms | 6.42–6.79 ms | 134–144 ms | 0.69–0.73 ms | 7.06–8.08 ms |
| 5,000 | 13.97–14.90 ms | 28.54–29.65 ms | 714–776 ms | 0.80–0.87 ms | 37.71–38.29 ms |
| 10,000 | 29.75–41.92 ms | 48.73–53.38 ms | 1,628–1,701 ms | 0.83–0.98 ms | 65.02–90.14 ms |

Each dataset exercised exact current search versions, saved-over text exclusion,
archive exclusion, paging and an index reopen faster than a rebuild. Query p95 is
computed from 122–126 exact-term/common-term queries per dataset. Fresh empty
Workspace open was 1.52–1.80 ms; this does not include native window construction.

Inbox measurements deliberately isolate the host scan/import/recovery route with
a deterministic text-conversion fixture. They do not time Xberg, Office/PDF
conversion or any model. All folders and files were generated for the test.

| Files | Interrupted scan | Resume after reopening | Exact replay after reopening |
| --- | --- | --- | --- |
| 16 | 6.2–7.2 ms | 12.0–12.9 ms | 6.8–7.4 ms |
| 64 | 41.8–47.5 ms | 76.0–105.3 ms | 62.7–97.7 ms |
| 256 (configured cap) | 354.6–394.1 ms | 1,000.1–1,042.7 ms | 825.8–965.0 ms |

Cancellation occurs halfway through conversion callbacks, before the in-flight
item commits. Reopen/resume converts only unfinished items. A second reopen and
scan must perform **zero conversions**, preserve all accepted records exactly,
retain exactly one protected General Note per source body and leave every original
file byte-for-byte unchanged. This is deterministic cancellation plus reopen,
not a new process-kill qualification; the existing killed-worker and crash-after-
commit tests remain necessary.

## Checks and preserved failures

- Unchanged production-baseline 10k witness: **1 passed**, 6.55 s test time.
- Explicit expanded profiles: **2 passed**, 12 total datasets; inbox 5.12 s and
  startup/search 21.42 s. The ordinary 10k test was filtered out in this profiling
  command because it was already measured separately. Normal CI retains it;
  the two repeated profiles are explicitly opt-in, avoiding a repeated matrix
  in every CI run.
- Strict targeted Clippy, workspace formatting and diff checks passed.
- First wrapper failed before Cargo because `/usr/bin/time` was unavailable.
  Shell builtin `time` replaced only the wrapper.
- The first compiled baseline run failed before fixture creation because the
  configured synthetic temporary directory did not survive environment restart.
  Creating `/tmp/brn-daily-fixtures` fixed that environmental prerequisite; the
  unchanged baseline then passed. The failed log is preserved. Compilation took
  4m28s; it is not included in the reported product timings.

These results do not erase the historical credential-boundary test failures or
claim release/Mac strict-bound qualification. The 10k index-reopen range crosses
50 ms on this host; only a pinned release build on the target Mac can assess the
existing strict acceptance bound. No bounds were changed to make this run pass.

## Recommendation

Keep the existing durability and exact replay checkpoints. The observed inbox
cost grows faster than file count, but the host route remains about one second
at its 256-file cap in these tiny fixtures; that alone does not justify removing
ledger writes or weakening race checks. Profile a demonstrated slow real
operation only under separately approved data scope. Likewise, the new search
baselines support future same-machine comparisons rather than speculative cache
or storage rewrites.

Reproduce using the commands in the receipt, with an existing writable synthetic
`TMPDIR`, one build slot and no live provider. Run the opt-in profiles with
`--ignored --nocapture --test-threads=1`; keep the normal 10k correctness witness
in ordinary CI. Native window startup, actual converters, larger document bodies,
RSS/peak memory, target-Mac release timings and owner-perceived responsiveness
remain unmeasured here.
