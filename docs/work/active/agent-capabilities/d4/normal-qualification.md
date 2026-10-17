# D4 normal qualification at precommit source

Each row is one actual command receipt, with complete command/features/environment/source hashes/raw output/artifacts in the [index](check-index.json). Counts overlap between commands and are not a unique-test total. The source handoff maps these actual precommit HEAD+WIP inputs to the immutable material pin; no future SHA is invented in an earlier receipt.

| Receipt | Exit | Passed / failed / ignored per summary | Seconds |
|---|---:|---|---:|
| `ai-full-scripted-reviewed-snapshot` | 0 | 477/0/1 | 98.692986 |
| `owner-mcp-surfaces-reviewed-snapshot` | 0 | 4/0/0; 3/0/0 | 60.670861 |
| `core-mcp-and-schema-current` | 0 | 14/0/0; 1/0/0; 4/0/0; 12/0/0 | 22.147395 |
| `strict-workspace-reviewed-snapshot` | 0 | No Rust test summary; command passed | 17.873103 |
| `capability-test-reviewed-snapshot` | 0 | 455/0/1; 3/0/0; 0/0/0 | 121.053551 |
| `capability-lint-reviewed-snapshot` | 0 | No Rust test summary; command passed | 6.952971 |
| `native-ui-build-reviewed-snapshot` | 0 | No Rust test summary; command passed | 13.920255 |
| `native-ui-lint-reviewed-snapshot` | 0 | No Rust test summary; command passed | 6.107316 |
| `native-ui-state-reviewed-snapshot` | 0 | 95/0/0 | 70.164792 |
| `native-widget-suite-stop-diagnostic` | 0 | 315/0/1 | 5.944267 |
| `native-widgets-lint-stop-diagnostic` | 0 | No Rust test summary; command passed | 3.827989 |
| `retrieval-native-test-current` | 0 | 33/0/0; 2/0/0 | 101.368632 |
| `app-native-retrieval-test-current` | 0 | 247/0/1 across37 summaries (raw rows retained) | 277.896746 |
| `cli-native-retrieval-build-current` | 0 | No Rust test summary; command passed | 54.957040 |
| `combined-native-build-current` | 0 | No Rust test summary; command passed | 70.748870 |
| `combined-native-lint-current` | 0 | No Rust test summary; command passed | 32.582664 |
| `combined-native-state-current` | 0 | 95/0/0 | 299.282143 |
| `default-workspace-build-current` | 0 | No Rust test summary; command passed | 8.109466 |
| `offline-end-to-end-current` | 0 | Offline fixtures; CLI108 steps/1266 checks | 3.923073 |
| `format-final` | 0 | No Rust test summary; command passed | 1.614753 |
| `documentation-final` | 0 | No Rust test summary; command passed | 0.350259 |

The current normal build emits two rmcp package units/profiles, both without HTTP-server/child-process/transport-io features; axum is absent from these actual normal build units. Upstream default server/macros/base64/async-rw code still compiles. `normal-unit-features-current.json` retains every observed selected unit and binary/library hashes. An initial extractor incorrectly asserted one package unit and failed; its corrected extraction reused the SAME raw build output, without a Cargo rerun or feature change. Dev/test feature unification remains separately documented.

The cached arm64 ORT static library and pinned build-script configuration are recorded in `ort-cached-no-download-preflight.json`; actual retrieval/combined commands use explicit local linking with skip/offline settings. No model asset was downloaded or loaded, and no inference/semantic retrieval or other-platform claim follows.

## Retained failures and gaps

- Original full workspace `--no-fail-fast`: **1224/5/6**,101 summaries,exit101. Four new D4 CLI exhaustive-registration/README omissions were corrected and the original unit29/0+agent-contract23/0 assertions passed at their source; the fifth is carried PDF preparation Unavailable without native pdftoppm. The full command is not relabelled green or repeated solely for green.
- Current native widgets initially **314/1/1** at an existing killed-reply Stop expectedInterrupted/fence2 assertion. Its actual state was not printed. Preserved binary/source, one unchanged exact1/0, message-only diagnostic exact1/0 and ONE full diagnostic315/0/1 remain separate. No cause/flakiness/baseline attribution is proven; [full qualification](native-stop-diagnostic.md) is required for review.
- Initial compiler/API/fixture/zero-filter attempts and genuine before behavioral failures remain in raw receipts. The interrupted capability build has NO VERDICT; replacement capability command has its own receipt.
- Old8/new9 compatibility execution remains at its actual earlier WIP/binary source association ([evidence](schema9-evidence.md)); it is not fabricated as a contemporary final-pin build. Later core checks preserve format/authority behavior.
- No mutation campaign or new live-provider/external MCP window has been granted or executed. Fresh comprehensive independent review and principal source acceptance remain pending. Hosted/other-platform/native-human/full-program/main integration are separate.
