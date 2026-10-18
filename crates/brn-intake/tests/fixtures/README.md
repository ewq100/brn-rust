# Maintained intake test fixtures

These synthetic fixtures belong to the maintained intake regression suite. On 11 October 2026 they were copied byte-for-byte from the standalone qualification experiments at baseline `3a3aa7ac6f2a70815ffc778a694663f729203a79`, before those experiments were retired. No asset was regenerated.

| Fixture | Original location at baseline | SHA-256 |
|---|---|---|
| `forecast.xlsx` | [source](https://github.com/ewq100/brn-rust/blob/3a3aa7ac6f2a70815ffc778a694663f729203a79/experiments/architecture-reassessment/p1-office-mime/fixtures/forecast.xlsx) | `916dc333d40b37dce3b0a32e4b9ea6bea145fee8369271778ca7e71196d04704` |
| `harbor.docx` | [source](https://github.com/ewq100/brn-rust/blob/3a3aa7ac6f2a70815ffc778a694663f729203a79/experiments/architecture-reassessment/p1-office-mime/fixtures/harbor.docx) | `46a227162352ff616344ab5bc91350d1c1f2810bb099f873ab49979c3133df85` |
| `harbor.pptx` | [source](https://github.com/ewq100/brn-rust/blob/3a3aa7ac6f2a70815ffc778a694663f729203a79/experiments/architecture-reassessment/p1-office-mime/fixtures/harbor.pptx) | `8fa8c3d08fc157cd08d4a5cb17c48c54ca7b7723930b6d35c484f9b975455280` |
| `hostile-dtd.docx` | [source](https://github.com/ewq100/brn-rust/blob/3a3aa7ac6f2a70815ffc778a694663f729203a79/experiments/architecture-reassessment/p1-office-mime/fixtures/hostile-dtd.docx) | `c5da4fb9c60f4c9455f6216ddef345c059953f3879afbb849239a3d5b8fb967b` |
| `hostile-inflate.docx` | [source](https://github.com/ewq100/brn-rust/blob/3a3aa7ac6f2a70815ffc778a694663f729203a79/experiments/architecture-reassessment/p1-office-mime/fixtures/hostile-inflate.docx) | `c79196faaded8e6d6f9eeb2a2b733607ee1e7b49a7affc076a0bb9c1b033d019` |
| `hostile-members.docx` | [source](https://github.com/ewq100/brn-rust/blob/3a3aa7ac6f2a70815ffc778a694663f729203a79/experiments/architecture-reassessment/p1-office-mime/fixtures/hostile-members.docx) | `befb44dbb220473403906cbbf4485e5561f5714e9dd8a171647cd22b561c32f2` |
| `hostile-path.docx` | [source](https://github.com/ewq100/brn-rust/blob/3a3aa7ac6f2a70815ffc778a694663f729203a79/experiments/architecture-reassessment/p1-office-mime/fixtures/hostile-path.docx) | `c85c4bad47aa98a3b763eade88307968da6ab18a42619d9a2d54007a841ecb74` |
| `plural.eml` | [source](https://github.com/ewq100/brn-rust/blob/3a3aa7ac6f2a70815ffc778a694663f729203a79/experiments/architecture-reassessment/p1-office-mime/fixtures/plural.eml) | `f10516cc3db6eb065290c9e372aba0a80373f2a7bc9b8582e51562e719c48c14` |
| `single.eml` | [source](https://github.com/ewq100/brn-rust/blob/3a3aa7ac6f2a70815ffc778a694663f729203a79/experiments/architecture-reassessment/p1-office-mime/fixtures/single.eml) | `47f34537c79fb7ad86f4751f6b59d856af5819e4e5918b93775403fd4970dd82` |
| `water-use.png` | [source](https://github.com/ewq100/brn-rust/blob/3a3aa7ac6f2a70815ffc778a694663f729203a79/experiments/architecture-reassessment/p1-office-mime/fixtures/water-use.png) | `f76dd37920a0fc5889b1d53a0c448f7ec6c619ed8200eef5d31621e36ca4d61d` |
| `features.docx` | [source](https://github.com/ewq100/brn-rust/blob/3a3aa7ac6f2a70815ffc778a694663f729203a79/experiments/architecture-reassessment/p2-api-check/features.docx) | `04db9ac31374868d55d0ff579f23ad5bc558fb9fe9a2363305498817e335ea6d` |
| `prefix-variation.docx` | [source](https://github.com/ewq100/brn-rust/blob/3a3aa7ac6f2a70815ffc778a694663f729203a79/experiments/architecture-reassessment/p2-api-check/prefix-variation.docx) | `7c2c8f4f4866db174495ad616d6995f002b63479d3ba3a22b182fb8151493709` |

`textual.html` (SHA-256 `04973e04d349c4415c5d89a97d754650f94a3128de0a35fd4044eaf5a2c7c1f5`) was written by hand on 17 October 2026 for the X2 `html-textual-1` helper slice. It is synthetic public text.

The existing `quay.*` and `quay-map.*` fixtures and `make-quay.py` remain the maintained PPTX fixture set. Current Threads PDF/DOCX fixtures live in `crates/brn-threads-intake/tests/fixtures/`.
