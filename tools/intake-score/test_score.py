"""Negative controls for the scorer; no provider, network or converter calls."""
import importlib.util
import json
from pathlib import Path
import subprocess
import os
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
        self.assertIn({"kind": "blocks", "element": "Other", "count": 1}, score.compare(base, final)["unexpected_losses"])

    def test_order_and_false_structure_are_rejected(self):
        expected = [{"type": "list_item", "text": "Check"}, {"type": "list_item", "text": "Release"}]
        inv = {"elements": [{**expected[1], "position": 0}, {**expected[0], "position": 20}], "blocks": []}
        self.assertEqual(score.fidelity(expected, inv, [[0, 1]])["score"], .5)
        base = {"retained": [True], "inventory": {"elements": expected, "blocks": []}, "unexpected_structure": []}
        final = {**base, "inventory": {"elements": expected[::-1], "blocks": []},
                 "unexpected_structure": [{"type": "heading", "level": 1, "text": "Fabricated"}]}
        comparison = score.compare(base, final)
        self.assertTrue(comparison["unexpected_losses"])
        self.assertTrue(comparison["unjustified_additions"])

    def test_text_removal_cannot_create_another_match(self):
        expected = [{"type": "text", "text": "REMOVED"}, {"type": "text", "text": "AB"}]
        self.assertEqual(score.fidelity(expected, {"elements": [], "blocks": ["AREMOVEDB"]})["retained"], [True, False])

    def test_nested_table_is_one_tree(self):
        html = '<table><tr><td>Stage</td><td>Detail</td></tr><tr><td>Prepare</td><td>Nested schedule<table><tr><td>Inner gate</td><td>Owner</td></tr><tr><td>Inspect</td><td>Iris</td></tr></table>After nested table.</td></tr></table>'
        parser = score.Tables(); parser.feed(html)
        self.assertEqual(len(parser.tables), 1)
        detail = parser.tables[0]["cells"][1][1]
        self.assertEqual(detail["before"], "Nested schedule")
        self.assertEqual(detail["table"]["cells"], [["Inner gate", "Owner"], ["Inspect", "Iris"]])
        self.assertEqual(detail["after"], "After nested table.")

    def test_real_ast_distinguishes_source_semantics(self):
        binary = os.environ["BRN_SCORE_INVENTORY"]
        def inv(markdown):
            return score.inventory(binary, {"markdown": markdown, "assets": []})
        for before, after in [
            ('1. Check\n2. Release\n', '- Check\n- Release\n'),
            ('3. Check\n4. Release\n', '1. Check\n2. Release\n'),
            ('- Check\n  - Inspect\n', '- Check\n- Inspect\n'),
            ('[station](https://example.test/a)', '[station](https://example.test/b)'),
            ('> Warning\n', 'Warning\n'),
            ('## Heading\n', '```text\n## Heading\n```\n')]:
            old, new = inv(before), inv(after)
            self.assertNotEqual(old, new, (before, after))
            comparison = score.compare({"retained": [], "inventory": old}, {"retained": [], "inventory": new})
            self.assertTrue(comparison["unexpected_losses"])


if __name__ == "__main__":
    unittest.main()
