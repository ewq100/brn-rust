# Application size report

Use `scripts/report-sizes.sh` when you need a quick, local diagnostic of allocated disk usage. It accepts only paths supplied on the command line and never scans the home directory. Run it without arguments to print usage.

Select components explicitly when their role is known:

```sh
bash scripts/report-sizes.sh \
  --app "/path/to/BRN.app" \
  --build "/path/to/brn-rust/target" \
  --model-cache "/path/to/model-cache"
```

Use `--path PATH` (or a positional path) for another explicitly selected path. Options may be repeated. Paths with spaces and non-ASCII characters are supported. Missing or unreadable paths are reported and make the command exit non-zero; a successful report exits zero.

The report uses macOS `du -skP`, so each result is allocated disk usage in KiB plus a human-readable unit. Trailing separators are removed before link checks and measurement, while the filesystem root remains `/`; the final symlink is not canonicalized. The `-P` policy does not follow symbolic links, including links below a selected directory; an explicitly supplied symlink is measured as the link itself. `du` receives a safe absolute lexical operand, so directories named `-s` or `-L` remain data paths. Nested or duplicate inputs are marked as overlapping and are printed separately rather than summed. Overlap labels normalize `.` and duplicate separators but preserve `..` and do not resolve symlink targets. No aggregate total is produced, and allocated usage is not the same as download size.

The command only measures paths explicitly supplied by the developer. It does not delete, compress, change permissions, launch an app, build Rust code, inspect file contents, inspect databases or credentials, or download models.
