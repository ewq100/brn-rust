# Principal native-lane follow-up

The final `writing_host` example declares `test = true` and still requires `native-ui`. The ordinary maintained macOS writing-UI command now automatically executes both the library and example test harness:

```sh
cargo test -p brn-writing-ui --features native-ui,native-test-support --locked --offline
```

This actual correction-window command passed **44 library tests and 5 example tests**, with separate `Running unittests src/lib.rs` and `Running unittests examples/writing_host.rs` headers. [Correction receipts](opus1-checks.json) retain the actual precommit source, command, log, binary and dep-info identities. Strict `--all-targets` Clippy also emits an example artifact with `profile.test = true`; its dependency list includes `journey.rs`. The ordinary example artifact is recorded separately and does not include that test module.

Historically, at c95 the example had Cargo's default `test = false`: explicit example tests passed 3/0/0, but ordinary library tests and all-target lint did not cover its test harness. Principal's reviewed 50fa explicit script command and three-line macOS CI step were correct for that source. They are now superseded by verified automatic coverage and only those redundant lines were removed. The required CI job names, existing writing test/lint commands, all other steps, default feature gates, six development dependencies and Cargo.lock remain unchanged.

The correction window checked script syntax and executed the applicable native writing/desktop commands individually; it did not execute the full script or hosted workflow. See the [review response and limits](opus1-response.md). Hosted/native-human/GUI and principal acceptance remain open.
