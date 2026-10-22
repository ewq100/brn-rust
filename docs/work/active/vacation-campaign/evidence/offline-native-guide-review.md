# Independent offline native-retrieval guide review

**APPROVED — no blocking or actionable source findings.** Exact clean candidate `9533de6d0be815653c38d241c615b2e38d4a471a`, branch `codex/offline-native-retrieval-guide-20261022`, checkout `/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust`; verified merge base `3827d6a13953533c7322fd7157d975a32dee5cb2`. Complete actual task diff is only `docs/development/tooling.md` and `docs/development/verification.md`, 24 insertions. Reviewed both complete documents and relevant callers/feature declarations. Reviewer authored neither candidate nor qualification runner.

Final file SHA256s: tooling `72007d2dea6aefb8dccf827a6bf353a09e20670bced2df052046986579e17bc3`; verification `4500d16010d54ae63064db81ae1b2349e0dc96583eac2518827f08715c98b629`.

## Source consistency

Inspected cached pinned `ort-sys 2.0.0-rc.13` actual `build/main.rs`, `build/vars.rs`, `build/download/mod.rs`, static/dynamic linking branches; root lock also selects ort 2.0.0-rc.13. In normal linking mode, explicit `ORT_LIB_PATH`/`ORT_LIB_LOCATION` is checked **before** download skipping or automatic distribution-cache selection. With no supplied path, `download::should_skip` consults the first defined variable in `CARGO_NET_OFFLINE`, `ORT_SKIP_DOWNLOAD`, `ORT_OFFLINE`; true/1 skips before reaching automatic cache lookup and defers failure to linking. The new example sets all three consistently and supplies an explicit existing library directory, matching the diagnosed failure/correction. An existing cache by itself does not select that directory on this skipped branch.

The documented aliases are both supported (upstream gives `ORT_LIB_PATH` precedence if both are present). The guide requires the caller to identify an actually compatible target/version/API library; it does not promise arbitrary directories, cross-target compatibility or a particular link mode. Default static-link versus explicit dynamic-link details remain dependency configuration, not a newly introduced BRN policy. The example's `brn-threads-app/native-retrieval` feature exists and delegates to the maintained retrieval native feature. Existing repository pin/lock instructions apply.

Read lead evidence `tls-final-qualify.py` and `tls-final-local-qualification.json` (receipt SHA256 `535e0bcb1effbc0a93540f17c1aa4d56a8061a85ad1920d091240af2b722ae5d`). They bind source `234956cb171beb025acb1dc17b41be98992bddf0`, declare ORT skip flags plus the explicit preexisting local library, record no selected download, assert the library hash unchanged, and report passing native-retrieval synthetic tests/combined compilation. The runner's inherited environment whitelist excludes conflicting ORT aliases. This is source-bound local evidence; it does not prove the example on every target or supply model/native/hosted acceptance. No gate was rerun by this reviewer.

## Scope and proportionality

The verification entry links to the single setup explanation rather than duplicating configuration. Generic absolute placeholders avoid baking in this host's cache path, architecture, archive hash or qualification timestamp. The text clearly distinguishes crate offline operation, native link inputs, embedding-model assets, synthetic/build evidence and native behavior; it explicitly stops when no compatible local library exists. No model acquisition, provider/account authorization, download instruction, global environment change, source/runtime behavior or required CI gate is added. The historical TLS failure remains evidence rather than being relabelled as product success.

`git diff --check` passed; new relative link resolves to the maintained tooling document. Documentation/link/command consistency checks are proportionate; no Rust rebuild, native launch or new test is warranted solely for these two text files. Lead should complete normal documentation/link checks and reconcile this docs-only candidate onto the actual integration base. This approval does not establish TLS214 hosted checks, merge, native acceptance or release.

Only this report was written. No source edit, build/test workload, dependency fetch, account/native action or helper was performed.
