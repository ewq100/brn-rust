export PATH=/Users/evokessler/.rustup/toolchains/1.98.1-aarch64-apple-darwin/bin:$PATH
export DYLD_LIBRARY_PATH=/Users/evokessler/.rustup/toolchains/1.98.1-aarch64-apple-darwin/lib
export CARGO_TARGET_DIR=/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust/target
export CARGO_BUILD_JOBS=1 CARGO_NET_OFFLINE=true CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
export BRN_XBERG_HELPER=$CARGO_TARGET_DIR/debug/brn-xberg-helper
export BRN_XBERG_HELPER_BRN=$CARGO_TARGET_DIR/debug/brn-xberg-helper-brn
export TMPDIR=/private/tmp/brn-launcher-focus-qualification-tmp
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
