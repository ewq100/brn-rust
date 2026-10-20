# Restore source refusal implementation

On `codex/restore-source-refusal` at `a23f486b109c67e8d07c77800030e527b9a18002`, the restore path now maps only `rusqlite::ErrorCode::NotADatabase` returned while validating the read-only source to `Error::UnsafeDirectory`. The mapping runs before `Self::prepare`, reports “The restore source is not a BRN backup; choose a BRN backup file,” and leaves all other validation, SQLite and I/O errors untouched.

Updated the `unsafe_directory` descriptions in `crates/brn/src/contract.rs` and `crates/brn/README.md` to document non-SQLite restore input. Retained the assigned core and CLI regression-test edits. Four files total changed. No build, test, format, commit, native, account or network action was run; lead owns final qualification and review.
