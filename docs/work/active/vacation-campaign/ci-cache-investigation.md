# Mac CI cache investigation — 20 October 2026

Current gates and next action belong to [campaign state](campaign.json) and [the overview](../../overview.md). This record retains the investigation, tradeoffs and source-bound observations. [Raw receipt](evidence/ci-cache-investigation.json), [cache analysis](evidence/ci-cache-causes.md) and [compile analysis](evidence/ci-compile-reuse.md) contain reproducible sources and limitations.

## Measured result

The same main source `20fcd738` took 45m08s with no cache returned (job cancelled at its 45-minute limit after functional checks passed), then 14m46s on a successful retry that restored the exact cache. This one pair establishes that cache reuse matters; runner conditions also differ, so it is not a controlled performance benchmark.

| Mac Core work | Cold run | Exact-cache retry |
|---|---:|---:|
| Both converter builds | 14m59s | 4m03s |
| Converter test compilation and lint | 10m10s | 3m23s |
| Complete job | 45m08s, cancelled | 14m46s, passed |

The converter test bodies together took about 1.8 seconds. Their assertions remain useful; deleting tests would address little of the observed cost. Stock and adapted engines have different Xberg source identities. Keep both exact-source builds, functional/restriction tests and lint, separate targets and generated-source admission guards.

## Cache findings

1. The cache key is stable across the inspected runs. Earlier main restored it at08:22:38UTC; later main received no cache at09:29:54, and PR182 also missed at09:46. Main saved a replacement at10:14; its retry restored it at10:59. The earlier cache became unavailable. The records do not establish whether eviction, deletion or a service event caused that disappearance.
2. The observed six-cache inventory totals8.94GB:5.89GB in PR refs and3.05GB in main refs. Identical-key main/PR archives are separate entries. GitHub allows PRs to read eligible main/base caches, while main cannot consume PR-scoped caches. Duplicate uploads therefore consume storage without warming main. Storage pressure is a plausible cause of churn, not a proven eviction event. See [GitHub cache scope and eviction rules](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching).
3. The pinned cache action sees the root workspace, then prunes target outputs against its dependency graph. The separately locked converters have helper-only dependencies that this graph does not describe. Some common dependencies survive, while converter binaries and helper-only outputs are discarded. This explains compilation after a full hit. The helper lock contains136 name/version tuples, including82 distinct names, absent from the root lock; these are graph differences, not a count of observed deleted files.
4. The pinned action's README describes broader manifest discovery than its actual configured-workspace implementation. Source and logs show the standalone converter locks are not automatically key inputs. Any broader helper cache must explicitly account for those locks and source/patch identities. Preserve regeneration of checkout-specific `target/xberg-adapted` source.

## Selected bounded follow-up

Candidate `4b50fc96` adds the existing cache action's main-only `save-if` option. PRs retain cache reads and every check; main populates shared caches. OS, architecture, lane, toolchain, profile and existing lock dimensions remain intact. This avoids future large PR-only copies; it does not delete existing caches, increase paid storage or claim a measured future speedup. New dependency/profile keys on a PR may remain cold on reruns until main has an eligible cache. That is the explicit tradeoff.

The same candidate includes the separately reviewed Mac Core45→60minute job allowance, matching existing native job limits. It covers complete execution and finalization; it is a bounded reliability adjustment, not an optimization. Hosted verification and normal integration remain separate from source review.

## Deferred experiment and trigger

Lead owns any further converter-cache experiment. Revisit if cache-policy runs still show converter compilation dominating warm Mac CI. First measure bounded archive size and cold/warm behavior, bind both helper locks/patch/source/engine identities, and prove current regeneration and target/source refusals survive. Raw unpruned helper targets could worsen storage pressure. Cross-engine target sharing currently crosses an explicit guard and is unqualified. No broad raw cache, producer framework, extra infrastructure or test removal is selected by this investigation.

## Exact obsolete-cache cleanup

After PR182 merged, four exact PR182-only cache IDs were removed following ref/key/version and merged-state revalidation. Readback confirmed all four absent and all five preexisting main cache IDs retained:5,886,330,190bytes removed. The [before/after receipt](evidence/ci-cache-pr182-cleanup.json) retains each outcome. A later inventory had grown to10,478,024,387bytes; the configured storage-limit read returned HTTP402, so the actual configured limit remains unknown. No storage/billing settings changed. Cleanup does not establish what caused the earlier cache disappearance.


