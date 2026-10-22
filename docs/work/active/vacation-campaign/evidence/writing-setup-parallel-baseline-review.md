# Original-binary parallel baseline review

**Approve one bounded diagnostic campaign. No actionable findings.** No execution was performed by the reviewer.

Read all three actual adapters and verified SHA-256:

- `run.sh`: `dfaeae594a1e96b74a3533fb815fc123f3061f2e605bc64cc2fbe8bb7258c166`
- `cold_processes.py`: `ed0f8621fb96a9dd5f3d1fbad9643b9a1076a7462630cfaa33d696929a1497a6`
- `supervise.py`: `b483d5c285108acad6da16d49fa3c014b5cf0e75372ed84e349191690d4de1f6`

The controller pins clean source `ccd05b28ea7e1d7bcd92fe266f6e37aa23461d1b`, actual writing_setup source hash `241ef3a5e87d8e0e1d02487aba5e43759664428196a0f2546a91937ee036f2b8`, original binary `cd20fe93330b7b00eb3e9a226421f451e46620c53a308c4106eecafb7e7e2ca4` and UUID `79F53E61-A47D-382F-B602-4D3046124A83`. Source/binary hashes independently match. No Cargo/build/discovery/clean or source modification remains.

Each fresh process explicitly uses 22 workers and must return the exact 22-pass summary. Maximum 20 processes; first nonzero/invalid result stops. Returned code/hash/bytes/validity are recorded before raw-log persistence and size rejection, retaining the prior correction.

Reuse `/tmp/brn-sqlite-cold-start-baseline-review.md` and its final disposition for unchanged whole-group cleanup, 180-second trigger, disk floors, accepted-size limits and persistence/error behavior. Supervisor diff confirms only identity/scope and binary-verification admission changed. Root must retain exclusive shared-target ownership.

This restores the original app fixture/link/layout boundary absent from the core-only witness. The original initialization/null-callback stack supports this discriminator without proving its cause. `qualified:false` remains unconditional. Native `-11` requires matching-stack classification; twenty passes establish non-reproduction only. After this campaign, reassess rather than repeat or infer a speculative fix.
