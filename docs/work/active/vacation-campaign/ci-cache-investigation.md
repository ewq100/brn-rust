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
