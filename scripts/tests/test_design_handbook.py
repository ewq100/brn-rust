"""Design handbook tooling: generated token files stay in sync with tokens.rs."""
import importlib.util
from pathlib import Path
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]


def module(name):
    spec = importlib.util.spec_from_file_location(name.replace('-', '_'), SCRIPTS / f'{name}.py')
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


handbook = module('design-handbook')


class DesignHandbookTests(unittest.TestCase):
    def test_generated_token_files_match_tokens_rs(self):
        self.assertEqual(handbook.check_tokens(), 0)

    def test_parse_reads_every_palette_role_in_both_schemes(self):
        tokens = handbook.parse(handbook.TOKENS_RS.read_text(encoding='utf-8'))
        self.assertEqual(set(tokens['order']), set(handbook.ROLES))
        for scheme in ('dark', 'light'):
            self.assertEqual(list(tokens['palettes'][scheme]), tokens['order'])
        self.assertEqual(tokens['fonts']['MONO_FONT'], 'Menlo')

    def test_text_tokens_meet_contrast_on_paper(self):
        tokens = handbook.parse(handbook.TOKENS_RS.read_text(encoding='utf-8'))
        for scheme in ('dark', 'light'):
            palette = tokens['palettes'][scheme]
            for key in ('text', 'muted', 'cyan', 'amber', 'purple', 'green', 'red'):
                with self.subTest(scheme=scheme, key=key):
                    self.assertGreaterEqual(handbook.contrast(palette[key], palette['paper']), 4.5)

    def test_parse_refuses_missing_palette(self):
        with self.assertRaises(SystemExit):
            handbook.parse('pub struct Palette { pub text: u32, }')


if __name__ == '__main__':
    unittest.main()
