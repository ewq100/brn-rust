#!/usr/bin/env python3
"""Render a small, honest owner view from the campaign work_view record."""

import argparse
from datetime import datetime
import json
import os
from pathlib import Path, PurePosixPath
import re
import sys
import tempfile
from urllib.parse import quote


ROOT = Path(__file__).resolve().parent.parent
DEFAULT_RECORD = ROOT / "docs/work/active/vacation-campaign/campaign.json"
DEFAULT_OUTPUT = ROOT / "docs/work/overview.md"
STATES = {"active", "ready", "blocked", "complete"}
ORIGINS = {"reported", "observed"}
GATES = ("implementation", "checks", "integration", "native", "owner_acceptance")
REQUIRED_TASK = ("id", "title", "state", "owner", "activity", "observed_at", "origin", "next_action", "depends_on", "resource_waits", "evidence", "gates", "attempts")


class InvalidRecord(ValueError):
    pass


def _text(value, field, allow_empty=False):
    if not isinstance(value, str) or (not allow_empty and not value.strip()):
        raise InvalidRecord(f"{field} must be {'a string' if allow_empty else 'non-empty text'}")
    return value


def _timestamp(value, field):
    _text(value, field)
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise InvalidRecord(f"{field} must be an ISO-8601 timestamp") from exc
    if parsed.tzinfo is None or parsed.utcoffset() is None:
        raise InvalidRecord(f"{field} must include a timezone")


def _evidence(value, field):
    if not isinstance(value, list) or not value:
        raise InvalidRecord(f"{field} must be a non-empty list")
    for item in value:
        _text(item, field)
        if "\n" in item or "\r" in item:
            raise InvalidRecord(f"{field} entries cannot contain newlines")
        if item.startswith("https://"):
            continue
        if re.match(r"^[A-Za-z][A-Za-z0-9+.-]*:", item):
            raise InvalidRecord(f"{field} supports only HTTPS URLs or repository-relative paths")
        path = PurePosixPath(item)
        if path.is_absolute() or ".." in path.parts or item.startswith("//"):
            raise InvalidRecord(f"{field} paths must be repository-relative")


def validate(view):
    if not isinstance(view, dict):
        raise InvalidRecord("work_view must be an object")
    _timestamp(view.get("observed_at"), "work_view.observed_at")
    _text(view.get("goal"), "work_view.goal")
    if "owner_attention" in view:
        _text(view["owner_attention"], "work_view.owner_attention")
    tasks = view.get("tasks")
    if not isinstance(tasks, list):
        raise InvalidRecord("work_view.tasks must be a list")
    by_id = {}
    for index, task in enumerate(tasks):
        label = f"tasks[{index}]"
        if not isinstance(task, dict):
            raise InvalidRecord(f"{label} must be an object")
        missing = [field for field in REQUIRED_TASK if field not in task]
        if missing:
            raise InvalidRecord(f"{label} missing required fields: {', '.join(missing)}")
        task_id = _text(task["id"], f"{label}.id")
        if not re.fullmatch(r"[a-z0-9-]+", task_id):
            raise InvalidRecord(f"{label}.id must use lowercase letters, digits, and hyphens")
        if task_id in by_id:
            raise InvalidRecord(f"duplicate task id: {task_id}")
        by_id[task_id] = task
        for field in ("title", "owner", "activity", "next_action"):
            _text(task[field], f"{label}.{field}")
        _timestamp(task["observed_at"], f"{label}.observed_at")
        if not isinstance(task["state"], str) or task["state"] not in STATES:
            raise InvalidRecord(f"{label}.state must be one of {sorted(STATES)}")
        if not isinstance(task["origin"], str) or task["origin"] not in ORIGINS:
            raise InvalidRecord(f"{label}.origin must be one of {sorted(ORIGINS)}")
        dependencies = task["depends_on"]
        if not isinstance(dependencies, list):
            raise InvalidRecord(f"{label}.depends_on must be a list of objects")
        dependency_ids = []
        for dependency_index, dependency in enumerate(dependencies):
            where = f"{label}.depends_on[{dependency_index}]"
            if not isinstance(dependency, dict) or set(dependency) != {"task", "condition", "satisfied"}:
                raise InvalidRecord(f"{where} must contain task, condition, and satisfied")
            dependency_id = _text(dependency["task"], f"{where}.task")
            if not re.fullmatch(r"[a-z0-9-]+", dependency_id):
                raise InvalidRecord(f"{where}.task must use lowercase letters, digits, and hyphens")
            _text(dependency["condition"], f"{where}.condition")
            if not isinstance(dependency["satisfied"], bool):
                raise InvalidRecord(f"{where}.satisfied must be a boolean")
            dependency_ids.append(dependency_id)
        if len(set(dependency_ids)) != len(dependency_ids):
            raise InvalidRecord(f"{label}.depends_on contains duplicate tasks")
        waits = task["resource_waits"]
        if not isinstance(waits, list) or any(not isinstance(item, str) or not item.strip() for item in waits):
            raise InvalidRecord(f"{label}.resource_waits must be a list of non-empty strings")
        _evidence(task["evidence"], f"{label}.evidence")
        gates = task["gates"]
        if not isinstance(gates, dict) or set(gates) != set(GATES) or any(not isinstance(gates[k], str) or not gates[k].strip() for k in GATES):
            raise InvalidRecord(f"{label}.gates must contain non-empty string values for {', '.join(GATES)}")
        attempts = task["attempts"]
        if not isinstance(attempts, list):
            raise InvalidRecord(f"{label}.attempts must be a list")
        for attempt_index, attempt in enumerate(attempts):
            where = f"{label}.attempts[{attempt_index}]"
            if not isinstance(attempt, dict) or not all(key in attempt for key in ("outcome", "change", "evidence")):
                raise InvalidRecord(f"{where} must contain outcome, change, and evidence")
            _text(attempt["outcome"], f"{where}.outcome")
            _text(attempt["change"], f"{where}.change")
            _evidence(attempt["evidence"], f"{where}.evidence")
    for task_id, task in by_id.items():
        for dependency in task["depends_on"]:
            if dependency["task"] not in by_id:
                raise InvalidRecord(f"task {task_id} depends on unknown task {dependency['task']}")
    marks = {}

    def visit(task_id):
        mark = marks.get(task_id, 0)
        if mark == 1:
            raise InvalidRecord(f"dependency cycle includes task {task_id}")
        if mark == 2:
            return
        marks[task_id] = 1
        for dependency in by_id[task_id]["depends_on"]:
            visit(dependency["task"])
        marks[task_id] = 2

    for task_id in by_id:
        visit(task_id)
    return view


