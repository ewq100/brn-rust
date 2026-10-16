# Individual slow-test sample

30 exact tests from the three slowest observed default-lane binaries passed. This samples those binaries only; it is not a global ranking. See [environment limitation](slow-environment.json): CLI artifact was rebuilt with native-retrieval before sampling. Exact executable paths/argv/output/exit/time are in [raw sample](slow-tests.json).

| Test | Process wall seconds |
|---|---:|
| `agent_bounds::begin_edit_under_a_held_lock_is_busy` | 12.2197 |
| `agent_bounds::held_write_lock_reports_busy_and_creates_nothing` | 10.5652 |
| `agent_bounds::busy_then_keyed_retry_applies_once` | 5.3641 |
| `edit_tokens::schema_5_upgrade_backs_up_first_and_ends_at_6_as_an_owner_workspace` | 5.29 |
| `agent_bounds::sigint_then_deadline_still_hard_stops` | 4.1597 |
| `agent_bounds::held_lock_past_deadline_hard_stops_with_one_line` | 4.1584 |
| `agent_bounds::sigint_then_sigterm_still_hard_stops` | 3.6732 |
| `agent_bounds::timeout_during_fifo_wait_is_a_soft_deadline` | 2.1085 |
| `agent_journeys::a_stale_candidate_is_refreshed_from_current_search_results` | 1.4418 |
| `agent_journeys::maintenance_applies_within_eight_rounds_and_finds_its_own_change` | 0.9924 |
