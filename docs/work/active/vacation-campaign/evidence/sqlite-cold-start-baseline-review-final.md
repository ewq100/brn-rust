# Final cold-start adapter disposition

**Approve one bounded campaign with the exclusive root build slot. P2 valid/fixed; no remaining actionable findings.** No execution occurred in this review.

Verified corrected `cold_processes.py` SHA-256:
`2e182ceaa08d030edba26628b382a48d61ce72868ba140316ea00065627e95f6`.

Actual lines 30–36 now append and print the returned attempt's actual code, raw-log hash, byte count, accepted-size status and `log_persisted:false` before writing its raw file or enforcing the 64 KiB limit. Successful writing updates `log_persisted:true`. Oversized output sets `one_test_pass:false`, remains represented in the final receipt, raises an explicit error and stops further launches. A raw-file write failure likewise retains the returned row with false persistence status and fails. This closes the selected evidence-loss path.

Independently rehashed unchanged components:

- Witness source: `73d0eae7229c66eaaed50dcd7340f0a75480e1b3c6e246add0c09474a3391023`, candidate `4c786a962ed52998648a0ae73232c61a0b41a5e9`.
- `run.sh`: `e1c2ef65a3c0dc854ff7795c7a9d164e4e98d60f1d999fe9ee83aa1398ac351c`.
- `supervise.py`: `10d0ee8ae181c8838a181b5bcc735eecc2b141a6a4e32169d621c43459cf8822`.

The complete initial review `/tmp/brn-sqlite-cold-start-baseline-review.md` is preserved and reused for unchanged source, process bounds, cleanup, accepted-size limits and interpretation. This remains at most 20 cold processes, stop-first-invalid, `qualified:false`. A native negative exit requires matching-stack assessment; passes cannot refute a timing-sensitive defect. Parent-reported remaining privacy gates passing do not close the original crash or change that limit.