def _safe_label(text):
    return text.replace("&", "&amp;").replace('"', "'").replace("<", "&lt;").replace(">", "&gt;").replace("\n", " ")


def _link(item, output_path):
    if item.startswith("https://"):
        return f"[{item}]({item})"
    target = ROOT / PurePosixPath(item)
    relative = os.path.relpath(target, output_path.parent)
    return f"[{item}]({quote(relative, safe='/._-()')})"


def _task_block(task, output_path):
    lines = [f"### {task['title']}", "", f"Owner: {task['owner']} · State: **{task['state']}** · Last {task['origin']}: {task['observed_at']}",
             f"Activity: {task['activity']}", f"Next: {task['next_action']}"]
    if task["depends_on"]:
        lines.append("Depends on: " + "; ".join(f"{dep['task']} when {dep['condition']} (reported satisfied: {str(dep['satisfied']).lower()})" for dep in task["depends_on"]))
    if task["resource_waits"]:
        lines.append("Resource waits: " + "; ".join(task["resource_waits"]))
    lines.extend(("", "<details>", "<summary>Checks, evidence and corrections</summary>", "", "Gates:", ""))
    lines.extend(f"- **{key.replace('_', ' ')}:** {task['gates'][key]}" for key in GATES)
    lines.extend(("", "Evidence: " + ", ".join(_link(item, output_path) for item in task["evidence"])))
    if task["attempts"]:
        lines.extend(("", "Attempts:", ""))
        lines.extend(f"- {a['outcome']}: {a['change']} ({', '.join(_link(e, output_path) for e in a['evidence'])})" for a in task["attempts"])
    lines.extend(("", "</details>"))
    return "\n".join(lines)


