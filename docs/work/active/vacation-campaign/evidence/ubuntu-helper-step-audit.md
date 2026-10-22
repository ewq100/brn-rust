# PR219 Ubuntu helper-stage timing audit

Snapshot taken 2026-10-22 16:39:16 UTC. Read-only GitHub job metadata identifies PR219 head `24a77f4f44072a084423f71bca0356f140dcd968`, run `39811859007`, required Ubuntu job `119939956231`, started 16:11:35 UTC and still `in_progress`. Its latest observed steps:

| Step | PR219 | Prior passing PR head `e5` job `119927384267` | Delta |
|---|---:|---:|---:|
| Prepare both locked helper graphs | 10m41s (16:12:05–16:22:46) | 9m02s (15:49:13–15:58:15) | +1m39s |
| Default workspace test | 5m03s (16:23:23–16:28:26) | 4m27s (15:58:46–16:03:13) | +0m36s |
| Both helper normal API/restrictions | 7m01s (16:28:26–16:35:27) | 5m49s (16:03:13–16:09:02) | +1m12s |
| Offline provider capability fixtures | In progress since 16:35:27; 3m49s elapsed at snapshot | 1m48s (16:09:02–16:10:40) | At least +2m01s so far |

The earlier job completed successfully at 16:10:52 after 22m07s total. At this snapshot PR219 had run 27m41s, about 2m19s before the stated 30-minute job budget; **no timeout or failure had occurred**. The helper stages had completed successfully; the currently active step was the later offline provider fixture test.

## What the commands show

The PR219 versions of `.github/workflows/ci.yml`, `scripts/build-document-helpers.sh`, and `scripts/prepare-xberg-adapted.py` match the local inspected versions (file blobs `7e746fa0…`, `91276b77…`, `e9fea575…`). The workflow's “Prepare both locked document helper graphs” step performs locked Cargo fetches for both manifests, materializes the adapted workspace, then calls the offline helper build script (`ci.yml:80-92`). The script compiles the stock and adapted helper binaries into separate per-mode targets (`build-document-helpers.sh:14-42`). The later helper test step points each mode's `CARGO_TARGET_DIR` at that same target and runs `cargo test --all-targets` plus `cargo clippy --all-targets` (`ci.yml:102-110`), so the declared target layout is intended to reuse each helper target.

There is one concrete repeated preparation: workflow line 86 invokes `prepare-xberg-adapted.py`, and `build-document-helpers.sh:14` invokes it again. The preparer revalidates/extracts the stock archive, reapplies and hashes the accepted patch set, then, because the generated output already exists, validates its manifest/source/lock instead of replacing it (`prepare-xberg-adapted.py:72-135`). This is duplicated verification work, but step metadata cannot show how much of the 10m41 it consumed. The workflow does not expose per-command timings.

The PR219 helper-preparation step is 99 seconds slower and the helper API/restrictions step 72 seconds slower than the passing comparison. Metadata alone does not show cache hits/misses or distinguish compilation from test/clippy runtime. The Rust setup step itself was 22 seconds versus 21 seconds; that is not evidence of a cache miss. I made one bounded request for the current job log endpoint, but `gh` refused to emit the response because it contained terminal escape sequences; I did not retry or retain its empty output. There is therefore no raw compiler/cache log evidence here.

**Disposition:** this snapshot supports a timing increase in both helper stages and a still-running later fixture step, not a confirmed timeout or cache/build failure. The repeated adapted-workspace preparation is a real command-level duplication but is not established as the cause of the duration delta. Keep the run outcome pending; no optimization or workflow change is justified by this packet alone. No CI rerun or source change was made.
