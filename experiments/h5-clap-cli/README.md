# H5 — clap as BRN's generic CLI scanner

**Result (2026-10-06): not adopted.** Keep BRN's existing scanner. The evaluator
adds no product code and no product dependency.

Question from the [V1 handoff](../../docs/work/active/v1-handoff.md#h5--evaluate-replacing-generic-cli-scanning):
does [clap 4.6.7](https://docs.rs/crate/clap/4.6.7) reduce total maintenance
while keeping BRN's errors, help and validation order? Decision record:
[compatible reuse, section A](../../docs/architecture/decisions/2026-10-06-compatible-reuse.md#a--cli-parsing-evaluated-not-adopted-h5).

## What was built

- `src/lib.rs`: a clap-based parser for a representative subset of commands:
  all global options, `help`, `status`, `search`, `notes show`, the nested
  `inbox add|list|show` and `findings close`, whose local `--version` is a
  review stamp. It uses the published `Command::try_get_matches_from` API with
  `default-features = false` and `std` and `error-context` enabled. The file has
  three sections: `DECLARATIONS` (command tables and clap tree), `ADAPTER` (the
  glue needed to reproduce BRN's contract) and `TYPED` (BRN's typed validation
  and `--data-dir` admission, copied for the subset, which would not change in
  a migration).
- `tests/matrix.rs`: 88 synthetic argv cases. The real `brn` binary is the
  oracle. Each case runs as given and again with `--json` appended, and must
  produce the same help, version or usage outcome, envelope `command` and
  message. Every case stops before storage, and an existing empty data
  directory must still be empty after each oracle run. Divergences are listed
  explicitly, so the test fails if the result changes.
- `tests/idiomatic.rs`: nine witnesses showing how idiomatic clap configuration
  departs from BRN's contract. Each departure required a piece of the adapter.
- [MATRIX.md](MATRIX.md): the generated table for every case.

## Reproduce

Baseline: `main@450eaa2dcd4d9571ae749c57806a192a4dd8e435`, macOS 26.5 arm64,
Rust 1.98.1. Use a separate Cargo target for the product and for the evaluator.

```sh
# From the repository root: build the oracle binary.
CARGO_TARGET_DIR=/private/tmp/brn-h5-target cargo build -p brn --locked --offline
cd experiments/h5-clap-cli
mkdir -p /private/tmp/brn-h5-fixtures
CARGO_TARGET_DIR=/private/tmp/brn-h5-eval-target cargo clippy --locked --offline --all-targets -- -D warnings
TMPDIR=/private/tmp/brn-h5-fixtures BRN_BIN=/private/tmp/brn-h5-target/debug/brn \
  CARGO_TARGET_DIR=/private/tmp/brn-h5-eval-target cargo test --locked --offline -- --nocapture
```

`BRN_BIN` is required, so the matrix cannot pass without the oracle. The first
dependency fetch needs crates.io; later runs work offline.

Observed on 2026-10-06: Clippy clean; `idiomatic` 9/9 passed; `matrix` passed
with 88 cases, 84 identical and the 4 listed divergences. The product's
`cargo test -p brn --test cli_basic --locked --offline` passed 6/6 on the same
tree, confirming the oracle's own help, usage and envelope contract.

## Findings

### Idiomatic clap departs from the contract

| clap behavior (witness) | BRN contract | Adapter response |
| --- | --- | --- |
| `ArgAction::Help` loses to an earlier unknown token; help is generated per command | Any exact `--help` prints the single static catalogue first | Keep BRN's exact-token pre-scan and static help; declare `--help` only to reject `--help=value` |
| `global(true)` values propagate and the later level silently overwrites; a repeated propagated flag is accepted | A duplicate global before and after the command is refused | Declare globals on the root and on every leaf, then merge and detect duplicates in the adapter |
| Values starting with `-` are refused by default (`--title -draft`, `search -foo`) | Only tokens starting with `--` are options | Enable hyphen values everywhere |
| Hyphen values also accept `--json` as a value; a hyphen positional absorbs every following option | `--limit --json` is `missing value for --limit`; options after positionals still parse | Use three single-value positional slots; re-check every value and positional for a `--` prefix and keep `--x=--y` inline values |
| A bare `--` is a silent escape | `--` is `unknown option: --` | Pre-check raw argv |
| A repeated `SetTrue` flag is refused | `--version` may repeat | Use `Count` for `--help` and `--version` |
| Errors carry no subcommand path and report `--name <name>` or `--name...` | The envelope `command` and some messages depend on how far the root scan got; messages echo the exact token | Re-walk root globals and command words to find the command, and look up raw tokens for messages |

### Remaining divergences

All four are usage errors with the same exit code and the same envelope
`command`. Only the reported message differs, because clap or the adapter
detects a different problem first:

| Case | BRN | Candidate |
| --- | --- | --- |
| `--data-dir D status --data-dir D --bogus` | duplicate option: --data-dir | unknown option: --bogus |
| `status --bogus --` | unknown option: --bogus | unknown option: -- |
| `status a b c d` | unexpected argument: a | unexpected argument: d |
| `status --data-dir D --data-dir`, run with `--json` appended | missing value for --data-dir | duplicate option: --data-dir |

Matching these would require scanning tokens left to right with BRN's rules,
which is the job of the existing scanner.

### Maintenance cost

Counts exclude blank and comment lines.

| Item | Current BRN | clap candidate |
| --- | --- | --- |
| Generic scanning code | about 130 lines in `cli/mod.rs` (`split_option`, `take_value`, `set_global`, `global_option`, `scan`, `sub_word`, root loop) | 45 lines to build the clap tree, plus about 195 lines of adapter (including about 5 lines of pre-scan that BRN already has) |
| Command declarations | 314 lines across 12 `scan_command` functions, plus the dispatch match | Same tables are still needed to generate clap arguments; no reduction |
| Help text and messages | Static `HELP` and local messages | Same static `HELP`; messages rebuilt from clap error kinds |
| Dependencies | none | `clap`, `clap_builder`, `clap_lex` (about 31,500 source lines); `anstyle` is already locked |
| Contract fidelity | reference | 84/88 identical; 4 ordering divergences |

Compile time is not a factor: a clean build of the evaluator library took about
1.4 s (dev) and 2.0 s (release) on this Mac.

clap's main benefits are generated help, typed value parsers, required-argument
checks, suggestions and completions. Each conflicts with a BRN requirement:
static help and help precedence, BRN's typed validation order and messages, and
validation before workspace access. Using `clap_lex` alone would replace only
`split_option` (7 lines), so it gives no net benefit.

## Decision

Do not adopt clap. The scanner stays a small local implementation because it is
demonstrably simpler: adopting clap would add a dependency and more custom code
than it removes, and would still change error ordering. There is no migration
boundary to implement.

Reopen only for a changed requirement or new evidence. Examples: the owner
accepts clap-generated help, error wording or ordering; BRN adds features such as
shell completion that the local scanner cannot reasonably provide; or a clap
release offers a lexer mode that fits BRN's option rules.

## Limitations

- Eight leaf commands were evaluated, not all of them. The adapter is a fixed
  cost; the excluded leaves differ only in their option tables.
- The candidate copies BRN's typed rules only for the subset. Other note-path,
  folder and cursor validation was not exercised.
- Both parsers take `String` argv, so non-UTF-8 arguments were not compared.
- Default clap features (help, usage, colour, suggestions) were not built,
  because BRN keeps its own help text and messages.
