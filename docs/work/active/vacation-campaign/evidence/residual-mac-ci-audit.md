# Residual Mac CI bottleneck audit

Read-only review of archived PR189 head run `39465475559` and resulting-main run `39469206851`, plus the existing cache investigation and current workflow/cache source. Two GitHub API run-metadata calls were used to capture `created_at`/`run_started_at`; the current in-progress PR190 run was not queried or polled. Detailed selected job/step timestamps and durations are in [the retained metadata](residual-mac-ci-audit.json).

## Queue and job time

Run creation and workflow start were the same timestamp for both runs (0 seconds at workflow level). Per-job queue below is job `created_at` to `started_at`; job duration is `completed_at` minus `started_at`, excluding this queue interval.

| Run | Mac Core/CLI queue → job | Mac Retrieval/Combined queue → job |
|---|---:|---:|
| PR189 head `57531c3` | 9s → 16m13s | 6s → 5m11s |
| Resulting main `a23f486b` | 21s → 17m36s | 14s → 7m58s |

Both selected Mac jobs succeeded in both runs. The workflow-level receipts say failure because optional Windows checks failed; the required contexts passed. These are different source commits and different hosted runner IDs (3320/3321 versus 3332/3333), so the increases on main are descriptive only, not evidence of regression or cache impact.

## Measured steps

| Lane | PR189 head | Resulting main |
|---|---:|---:|
| Core/CLI: default workspace tests | 5m37s | 6m07s |
| Core/CLI: helper API/restriction tests | 3m28s | 3m36s |
| Core/CLI: prepare both helper graphs | 2m23s | 2m35s |
| Core/CLI: provider capability fixtures | 2m22s | 2m18s |
| Retrieval/Combined: canonical search app tests | 1m32s | 2m23s |
| Retrieval/Combined: prepare both helper graphs | 1m02s | 1m21s |
| Retrieval/Combined: combined desktop state tests | 54s | 1m13s |
| Retrieval/Combined: native desktop build | 19s | 32s |

The Core/CLI elapsed time is led by the full workspace tests and helper API tests. Retrieval/Combined is led by its canonical search app tests, with much shorter native retrieval tests, builds and lint. The same named dual-helper preparation step runs in both Mac lanes: 3m25s total across those two jobs on the head run and 3m56s on resulting main. Workflow source shows that step fetches both locks, regenerates the adapted workspace, builds both helpers, copies artifacts and exports their paths. This is repeated work, but these receipts do not prove that either lane can safely omit it; tests may consume the helpers.

## Cache interpretation and disposition

The completed job receipts expose `Run ./.github/actions/setup-rust` as one composite step: 46s/58s in Core/CLI and 50s/89s in Retrieval/Combined (head/main). It also includes toolchain reporting, native dependency setup for the native lane, cache action, generated-source cleanup, dependency fetch and safe temp setup. The receipts do not provide nested cache restore/save durations or exact-hit/upload results for these four jobs. No cache hit or miss is inferred from that wrapper duration or job success.

The source configures separate Mac cache keys by lane (`default` for Core/CLI and `native-ui-native-retrieval` for the combined lane) and retains main-only cache writes. Existing [cache investigation](../ci-cache-investigation.md) already records the completed UI cache improvement, its same-source cold/warm evidence, and the limits on cross-lane sharing. It does not establish these Core/CLI or Retrieval/Combined run-specific cache outcomes. The helper graph step is a measured repeated operation, but no change is justified from this non-controlled pair: the cache hit state is unknown, runners and source differ, and cross-lane sharing is already a deferred boundary. No repeat workflow run is warranted solely to create activity. After the in-progress PR190 run completes, its existing logs can establish whether the two lane cache restores hit and whether helper preparation remains the useful next investigation.

## Evidence and limits

- Job/step receipts: `evidence/pr189-premerge.json` and `evidence/pr189-resulting-main.json` under `docs/work/active/vacation-campaign/`.
- Workflow/job definitions: `.github/workflows/ci.yml:54-123,231-271`; composite cache/setup action `.github/actions/setup-rust/action.yml:13-89`.
- Prior cache decision and evidence: `docs/work/active/vacation-campaign/ci-cache-investigation.md:79-88`, including the explicit no-controlled-comparison limitation.
- No source, cache, workflow, repository, or CI configuration was changed. The current PR190 run was not queried, and this report makes no claim about its live status or results.
