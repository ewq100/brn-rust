# Combined review round 1 response

18 October 2026. The [full verbatim review](review-round1.txt) returned CHANGES_REQUIRED with one P2 finding and no advisories. This bounded implementation addresses P2; independent parent-owned round 2 is pending. This is not review self-acceptance or publication readiness.

Corrected test-only material: **`ec2f5c25eef393c60816305efda56836bd1d1cd5`**. Tree: `8119e6cce5ab8032139a3dd7734e5626a00bd0b2`. Crates tree: `7905a64962ed2ecacc79b8eed5aecbe14adce4bf`. Parent: evidence HEAD `779cf0d19e92e0071c74bbe895996021869f1c7c`. The material delta is exactly two source paths, ten attribute insertions and one replaced attribute line. No test body, assertion, production statement, dependency, manifest or lock changed.

## P2 mapping

| Affected item | Correction |
|---|---|
| `threads_summary.rs::filesystem_tests` | `cfg(test)` becomes `cfg(all(test, unix))`; the symlink, FIFO and permission fixture is excluded on non-Unix targets. |
| `copilot_compaction_uses_same_chosen_account_model_effort_and_wire_without_caps` | Individual `cfg(unix)`; its direct Unix directory/permission APIs remain unchanged. |
| `review1::credentials` | Individual `cfg(unix)` around the Unix fixture helper. |
| `review1::copilot_settings` and `review1::exchange` | Individual `cfg(unix)` matching their only callers, avoiding unused helper failures off Unix. |
| `review1::exchanged_copilot_rotation_reuses_bootstrap_but_account_switch_invalidates` | Individual `cfg(unix)`; also directly imports Unix permissions. |
| `review1::compactor_empty_chat_refusal_retries_same_model_on_responses_once` | Individual `cfg(unix)` matching its credential/settings/exchange helper use. |
| `review1::compactor_context_overflow_or_partial_refusal_never_retries_responses` | Individual `cfg(unix)` matching the same three helpers. |
| `missing_copilot_bootstrap_leaves_summary_uncached_and_preserves_chat` | Individual `cfg(unix)` around its direct Unix fixture APIs. |
| `checked_bootstrap_fingerprint_is_trim_stable_and_rejects_invalid_credential` | Individual `cfg(unix)` around its direct Unix permission APIs. |

All callers of the three `review1` helpers are the three gated `review1` tests in the table. The enclosing `summary_tests` and `review1` modules remain ungated. Four portable `review1` tests remain enabled: access-token rotation, complete cache with a busy lease, well-formed cache corruption, and unrelated project/current-source changes. Source analysis retains 20 portable summary tests on non-Unix targets. The existing frozen-skill macOS/Linux cfg is unchanged. No `allow` attribute or assertion weakening was added.

## Actual bounded checks

Rust 1.98.1, locked/offline, jobs 1, existing own fixed target, fresh 0700 temporary directories. No inputs changed during Cargo. Old AI artifacts and `.d` files were copied into this follow-up's content-addressed custody before overwrite.

- `cargo +1.98.1 test --locked --offline --jobs 1 -p brn-ai --features scripted-runs --lib -- --test-threads=1 threads::summary_tests:: threads::summary::filesystem_tests::`: **28 passed / 0 failed / 0 ignored**. This one focused lane executes all 27 summary tests and the filesystem test on macOS. The exact selected 28 names match the preserved prior binary's list.
- `cargo +1.98.1 clippy --locked --offline --jobs 1 -p brn-ai --features scripted-runs --all-targets -- -D warnings`: passed.
- `cargo +1.98.1 fmt --all --check`: passed. Whitespace and final evidence-link checks are indexed separately.

Only `aarch64-apple-darwin` is installed. The non-Unix correction is source-derived cfg/caller qualification; no Windows target, build, download or hosted pass is claimed. All gates evaluate true on this Mac, and stripping only the added/replaced cfg attributes reproduces the prior source bytes exactly. This preserves Mac test bodies and production behavior; it does not imply a newly rebuilt production package.

Raw namespace: `/private/tmp/brn-final-combined-integration-20261018/review-round1-cfg-20261018`. `material-test-artifacts.json` binds the exact test binary, `.d`, source hashes, features and 28 test names. Binary SHA-256: `a76d53057294ae0d4e7095ffb8b99497b2112b304aac745c2a2b31b498750ace`; `.d` SHA-256: `4a2e211737ddbe43a9a509bb0fa6ea3bc8e2748407cd77e8640805d40424b3ad`. Every command also has before/after artifact maps, input hashes, status/diff snapshots, output and source-stability checks.

The protected-byte audit confirms 396 other tracked source files and 1418 prior package/artifact files unchanged, including root/standalone locks, helper runtime, accepted patches, original package and signed relocated contents. Current source and parent/main comparison patches are in this namespace. The earlier 1265/10/5 workspace and its dispositions are unchanged; no workspace, full AI, native, retrieval, helper or package repeat occurred.

## Package and handoff separation

The existing package remains pinned to **`2dd7cc604425658ad60f52734e4f8aa2f6b7fa3d`**, ZIP SHA-256 `08f34d06654b627a1951d13239fed931f44e7054f5b3792194e78816daca2196`. No byte or embedded pin was changed. Its earlier qualification remains associated with that source. The new material has only the bounded focused/lint/fmt evidence above; earlier broad passes are not relabelled to it.

The [source pin](source-pin.json), [check index](check-index.json), [release inventory](release.json) and [handoff](handoff.md) distinguish current test-only material from the original compiled/package source. Original implementation records and the review text remain preserved. SDK/widget causes, schema 7 coverage, native-human, live/owner/program/platform qualifications and all closed/frozen allocations remain as previously recorded. Parent owns a new independent round 2 and subsequent PR/required CI/main CI; this worker does not publish.
