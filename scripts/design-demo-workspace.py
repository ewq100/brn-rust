#!/usr/bin/env python3
"""Create a disposable synthetic BRN workspace for design screenshots and review.

The content is fictional (Serna house project, Anna Tamm, paint purchasing).
Only owner CLI contracts are used: no provider call, model download, network
access or private data. The root must be new or empty.

Usage: python3 scripts/design-demo-workspace.py --root /abs/new/dir [--brn PATH]
Prints the data and vault directories as JSON on success.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import uuid
from pathlib import Path

NOTES = {
    "projects/serna.md": """---
brn_id: 5e2a0000-0000-4000-8000-000000000001
---
# Serna house renovation

**Status:** Active · **Owner:** Evo · **Updated:** 2026-10-02

## Current state

- Exterior repaint scheduled for late October 2026.
- House colour: **dark red** (decided 2026-09-12 meeting).
- Supplier: Värvikoda OÜ (quote accepted 2026-09-20).

## Key decisions

- [Exterior colour decision](../decisions/serna-exterior-colour.md)
- [Paint supplier selection](../decisions/serna-paint-supplier.md)

## People

- [Anna Tamm](../people/anna-tamm.md) — architect, owns the paint specification.
""",
    "people/anna-tamm.md": """---
brn_id: 5e2a0000-0000-4000-8000-000000000002
---
# Anna Tamm

- **Role:** Architect, Tamm & Partners
- **Email:** anna@tamm-partners.example
- **Projects:** [Serna house renovation](../projects/serna.md)

## Commitments

- Final paint specification for Serna (promised by 2026-10-04).
""",
    "decisions/serna-exterior-colour.md": """---
brn_id: 5e2a0000-0000-4000-8000-000000000003
---
# Serna exterior colour

Decided at the 2026-09-12 site meeting: the exterior is painted **dark red**
(RAL 3011). Alternatives discussed: green, blue.
""",
    "decisions/serna-paint-supplier.md": """---
brn_id: 5e2a0000-0000-4000-8000-000000000004
---
# Serna paint supplier

Värvikoda OÜ selected on 2026-09-20 after comparing three quotes.
Delivery lead time: 10 working days from a confirmed specification.
""",
    "processes/purchasing.md": """# Purchasing process (general)

1. Requester writes a purchasing request with budget code.
2. Team lead approves requests under 5 000 EUR; above that the CEO approves.
3. Finance places the order and files the invoice.
""",
    "meetings/2026-10-01-serna-site-meeting.md": """# Serna site meeting — 2026-10-01

Attendees: Evo, Anna Tamm, Märt (builder)

- Scaffolding goes up on 2026-10-20.
- Anna to send final paint specification this week.
- Green versus red discussed again; no new decision recorded.
""",
    "sources/2026-09-12-site-meeting-notes.md": """---
brn_kind: source
brn_id: 5e2a0000-0000-4000-8000-000000000005
---
Original meeting notes (imported 2026-09-12)

Colour: team agreed dark red, RAL 3011. Anna will confirm the exact product line.
""",
    "archive/serna-plan-v1.md": """---
brn_state: history
---
# Serna plan v1 (superseded)

Original plan from June 2026: repaint in spring 2027, colour undecided.
""",
}

ANNA_EMAIL = """From: Anna Tamm <anna@tamm-partners.example>
To: Evo <evo@example.com>
Subject: Serna – three questions before I finalise the paint spec
Date: Mon, 6 Oct 2026 09:14:00 +0300

Hi Evo,

Before I send the final specification I need three answers:

1. Is the colour still dark red (RAL 3011), or did the team move to blue?
2. Should the window frames use the same product line?
3. Can Värvikoda deliver by 20 October if I confirm on Wednesday?

Thanks,
Anna
"""

TEAMS = """[Teams · Serna build chat · 2026-10-06 16:40]
Märt: Scaffolding is confirmed for 20 Oct. We still need the paint order placed by the 10th.
Evo: Waiting on Anna's spec, will chase.
"""

MEETING = """# Weekly sync — 2026-10-06