## Actual cache-policy inputs after integration

The completed PR183 Mac Core log records `save-if: false` and a successful cache
restore; the completed resulting-main633 Mac UI log records `save-if: true`, a
successful restore and `Cache up-to-date.` at finalization. These observations
confirm that PR restoration remains available and the main-only save condition
reaches the pinned action. The main sample did not need a new upload. They are
not a controlled speed comparison or proof of future cache retention.
[PR log receipt](evidence/ci-pr183-cache-observation.json) and
[main log receipt](evidence/ci-main633-cache-observation.json) retain exact jobs,
source, compressed raw logs and hashes. The initial log reader refused ANSI bytes;
the corrected read captured them to a file and stripped control sequences before
selected display. This was a log-access limitation, not a failed CI check.

## Warm converter-cache experiment —20October

The [completed exact-hit PR186 Mac UI job](evidence/ci-pr186-warm-mac-ui.json)
still spent404 of785seconds preparing document helpers. This meets the earlier
research trigger. The [bounded source audit](evidence/warm-helper-cache-audit.md)
initially proposed nested workspace targets; lead review rejected that layout
because root cleanup would remove helper-only dependencies before their own
cleanup pass. The corrected configuration uses disjoint targets outside root
`target`, retaining the pinned action's dependency pruning and main-only saves.

[Source qualification](evidence/ci-helper-cache-qualification.json) and
[independent review](evidence/ci-helper-cache-review.md) bind local candidate
`cd99ad1e`. The import-isolated pinned cleaner reproduced the rejected nesting
and preserved dependencies in the corrected layout. Actual stock/adapted metadata
have the same408 external package name/version/library-target tuples. All183
development-tooling tests passed; no check or source guard was removed.

A [disposable local measurement](evidence/ci-helper-cache-local-size.json)
cloned the already qualified helper outputs without modifying them. Dependency
pruning reduced758228KiB to240636KiB; the helper-only local gzip was71262653bytes
(about68MiB), with5780 entries and no helper executable, artifact, receipt or
source-root guard entries. This is a local estimate, not a hosted cache archive,
configured storage limit or speedup. Real exact-head/main checks, first main
publication size and later warm-job behavior remain separate gates in the task
record. Preview packaging retains its prior helper paths; no release was run.

## 20 October: observed same-source cache reuse after PR188

[Integrated-main cold/warm receipt](evidence/ci-main188-warm-mac-ui.json) records exactly one repeat Mac UI job on6622443e. The first run uploaded a1,902,323,274-byte combined cache. The repeat restored the exact full key, regenerated the derived adapted workspace, passed every applicable step and reported cache up-to-date with no new upload. All four resulting-main required contexts remain passed; optional Windows failures remain visible.

| Observed same-source job | Cold | Warm |
|---|---:|---:|
| Complete Mac UI job, excluding runner queue |30m28s|6m23s|
| Both helper graphs preparation |11m07s|1m23s|

The warm job additionally waited5m07s for a hosted runner. This is one pair on different actual runners with the same source, cache key, pinned toolchain and runner image; it is no controlled benchmark or future cache-retention/latency guarantee. The measured combined archive is distinct from the local helper-only size estimate. No further experiment reruns or storage/billing changes are selected.

## Remaining Mac lanes measured20October

[Two completed-run measurements](evidence/residual-mac-ci-audit.md) separate job queue6–21seconds from Mac Core16m13/17m36 and Combined5m11/7m58 runtimes. Core is led by default workspace tests, helper API tests and dual-helper preparation; combined is led by canonical search app tests. These are different sources/runners with no nested exact-cache-hit observations; repeated preparation is not proven dispensable. No lane sharing, gate removal or extra rerun is selected from this sample. Existing logs of the in-progress PR190 can support a later exact-hit/preparation assessment without a new benchmark run.

### Subsequent PR190 Mac cache observation

[Completed actual Mac jobs](evidence/ci-pr190-mac-cache-observation.json) restored full matching keys: Core archive1,327,152,455bytes and Combined2,332,333,091bytes. Core passed in16m11; Combined in10m05. Core default tests5m43, helper API/restrictions3m52 and helper preparation2m06 remain major costs despite the cache hit. This is another uncontrolled source/runner observation, not an optimization result. The completed Mac job logs are available while the required Ubuntu gate is still running; no rerun or configuration change is selected.