def render(view, output_path, record_path=DEFAULT_RECORD):
    validate(view)
    output_path = Path(output_path).resolve()
    tasks = view["tasks"]
    by_id = {task["id"]: task for task in tasks}

    def unmet(task):
        waiting = [f"prerequisite {dep['task']}: {dep['condition']} (reported satisfied: false; task state: {by_id[dep['task']]['state']})"
                   for dep in task["depends_on"] if not dep["satisfied"]]
        waiting.extend(f"resource: {resource}" for resource in task["resource_waits"])
        return waiting

    def eligible(task):
        return task["state"] == "ready" and not unmet(task)

    out = ["# Development overview", "", f"**Goal:** {view['goal']}"]
    if "owner_attention" in view:
        out.append(f"**Owner attention:** {view['owner_attention']}")
    out.extend(("", "Activity below is the last reported or observed update. It does not establish that work is running now.", "", "Ready eligibility is a conservative state/admission display derived from reported prerequisites and resource waits; actual job and quota availability must still be rechecked.", ""))
    sections = ("Working on", "Next", "Needs attention", "Completed")
    for heading in sections:
        out.extend((f"## {heading}", ""))
        if heading == "Needs attention":
            selected = [task for task in tasks if task["state"] == "blocked" or (task["state"] in {"active", "ready"} and task["resource_waits"]) or (task["state"] == "ready" and not eligible(task))]
        elif heading == "Working on":
            selected = [task for task in tasks if task["state"] == "active"]
        elif heading == "Next":
            selected = [task for task in tasks if eligible(task)]
        else:
            selected = [task for task in tasks if task["state"] == "complete"]
        if not selected:
            out.extend(("None recorded.", ""))
        elif heading == "Completed":
            record_target = os.path.relpath(Path(record_path).resolve(), output_path.parent)
            record_link = f"[selected work record]({quote(record_target, safe='/._-()')})"
            out.extend((
                "Completion here means the selected task scope is complete; it does not establish every gate, native observation, or owner acceptance.",
                f"Full task details, gates, evidence, and attempts remain in the {record_link}. Inspect one validated task with `python3 scripts/render-work-overview.py --record PATH_TO_SELECTED_RECORD --task TASK_ID`, replacing PATH_TO_SELECTED_RECORD with the linked record path and TASK_ID with an ID listed below.",
                "",
            ))
            out.extend(f"- `{task['id']}` — {task['title']}" for task in selected)
            out.append("")
        else:
            for task in selected:
                if heading == "Needs attention":
                    waiting = unmet(task)
                    if waiting:
                        if task["state"] == "active":
                            out.extend((f"### {task['title']}", "", f"Owner: {task['owner']}", "", "Waiting:", "", *[f"- {reason}" for reason in waiting], ""))
                        else:
                            out.extend((_task_block(task, output_path), "", "Waiting:", "", *[f"- {reason}" for reason in waiting], ""))
                    else:
                        out.extend((_task_block(task, output_path), ""))
                else:
                    out.extend((_task_block(task, output_path), ""))
    edges = [(task, dep) for task in tasks for dep in task["depends_on"]]
    if edges:
        ids = {task["id"]: f"t{index}" for index, task in enumerate(tasks)}
        connected = {task["id"] for task, _ in edges} | {dep["task"] for _, dep in edges}
        out.extend(("## Dependencies", "", "```mermaid", "graph TD"))
        for task in tasks:
            if task["id"] in connected:
                out.append(f"  {ids[task['id']]}[\"{_safe_label(task['title'])}\"]")
        for task, dep in edges:
            status = "satisfied" if dep["satisfied"] else "not satisfied"
            out.append(f"  {ids[dep['task']]} -->|{_safe_label(dep['condition'] + ' (' + status + ')')}| {ids[task['id']]}")
        out.extend(("```", "", "Resource waits are listed on tasks and are not dependency edges.", ""))
    out.extend((f"Local page reflects the last saved observations in this record ({view['observed_at']}). A remote page reflects the last published commit. Neither indicates that work is live now.", ""))
    return "\n".join(out)


def read_record(record_path):
    try:
        record = json.loads(Path(record_path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise InvalidRecord(f"cannot read record: {exc}") from exc
    if not isinstance(record, dict) or "work_view" not in record:
        raise InvalidRecord("record must contain work_view")
    return validate(record["work_view"])


def write_atomic(path, content):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    temp_name = None
    try:
        with tempfile.NamedTemporaryFile("w", encoding="utf-8", dir=path.parent, delete=False) as temp:
            temp.write(content)
            temp.flush()
            os.fsync(temp.fileno())
            temp_name = temp.name
        os.replace(temp_name, path)
    finally:
        if temp_name and os.path.exists(temp_name):
            os.unlink(temp_name)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--record", type=Path, default=DEFAULT_RECORD)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true", help="report stale output without writing")
    mode.add_argument("--task", help="print one validated task object by ID without rendering")
    args = parser.parse_args(argv)
    try:
        view = read_record(args.record)
        if args.task is not None:
            selected = next((task for task in view["tasks"] if task["id"] == args.task), None)
            if selected is None:
                raise InvalidRecord(f"unknown task ID: {args.task}")
            print(json.dumps(selected, ensure_ascii=False, indent=2))
            return 0
        content = render(view, args.output, args.record)
    except InvalidRecord as exc:
        print(f"invalid work overview: {exc}", file=sys.stderr)
        return 2
    if args.check:
        try:
            current = args.output.read_text(encoding="utf-8")
        except OSError:
            current = None
        if current != content:
            print(f"stale work overview: {args.output}", file=sys.stderr)
            return 1
        print(f"work overview is current: {args.output}")
        return 0
    write_atomic(args.output, content)
    print(f"updated work overview: {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
