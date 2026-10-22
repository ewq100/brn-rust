"""Focused stdlib tests for the generated development owner view."""

import contextlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "render-work-overview.py"
SPEC = importlib.util.spec_from_file_location("render_work_overview", SCRIPT)
renderer = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(renderer)


def task(task_id="one", **updates):
    value = {
        "id": task_id, "title": "First task", "state": "active", "owner": "Ari",
        "activity": "Reviewing the source", "observed_at": "2026-10-20T10:00:00+03:00",
        "origin": "observed", "next_action": "Run focused check", "depends_on": [],
        "resource_waits": [], "evidence": ["docs/work/active/example.md"],
        "gates": {key: f"{key} claim" for key in renderer.GATES}, "attempts": [],
    }
    value.update(updates)
    return value


def view(*tasks, owner_attention=None):
    result = {"observed_at": "2026-10-20T10:01:00+03:00", "goal": "Ship a useful owner view", "tasks": list(tasks)}
    if owner_attention is not None:
        result["owner_attention"] = owner_attention
    return result


class WorkOverviewTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.record = self.root / "campaign.json"
        self.output = self.root / "docs/work/overview.md"

    def tearDown(self):
        self.temp.cleanup()

    def save(self, work_view):
        self.record.write_text(json.dumps({"work_view": work_view}), encoding="utf-8")

    def run_main(self, *args):
        stdout, stderr = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            result = renderer.main(["--record", str(self.record), "--output", str(self.output), *args])
        return result, stdout.getvalue(), stderr.getvalue()

    def test_rejects_missing_id_cycle_and_naive_timestamp(self):
        missing = task()
        del missing["id"]
        with self.assertRaisesRegex(renderer.InvalidRecord, "missing required fields"):
            renderer.validate(view(missing))
        a = task("a", depends_on=[{"task": "b", "condition": "b complete", "satisfied": False}])
        b = task("b", depends_on=[{"task": "a", "condition": "a complete", "satisfied": False}])
        with self.assertRaisesRegex(renderer.InvalidRecord, "cycle"):
            renderer.validate(view(a, b))
        with self.assertRaisesRegex(renderer.InvalidRecord, "timezone"):
            renderer.validate(view(task(observed_at="2026-10-20T10:00:00")))

    def test_rejects_duplicate_ids_unknown_dependencies_and_bad_evidence(self):
        with self.assertRaisesRegex(renderer.InvalidRecord, "duplicate task id"):
            renderer.validate(view(task("same"), task("same")))
        with self.assertRaisesRegex(renderer.InvalidRecord, "unknown task"):
            renderer.validate(view(task(depends_on=[{"task": "missing", "condition": "complete", "satisfied": False}])))
        with self.assertRaisesRegex(renderer.InvalidRecord, "supports only HTTPS"):
            renderer.validate(view(task(evidence=["javascript:alert(1)"])))
        with self.assertRaisesRegex(renderer.InvalidRecord, "must be a boolean"):
            renderer.validate(view(task(depends_on=[{"task": "known", "condition": "done", "satisfied": "yes"}])))
        with self.assertRaisesRegex(renderer.InvalidRecord, "must contain task, condition, and satisfied"):
            renderer.validate(view(task(depends_on=[{"task": "known", "condition": "done"}])))

    def test_render_keeps_observation_static_gates_distinct_and_resources_out_of_graph(self):
        first = task(resource_waits=["native Mac slot"], gates={key: key.upper() for key in renderer.GATES})
        second = task("two", title="Dependent", state="ready", depends_on=[{"task": "one", "condition": "reviewed first task", "satisfied": False}])
        output = renderer.render(view(first, second), self.output)
        self.assertIn("Last observed: 2026-10-20T10:00:00+03:00", output)
        self.assertIn("Local page reflects the last saved observations in this record (2026-10-20T10:01:00+03:00)", output)
        self.assertIn("A remote page reflects the last published commit", output)
        self.assertIn("Resource waits: native Mac slot", output)
        self.assertIn("**integration:** INTEGRATION", output)
        self.assertIn("**owner acceptance:** OWNER_ACCEPTANCE", output)
        self.assertIn("t0 -->|reviewed first task (not satisfied)| t1", output)
        self.assertNotIn("### Dependent", output.split("## Next", 1)[1].split("## Needs attention", 1)[0])
        self.assertIn("prerequisite one: reviewed first task (reported satisfied: false; task state: active)", output)
        self.assertNotIn("native Mac slot -->", output)
        self.assertIn("[docs/work/active/example.md]", output)
        self.assertEqual(output, renderer.render(view(first, second), self.output))

    def test_malformed_update_preserves_old_file_and_check_detects_staleness_without_write(self):
        self.save(view(task()))
        self.assertEqual(self.run_main()[0], 0)
        old = self.output.read_bytes()
        self.save(view(task(depends_on=[{"task": "missing", "condition": "is complete", "satisfied": False}])))
        result, _, error = self.run_main()
        self.assertEqual(result, 2)
        self.assertIn("unknown task", error)
        self.assertEqual(self.output.read_bytes(), old)

        self.record.write_text("{broken json", encoding="utf-8")
        result, _, error = self.run_main()
        self.assertEqual(result, 2)
        self.assertIn("cannot read record", error)
        self.assertEqual(self.output.read_bytes(), old)

        self.save(view(task(title="Updated title")))
        result, _, error = self.run_main("--check")
        self.assertEqual(result, 1)
        self.assertIn("stale work overview", error)
        self.assertEqual(self.output.read_bytes(), old)

    def test_next_requires_complete_prerequisite_and_no_resource_wait(self):
        done = task("done", state="complete")
        ready_after_done = task("ready-after", title="Ready-after", state="ready", depends_on=[{"task": "done", "condition": "review completed", "satisfied": True}])
        ready_waiting = task("ready-waiting", title="Ready-waiting", state="ready", resource_waits=["build slot"])
        isolated = task("isolated", title="Isolated work")
        output = renderer.render(view(done, ready_after_done, ready_waiting, isolated, owner_attention="No owner decision requested; remaining waits are checks or capacity."), self.output)
        next_section = output.split("## Next", 1)[1].split("## Needs attention", 1)[0]
        attention = output.split("## Needs attention", 1)[1].split("## Completed", 1)[0]
        self.assertIn("### Ready-after", next_section)
        self.assertNotIn("### Ready-waiting", next_section)
        self.assertIn("### Ready-waiting", attention)
        self.assertIn("- resource: build slot", attention)
        self.assertIn("**Owner attention:** No owner decision requested; remaining waits are checks or capacity.", output)
        graph = output.split("```mermaid", 1)[1].split("```", 1)[0]
        self.assertNotIn("Isolated work", graph)
        attention_active = output.split("## Needs attention", 1)[1].split("## Completed", 1)[0]
        self.assertNotIn("### Isolated work", attention_active)

    def test_reported_condition_satisfaction_not_prerequisite_state_controls_next(self):
        complete_task = task("done", state="complete")
        waiting = task("waiting", title="Waiting", state="ready", depends_on=[{"task": "done", "condition": "owner acceptance complete", "satisfied": False}])
        output = renderer.render(view(complete_task, waiting), self.output)
        next_section = output.split("## Next", 1)[1].split("## Needs attention", 1)[0]
        attention = output.split("## Needs attention", 1)[1].split("## Completed", 1)[0]
        self.assertNotIn("### Waiting", next_section)
        self.assertIn("owner acceptance complete (reported satisfied: false; task state: complete)", attention)

        waiting["depends_on"][0]["satisfied"] = True
        output = renderer.render(view(complete_task, waiting), self.output)
        next_section = output.split("## Next", 1)[1].split("## Needs attention", 1)[0]
        self.assertIn("### Waiting", next_section)

    def test_current_check_does_not_rewrite(self):
        self.save(view(task()))
        self.assertEqual(self.run_main()[0], 0)
        before = self.output.stat().st_mtime_ns
        self.assertEqual(self.run_main("--check")[0], 0)
        self.assertEqual(self.output.stat().st_mtime_ns, before)

    def test_task_lookup_prints_exact_selected_object_and_preserves_record_and_output(self):
        first = task()
        selected = task("two", title="Second task", activity="Review õun")
        self.save(view(first, selected))
        record_before = self.record.read_bytes()
        self.output.parent.mkdir(parents=True)
        self.output.write_bytes(b"existing output\x00\xff")
        output_before = self.output.read_bytes()

        result, stdout, stderr = self.run_main("--task", "two")

        self.assertEqual(result, 0, stderr)
        self.assertEqual(stdout, json.dumps(selected, ensure_ascii=False, indent=2) + "\n")
        self.assertEqual(stderr, "")
        self.assertEqual(self.record.read_bytes(), record_before)
        self.assertEqual(self.output.read_bytes(), output_before)

    def test_task_lookup_does_not_create_missing_output_parent(self):
        self.save(view(task()))
        self.assertFalse(self.output.parent.exists())

        result, stdout, stderr = self.run_main("--task", "one")

        self.assertEqual(result, 0, stderr)
        self.assertEqual(json.loads(stdout), task())
        self.assertEqual(stderr, "")
        self.assertFalse(self.output.parent.exists())

    def test_task_lookup_unknown_id_returns_two_without_output(self):
        self.save(view(task()))
        record_before = self.record.read_bytes()

        result, stdout, stderr = self.run_main("--task", "missing")

        self.assertEqual(result, 2)
        self.assertEqual(stdout, "")
        self.assertIn("unknown task ID: missing", stderr)
        self.assertEqual(self.record.read_bytes(), record_before)
        self.assertFalse(self.output.parent.exists())

    def test_task_lookup_validates_other_tasks_before_printing_selection(self):
        invalid_sibling = task("broken", evidence=[])
        self.save(view(task(), invalid_sibling))

        result, stdout, stderr = self.run_main("--task", "one")

        self.assertEqual(result, 2)
        self.assertEqual(stdout, "")
        self.assertIn("tasks[1].evidence", stderr)
        self.assertFalse(self.output.parent.exists())

    def test_task_lookup_and_check_are_mutually_exclusive(self):
        self.save(view(task()))
        stdout, stderr = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            with self.assertRaises(SystemExit) as raised:
                renderer.main([
                    "--record", str(self.record), "--output", str(self.output),
                    "--task", "one", "--check",
                ])

        self.assertEqual(raised.exception.code, 2)
        self.assertEqual(stdout.getvalue(), "")
        self.assertIn("not allowed with argument", stderr.getvalue())
        self.assertFalse(self.output.parent.exists())

    def test_completed_tasks_use_compact_index_and_link_selected_record(self):
        completed = task("done", state="complete", attempts=[{
            "outcome": "corrected", "change": "Updated the source", "evidence": ["docs/work/active/proof.md"]
        }])
        work_view = view(completed)
        self.save(work_view)
        original_record = self.record.read_bytes()
        result, _, error = self.run_main()
        self.assertEqual(result, 0, error)
        output = self.output.read_text(encoding="utf-8")
        section = output.split("## Completed", 1)[1]
        self.assertIn("Completion here means the selected task scope is complete", section)
        self.assertIn("does not establish every gate, native observation, or owner acceptance", section)
        self.assertIn("[selected work record](../../campaign.json)", section)
        self.assertIn("- `done` — First task", section)
        self.assertNotIn("<details>", section)
        self.assertNotIn("implementation claim", section)
        self.assertNotIn("docs/work/active/example.md", section)
        self.assertNotIn("Updated the source", section)
        self.assertEqual(self.record.read_bytes(), original_record)
        retained = json.loads(self.record.read_text(encoding="utf-8"))["work_view"]["tasks"][0]
        self.assertEqual(retained["gates"]["implementation"], "implementation claim")
        self.assertEqual(retained["evidence"], ["docs/work/active/example.md"])
        self.assertEqual(retained["attempts"][0]["change"], "Updated the source")
        self.assertIn("Local page reflects the last saved observations", output)
        self.assertIn("A remote page reflects the last published commit", output)
        self.assertIn("Neither indicates that work is live now", output)


if __name__ == "__main__":
    unittest.main()
