# Ubuntu required-gate cache audit

Read-only audit at checkout `59f036f1` (integrated product main `a23f486b`). No source changes, builds, CI/cache/account actions, or reruns.

## Recorded required-gate time

All observations are 20 Oct 2026. Durations are check-run start to completion and named step start to completion, rounded to the nearest second. Each listed Core and CLI result succeeded.

| Evidence / source | Core and CLI Ubuntu | Prepare both helper graphs | Workspace tests | Helper API/restrictions |
|---|---:|---:|---:|---:|
| `pr187-resulting-main.json`, main `4c50b614` | 23:47 | 8:13 | 6:10 | 5:06 |
| `pr188-resulting-main.json`, main `6622443e` | 18:53 | 6:21 | 4:55 | 4:11 |
| `pr189-premerge.json`, candidate `57531c3d` | 24:46 | 8:41 | 6:27 | 5:28 |
| Same PR189 candidate, macOS Core and CLI | 16:13 | 2:23 | 5:37 | 3:28 |

PR189's Ubuntu check took 8:33 longer than its Mac counterpart; helper preparation differed by 6:18. Across the three Ubuntu observations, helper preparation moved 8:13 → 6:21 → 8:41 and total time moved 23:47 → 18:53 → 24:46. These are separate CI observations, not a controlled cache experiment. They show current feedback cost and a possible helper-cache opportunity, but cannot attribute the timing changes to cache layout. Runner load, cache-hit state, and the rest of each job vary. PR189's receipt says fetched inputs preceded merge and identifies the candidate; it is evidence for that run, not a benchmark.

## What PR188 changed for Linux cache coverage

The complete net PR188 diff from `4c50b614` to `6622443e` is three files: `.github/actions/setup-rust/action.yml`, `.github/workflows/ci.yml`, and `.gitignore`.

Before PR188, helper `CARGO_TARGET_DIR`s were `target/document-helpers/{stock,adapted}`. Linux used the generic pinned `rust-cache` invocation whose default workspace is `.` and target is `target`. Thus helper build directories were physically below its target path, but not configured as helper workspaces. The pinned action's `src/config.ts:144-150,274-277` confirms that only configured workspace targets become cache paths. Its `src/save.ts:39-53` cleans each configured target with that workspace's package list; `src/cleanup.ts:10-36,62-79` descends into nested profile directories and removes dependency/build outputs outside that list. So the old layout did not guarantee helper artifacts survived save: it could retain artifacts whose crate names overlap the root graph, while helper-only outputs (including the helper binary) were pruned by the root graph. It was inclusion in a broad root path, not a helper-graph cache.

PR188 moved helper target roots to `$GITHUB_WORKSPACE/.brn-ci-document-helpers/{stock,adapted}` in all three Linux/macOS CI lanes (`ci.yml` headless, native-ui, native-retrieval; `.github/actions/setup-rust/action.yml` lines 65–66). Linux still runs only generic cache: its workspace target is root `target`, so the new helper targets are outside every cache target path. Cargo registry/git are still cache paths, but save cleanup receives only root workspace packages; helper-specific registry source/archive entries are not guaranteed retained either. The workflow's locked helper `cargo fetch` precedes offline builds. `.gitignore` now excludes `.brn-ci-document-helpers`; that protects Git status but does not add it to cache coverage.

macOS has a separate action invocation (`setup-rust/action.yml:57-70`) that explicitly configures root `target` plus both external helper targets as workspaces, with a cache key covering helper locks/configuration, build identity, preparation script and accepted patch/source-manifest inputs. The same source root is used for each helper target, but the targets are disjoint. Linux does not use that multi-workspace mapping or key extension. The earlier [`warm-helper-cache-audit.md`](warm-helper-cache-audit.md#follow-up-disposition-overlap-makes-the-proposed-mapping-unsafe) correctly warns that placing helper targets inside root `target` while also adding them as cache workspaces would let the root cleaner prune them first; PR188's actual external path avoids that overlap on Mac.

## Bounded options

- **Defer Linux helper caching.** This matches the stated Mac-first priority and avoids new cache uploads while storage headroom and retention remain unknown. Cost is keeping an observed 8:41 helper-preparation step and 24:46 Ubuntu required check in this PR189 run; those numbers are not a future guarantee.
- **Add a Linux helper-graph cache if faster Ubuntu feedback is selected.** Concrete path: preserve `.brn-ci-document-helpers/{stock,adapted}` outside `target`, and configure separate helper workspace targets and helper identity inputs for Linux as Mac already does. This would give the cache cleaner the helper package graph and could retain compiled helper dependencies. It requires validating the runner-relative targets/key and accepting additional per-OS/lane cache volume. The Mac main publication receipt [`ci-main188-cold-mac-ui.json`](ci-main188-cold-mac-ui.json) records a successful 1,902,323,274-byte upload; the configured storage limit was unavailable (HTTP 402), and the receipt expressly does not establish retention or warm speed. Do not infer that another OS/lane archive is free or that it will produce a speedup.
- **Restore Linux helpers under `target/document-helpers`.** This is a smaller workflow reversal with no distinct helper cache, but the pinned cleaner prunes using the root package graph, so it cannot be presented as a reliable way to preserve helper-only compiled artifacts. It would also leave helper pin/patch identity outside the explicit Linux key inputs. It offers no evidence-backed timing improvement over deferral.

Lead disposition: defer a Linux cache change under the existing Mac-first priority until comparable hit/miss evidence and a bounded cache-volume/retention assessment justify it. Existing owner authority permits a later in-scope decision; no additional approval gate is inferred. No platform outside the existing required Ubuntu and Mac checks is proposed.
