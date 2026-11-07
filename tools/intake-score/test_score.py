"""Negative controls for the scorer; no provider, network or converter calls."""
import importlib.util
import json
from pathlib import Path
import subprocess
import unittest

SPEC = importlib.util.spec_from_file_location("score", Path(__file__).with_name("score.py"))
score = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(score)


class ScoreTests(unittest.TestCase):
    def test_structure_cannot_be_replaced_by_text_or_reused(self):
        expected = [{"type": "heading", "level": 2, "text": "Warning"},
                    {"type": "list_item", "text": "Check"},
                    {"type": "list_item", "text": "Check"}]
        inv = {"elements": [{"type": "heading", "level": 1, "text": "Warning"},
                            {"type": "list_item", "text": "Check"}],
               "blocks": ["Warning", "Check"]}
        self.assertEqual(score.fidelity(expected, inv)["retained"], [False, True, False])

    def test_table_counts_and_cell_order_are_exact(self):
        table = {"type": "table", "rows": 2, "columns": 2, "cells": [["A", "B"], ["1", "2"]]}
        for changed in [{**table, "rows": 3}, {**table, "cells": [["B", "A"], ["1", "2"]]}]:
            self.assertEqual(score.fidelity([table], {"elements": [changed], "blocks": []})["score"], 0)

    def test_figures_require_exact_identity_and_occurrence(self):
        image = {"type": "image", "sha256": [1]*32, "width": 2, "height": 3}
        inv = {"elements": [image], "blocks": []}
        self.assertEqual(score.fidelity([image, image], inv)["retained"], [True, False])
        wrong = {**image, "sha256": [2]*32}
        self.assertEqual(score.fidelity([image], {"elements": [wrong], "blocks": []})["score"], 0)

    def test_text_occurrences_are_consumed_without_case_or_space_folding(self):
        want = {"type": "text", "text": "Gate  A"}
        inv = {"elements": [], "blocks": ["Gate  A and gate A"]}
        self.assertEqual(score.fidelity([want, want], inv)["retained"], [True, False])

    def test_unmanifested_losses_are_reported(self):
        base = {"retained": [True], "inventory": {"elements": [], "blocks": ["Keep", "Other"]}}
        final = {"retained": [True], "inventory": {"elements": [], "blocks": ["Keep"]}}
        self.assertEqual(score.compare(base, final)["unexpected_losses"],
                         [{"kind": "blocks", "element": "Other", "count": 1}])


if __name__ == "__main__":
    unittest.main()
