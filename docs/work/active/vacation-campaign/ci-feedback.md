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
receipts. This could remove two net duplicate Mac builds; a producer itself still
builds the pair. It could also lengthen the dependency path or repeat helper tests,
so no savings or approval is claimed. Evaluate graph/test ownership, required check
names, artifact integrity, source/engine/lock identity, platform separation and
failure behavior before implementation. Existing helper API/restriction/lint and
all application checks must remain effective. Do not weaken checks, reuse a helper
across platforms or silently trust cached generated source. Linux work remains
deferred apart from preservation of existing required checks.
