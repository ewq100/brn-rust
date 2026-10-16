# Principal native-lane follow-up

The B4 host's example tests require an explicit Cargo target. Ordinary library tests do not execute them. The principal adds the existing verified command to the macOS native CI lane and maintained desktop verification script:

```sh
cargo test -p brn-writing-ui --example writing_host --features native-ui,native-test-support --locked --offline
```

The B4 final-material command already passed3/0/0 at sourcec95b17a8; [raw command/source evidence](checks.json) retains its identity. This follow-up changes no host/library/desktop source, manifest, lock, default feature or required CI job name. Script syntax and diff whitespace were checked locally; the full revised script and hosted workflow were not executed by this docs/CI follow-up. Fresh Opus1 reviews the B4 source pin and this separate principal CI/script delta before acceptance.
