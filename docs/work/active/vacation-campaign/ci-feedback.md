# CI feedback cost observation

A bounded Luna audit inspected completed PR165 run39213694797 and the unchanged
workflow/helper preparation sources. The lead independently read the completed
job/step timestamps into [this receipt](evidence/ci-helper-timing.json).

| Required lane | Helper preparation step | Whole job |
| --- | --- | --- |
| Core Ubuntu | 8m31s | 24m07s |
| Native UI Mac | 6m21s | 12m02s |
| Native retrieval Mac | 5m48s | 12m05s |
| Core Mac | 5m43s | 20m19s |

Each lane builds both separately locked stock/adapted helper graphs. Existing
Rust cache keys are lane-specific. The audit observed full-match cache hits in
the Mac jobs but still observed both helper binaries compiling; Ubuntu Core missed
the cache. These are one-run observations, not stable cold/warm benchmarks or
additive wall-clock timings.

One possible follow-up is to qualify one exact Mac helper pair and let the three
Mac application lanes validate the same-run artifacts and their source/lock/patch
receipts. This could remove two net duplicate Mac helper pairs; a producer itself still
builds the pair. It could also lengthen the dependency path or repeat helper tests,
so no savings or approval is claimed. Evaluate graph/test ownership, required check
names, artifact integrity, source/engine/lock identity, platform separation and
failure behavior before implementation. Existing helper API/restriction/lint and
all application checks must remain effective. Do not weaken checks, reuse a helper
across platforms or silently trust cached generated source. Linux work remains
deferred apart from preservation of existing required checks.

## Feasibility follow-up, not adoption

The two native Mac preparation steps total12m09s of observed runner time; these
parallel step costs are not an elapsed-workflow saving. Core Mac helper qualification
adds5m17s to its5m43s preparation. A dedicated producer preserving both would take
about11m plus setup/queue/transfer, while inferred remaining Core/UI/retrieval Mac
work is9m19s/5m41s/6m17s. A hypothetical12m producer would leave Core Ubuntu24m07s
as the longest required path in this one snapshot. These subtractions are an
inference; no changed workflow was run or savings established.

Do not attach the native matrices to the whole headless matrix: that would couple
Mac checks to unrelated Windows failure. Preserve the four required context names,
platform/event conditions and all effective tests. Failed dependencies normally
skip consumers, and [skipped required checks can count as success](https://docs.github.com/en/pull-requests/how-tos/merge-and-close-pull-requests/troubleshooting-required-status-checks).
An isolated prototype would need explicit consumer execution/failure on producer
failure, using the documented [dependency/always semantics](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax),
plus exact platform/toolchain/source/lock/patch/engine/binary admission and tamper
rejection before use. [Same-run artifacts](https://docs.github.com/en/actions/concepts/workflows-and-actions/workflow-artifacts)
can transport builds; that capability alone does not qualify this design.

Current disposition: defer workflow implementation while the reference-import
owner and Mac application qualification take priority. A future bounded prototype
must demonstrate passing check identities, explicit producer-failure propagation,
tampered-artifact refusal and retained helper/application qualification without
branch-protection changes. No extra infrastructure or weaker checks are selected.


## Selected small follow-up: cheap intake guard before preparation

The current candidate moves only the existing headless production-intake retirement
check before expensive helper preparation, alongside the existing provider guard.
The check reads repository source/manifests/workflow text and needs no prepared
binary or generated workspace. Its name, command and Unix condition are unchanged.
Build, lint, helper preparation/qualification, tests, runner matrix, pins, permissions
and required context names remain unchanged. A retired-route failure now stops
before helper preparation; no successful-run speedup is claimed.

A broader compile-before-preparation reorder is not selected. Headless tests
deliberately run after lint failure using workspace-build success; moving that
build ahead of preparation would also make them run after failed preparation.
Preserving that behavior would add more guards/state for limited benefit. Native
regrouping and producer/artifact sharing remain deferred. This is a three-line step
move, not a new CI framework or qualification shortcut.

Local retirement check and all5 existing retirement tooling tests passed, including
negative controls for legacy paths/dependencies and workflow fetch/generator ordering.
Ruby Psych parsed workflow YAML; no parser dependency was installed. Application
Rust/manifests/locks/toolchain are unchanged from qualified MCP production. Independent Sol review of complete97f against63b found no actionable issues;
[the receipt](evidence/native-prep-review.json) binds the exact complete diff.
Actual hosted gates remain next; no new Rust suite is needed merely for this
configuration/docs change.
