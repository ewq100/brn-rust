# Shared provider fixture readers

Source `22d5cd5f3761ed38800c7b934f88b52107e82726`, integrated as `54eb89cc`,
replaces five local fixture-reading definitions with two shared parent functions
in provider_formats_tests.rs. Three preamble readers differed only in equivalent
String conversion/variable spelling; two tool-definition readers were identical.
All existing callers, assertions and test cases remain. No production code,
provider/model policy, dependencies or tests were added or retired.

Action, knowledge and conflict families retain their provider-wire guidance,
closed-schema/presence/gating and scoped conflict/cursor assertions. They passed
4, 7 and 12 tests respectively, with strict capability-spike all-target Clippy,
format and whitespace checks passing. Independent Sol approved complete source945,
including helper equivalence and removal of the five obsolete copies.

[Execution](evidence/provider-fixture-execution.txt), [worker receipt](evidence/provider-fixture-receipt.md)
and [hash/results](evidence/provider-fixture-qualification.json) retain actual commands,
source patch and compressed logs. commands.txt also contains a full CI reference
command that was not run; focused nonzero test results are the local qualification.
An initial wrong-directory knowledge invocation failed at workspace resolution;
its worker-copied tool output and successful assigned-checkout rerun remain explicit.
No native rebuild is warranted for this cfg(test)-only move; the separate export
source has its own native qualification. No live provider or external network
qualification is claimed. Hosted/main gates remain separate.
