# Qualification evidence

Read the [qualification report](../qualification.md) for results and limits. These
receipts are dated evidence, not current product guidance or owner authorization.
All document/test inputs are synthetic. No provider credentials or raw chat
transcripts are included.

- `intake-helper-artifacts.json` identifies both final qualified binaries;
  `intake-helper-source-manifest.json` binds the helper sources to their frozen
  commit. `intake-independent-pdf-pixels.json` is the independent raster oracle.
- `integrated-*.log.gz` records the integration checks. The initial
  `integrated-source-assets.log.gz` matched zero tests and is not proof; the
  corrected log runs the two intended source-tool cases.
- `workflow-*.log.gz` and `intake-*.log.gz` preserve focused lane runs, including
  failed attempts before corrections. Read the report rather than interpreting
  every log as successful. The rejected JPEG decoder probe is research evidence,
  not a shipped dependency.
- `baseline-python.log.gz` preserves the initial process-reaping failure;
  `baseline-python-reaped.log.gz` and `integrated-python.log.gz` record successful
  unchanged-suite runs under a process-local Linux subreaper.
- `overview-tests.log.gz` matched zero tests because of a mistyped discovery
  filename; `overview-tests-corrected.log.gz` runs all 16 overview tests.
- `environment.sh.gz`, `container-subreaper.py.gz` and
  `integrated-commands.sh.gz` preserve the exact task-local execution setup.
  They are reproduction records, not a second product test framework.
- `live-admission.json` records a supported initialization failure with zero model
  calls. Unknown usage/cost is not represented as zero measured spend.
- `manifest.json` records SHA256 digests of the archived receipts.

Decompress a log with `gzip -dc path/to/log.gz`. Local absolute paths in receipts
identify this cloud execution environment; they are not portable installation
paths. Current product gates remain the maintained scripts and tests in the repo.
