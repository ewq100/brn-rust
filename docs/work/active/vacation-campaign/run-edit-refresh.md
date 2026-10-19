# Run-bound edit-refresh ownership

Production source `23c27cd1ed2a0878c4bed9cae254106584ef7034`, integrated as
`c39a6af3abec4635b07af7287fc60528468e6400`, moves the optional exact Note/version
refresh target into the existing private Run operation. The separate UI tuple,
duplicated run-id filtering and disconnect cleanup are retired together. Run
identity remains about thread/run ids. Non-Run work cannot carry the target; it is
attached only after successful preparation/spawn, copied before terminal retirement
and dropped with a disconnected worker. The existing clean-buffer/version/visible
Note gate is unchanged. No reproduced production defect or speedup is claimed.

Independent Sol approved the plan and complete candidate without source/design
findings. Review identified stale current records after integration; those are
reconciled before publication. Existing clean/typed/hidden completion, contextual
refusal, worker/disconnect/Stop/Steer and successor witnesses are retained; only
constructor fixtures changed. There is no redundant getter test or general owner.

Focused completion1/0, worker7/0 and contextual3/0 passed. Final desktop321/0/1,
strict all-target Clippy, formatting, whitespace and an actual production-only
native-ui build passed. The ignored external CLI fixture remains explicitly unrun.
[Commands](evidence/run-edit-refresh-commands.txt), [worker receipt](evidence/run-edit-refresh-worker-receipt.md)
and [hash/result receipt](evidence/run-edit-refresh-qualification.json) retain source,
raw/gzip log hashes, artifact identity and observed Cargo results. Separate shell
exit-code logs were not retained; Cargo result/Finished lines corroborate success.
The lead verified all tracked Rust/crate manifests against the qualified source
and the immutable binary hash. Older artifacts/helpers are preserved.

The successful provider EditNote button-to-refresh journey is not established by
offline refusal plus injected completion checks. No live call or native OS input
was added. Hosted exact-head CI, integration/main verification and queued Mac
qualification remain separate gates. The [late-refresh regression](late-refresh.md)
adds a distinct protection-effectiveness witness in the same later batch.
