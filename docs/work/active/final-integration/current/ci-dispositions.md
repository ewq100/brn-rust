# PR 159 CI dispositions

## Documentation links, 18 October 2026

Initial head `c51684fbd7e8629b9f7817dcefc3b0a025824e83`, run [39144370163](https://github.com/ewq100/brn-rust/actions/runs/39144370163), Documentation and tooling job failed: the Markdown checker reported two missing files in the historical Office O1 review. Both rendered links escaped the checkout into the developer's Cargo registry. The 171 tooling tests passed with one platform skip; their pass did not close the failed link step.

The rendering wrapper now links those same two source locations to immutable upstream revision `34b9e93a657158fcf3d1d1035df3202031625bdd`, obtained from the published package's `.cargo_vcs_info.json` and repository metadata. The upstream file was read and verified byte-equal to the cached `betteroffice-docx-parse` 0.3.0 source, SHA-256 `cc4f0f33951de555e0dc91deb2984828acd21efd7c801025cc1ec0803e831cd6`. Original line labels and report wording remain; only the rendering note and two destinations change. The existing verbatim `opus-review-round1.txt` is untouched.

A current-document link scan found exactly these two links escaping the checkout. After correction, the maintained Markdown checker and whitespace check passed. No checker exception, dependency, workflow, test or runtime change was introduced. The failed CI job log is preserved under the lead's `pr159-ci/39144370163-documentation.log` evidence namespace; exact-head hosted verification remains pending after the correction.