- Serna: client mentioned they now prefer **blue** for the exterior.
- Purchasing: new rule from finance — all orders above 2 000 EUR need two approvers.
"""


def action(title, state, *, description="", due=None, follow=None, owner="Evo", priority=None):
    return {
        "title": title,
        "description": description,
        "state": state,
        "owner": owner,
        "related_person": None,
        "related_project": None,
        "sources": [],
        "thread": None,
        "due_on": due,
        "follow_up_on": follow,
        "dependencies": [],
        "parent": None,
        "follows_up": None,
        "priority": priority,
    }


class Cli:
    def __init__(self, brn: Path, data: Path, vault: Path, scratch: Path):
        self.brn, self.data, self.vault, self.scratch = brn, data, vault, scratch

    def run(self, *args: str, vault: bool = True) -> dict:
        cmd = [str(self.brn), "--json", "--data-dir", str(self.data)]
        if vault:
            cmd += ["--vault", str(self.vault)]
        cmd += list(args)
        done = subprocess.run(cmd, capture_output=True, text=True)
        if done.returncode != 0:
            sys.exit(f"brn {' '.join(args)} failed:\n{done.stdout}\n{done.stderr}")
        return json.loads(done.stdout) if done.stdout.strip() else {}

    def file(self, name: str, payload) -> str:
        path = self.scratch / name
        if isinstance(payload, str):
            path.write_text(payload, encoding="utf-8")
        else:
            path.write_text(json.dumps(payload, ensure_ascii=False), encoding="utf-8")
        return str(path)

    def propose(self, name: str, draft: dict) -> dict:
        return self.run("proposals", "create", "--file", self.file(name, draft))

    def approve(self, proposal_id: str) -> dict:
        shown = self.run("proposals", "show", proposal_id)
        version = find_key(shown, "version")
        return self.run(
            "proposals", "approve", proposal_id,
            "--review-version", str(version), "--operation", str(uuid.uuid4()),
        )


def find_key(value, key):
    if isinstance(value, dict):
        if key in value and isinstance(value[key], int):
            return value[key]
        for child in value.values():
            found = find_key(child, key)
            if found is not None:
                return found
    elif isinstance(value, list):
        for child in value:
            found = find_key(child, key)
            if found is not None:
                return found
    return None


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument("--brn", type=Path, default=Path(__file__).resolve().parents[1] / "target/debug/brn")
    args = parser.parse_args()
    root = args.root
    if not root.is_absolute():
        sys.exit("--root must be absolute")
    if root.exists() and any(root.iterdir()):
        sys.exit(f"{root} is not empty; use a new disposable directory")
    data, vault, scratch = root / "data", root / "vault", root / "requests"
    for path in (data, vault, scratch):
        path.mkdir(parents=True)
    root = root.resolve()
    data, vault, scratch = data.resolve(), vault.resolve(), scratch.resolve()
    for rel, text in NOTES.items():
        target = vault / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")
    cli = Cli(args.brn, data, vault, scratch)
    cli.run("status")

    inbox = [
        ("Anna Tamm — Serna: three questions before the paint spec", "email", ANNA_EMAIL, "anna-serna-questions.eml.txt"),
        ("Teams · Serna build chat (6 Oct)", "teams", TEAMS, "serna-build-chat.txt"),
        ("Weekly sync — 2026-10-06", "markdown", MEETING, "weekly-sync-2026-10-06.md"),
    ]
    for title, kind, text, name in inbox:
        cli.run(
            "inbox", "add", "--id", str(uuid.uuid4()), "--title", title,
            "--file", cli.file(name, text), "--kind", kind, "--original-name", name,
        )

    approved_actions = [
        action("Reply to Anna's three Serna questions", "open",
               description="Anna needs colour, window-frame product line and delivery answers before finalising the spec.",
               due="2026-10-06", priority="high"),
        action("Get final paint specification from Anna", "waiting",
               description="Anna promised the final specification by 4 October.",
               follow="2026-10-07", owner="Anna Tamm"),
        action("Place Serna paint order with Värvikoda", "blocked",
               description="Blocked until the final specification arrives.", due="2026-10-10"),
        action("Confirm scaffolding date with Märt", "completed",
               description="Confirmed for 20 October."),
    ]
    for index, data_record in enumerate(approved_actions):
        state = data_record["state"]
        if state == "completed":
            data_record = dict(data_record, state="open")
        proposal = str(uuid.uuid4())
        action_id = str(uuid.uuid4())
        cli.propose(f"action-{index}.json", {
            "id": proposal, "group_id": None, "session_id": None,
            "title": f"Create action: {data_record['title']}",
            "changes": [], "sources": [],
            "action_changes": [{"kind": "create", "id": action_id, "data": data_record}],
        })
        cli.approve(proposal)
        if state == "completed":
            before = cli.run("actions", "show", action_id)["data"]
            cli.run("actions", "complete", "--file", cli.file("complete.json", {
                "operation_id": str(uuid.uuid4()), "before": before,
            }))

    # One approved knowledge note for Activity.
    note = str(uuid.uuid4())
    cli.propose("people-mart.json", {
        "id": note, "group_id": None, "session_id": None,
        "title": "Create person profile: Märt (builder)",
        "changes": [{"kind": "create", "path": "people/mart.md",
                     "text": "# Märt\n\n- **Role:** Builder, Serna renovation\n- Scaffolding lead.\n"}],
        "sources": [],
    })
    cli.approve(note)

    # Pending proposals awaiting review.
    group = str(uuid.uuid4())
    paint = str(uuid.uuid4())
    cli.propose("process-draft.json", {
        "id": paint, "group_id": None, "session_id": None,
        "title": "New process: How we buy paint",
        "changes": [{"kind": "create", "path": "processes/buying-paint.md", "text": PAINT_PROCESS}],
        "sources": [],
    })
    for index, (quote, note) in enumerate([
        ("team lead approves", "Check with finance: is 5 000 EUR still the limit?"),
        ("Finance places the order", "Märt usually places paint orders, not finance."),
    ]):
        start = PAINT_PROCESS.encode("utf-8").index(quote.encode("utf-8"))
        version = find_key(cli.run("proposals", "show", paint), "version")
        cli.run("proposals", "comment", "--file", cli.file(f"comment-{index}.json", {
            "expected": {"id": paint, "version": version},
            "comment": {"id": str(uuid.uuid4()), "text": note, "target": {
                "kind": "text",
                "anchor": {"change_index": 0, "start": start,
                           "end": start + len(quote.encode("utf-8")), "quote": quote},
            }},
        }))
    cli.propose("serna-followup-action.json", {
        "id": str(uuid.uuid4()), "group_id": group, "session_id": None,
        "title": "Create action: Ask Märt whether the 10 Oct order deadline can move",
        "changes": [], "sources": [],
        "action_changes": [{"kind": "create", "id": str(uuid.uuid4()), "data": action(
            "Ask Märt whether the 10 Oct order deadline can move", "open",
            description="From the Serna build chat on 6 October.", due="2026-10-08")}],
    })
    cli.propose("anna-profile.json", {
        "id": str(uuid.uuid4()), "group_id": group, "session_id": None,
        "title": "Create note: Serna window frames question",
        "changes": [{"kind": "create", "path": "projects/serna-open-questions.md",
                     "text": "# Serna open questions\n\n- Window frames: same product line as walls? (asked by Anna, 6 Oct)\n"}],
        "sources": [],
    })

    print(json.dumps({"data": str(data), "vault": str(vault), "root": str(root)}))


PAINT_PROCESS = """# How we buy paint

> Draft proposed by BRN. Sections marked *Proposed* are not established in the vault.

## 1. Specify

The project architect provides a written paint specification (product line,
colour code, finish, quantity). *Source: Serna supplier decision.*

## 2. Request

The requester writes a purchasing request with budget code
(*Source: Purchasing process*).

## 3. Approve

- Under 5 000 EUR: team lead approves (*Source: Purchasing process*).
- **Conflict:** the 2026-10-06 weekly sync says orders above 2 000 EUR need two
  approvers. Resolve before approving this process.

## 4. Order and receive *(Proposed)*

Finance places the order with the selected supplier and confirms the delivery
date with the site lead.
"""

if __name__ == "__main__":
    main()
