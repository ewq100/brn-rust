#!/usr/bin/env python3
"""Reject live retired readers/builds; historical evidence and eval support are data."""
from pathlib import Path
import re
import sys
import tomllib

LEGACY_PATHS = (
    'crates/brn-store', 'crates/brn-workflow', 'crates/brn/src/cli',
    'crates/brn-intake/src/bin/brn-intake-helper.rs',
    'crates/brn-intake/src/helper.rs', 'crates/brn-intake/src/helper',
    'crates/brn-intake/src/prepared.rs', 'crates/brn-intake/src/prepared',
    'crates/brn-threads-intake/src/prepared.rs',
    'crates/brn-threads-intake/src/pdf_tools.rs',
    'crates/brn-threads-intake/src/docx_markdown.rs',
    'crates/brn-ai/src/conversion_tests.rs', 'crates/brn-ai/examples/qualify_conversion.rs',
    'crates/brn-threads-intake/tests/qualification.rs',
    *(f'crates/brn-intake/tests/{name}.rs' for name in
      ('maintained', 'pptx', 'docx_inventory', 'office_differential', 'restricted_helper', 'protocol')),
)
HELPER_DEPS = {'betteroffice-docx-edit', 'betteroffice-docx-parse', 'betteroffice-pptx-parse',
               'betteroffice-opc', 'mail-parser', 'zip', 'image', 'libc', 'html5ever',
               'markup5ever_rcdom', 'calamine', 'quick-xml', 'xberg', 'tl', 'schemars',
               'unicode-normalization'}


def workflow_failures(name, text):
    failures = []
    # Jobs own their fetch prerequisites; another job's warm cache is no proof.
    for job in re.split(r'^  [\w-]+:\s*$', text, flags=re.M):
        stock = job.find('cargo fetch --locked --manifest-path tools/xberg-intake-helper/Cargo.toml')
        discovery = job.find('python3 -m unittest discover -s scripts/tests')
        if discovery >= 0 and (stock < 0 or stock > discovery):
            failures.append(f'{name}: stock lock must be fetched before generator-test discovery')
        for invocation in re.finditer(r'^\s*python3 scripts/prepare-xberg-adapted.py([^\n]*)', job, re.M):
            if not re.search(r'--archive\s+\S+', invocation.group(1)):
                failures.append(f'{name}: generator requires explicit --archive')
            if stock < 0 or stock > invocation.start():
                failures.append(f'{name}: stock lock fetch must precede generation')
            adapted = job.find('cargo fetch --locked --manifest-path target/xberg-adapted/workspace/Cargo.toml')
            if adapted < invocation.end():
                failures.append(f'{name}: adapted fetch must follow generation')
    return failures


def check(root):
    failures = [f'Retired path remains: {name}' for name in LEGACY_PATHS if (root / name).exists()]
    for path in (root / 'crates').rglob('*.rs'):
        if 'tests' in path.parts or 'tests' in path.name:
            continue
        text = re.split(r'#\[cfg\(test\)\]\nmod \w+\s*\{', path.read_text(), maxsplit=1)[0]
        for token in ('convert_docx_source', 'convert_docx_original', 'roxmltree::',
                      'schemas.openxmlformats.org/wordprocessingml',
                      'schemas.openxmlformats.org/drawingml', 'HelperRequest',
                      'PreparedDocument', 'PdfTools', 'SourceInventory', 'run_renderer'):
            if token in text:
                failures.append(f'{path.relative_to(root)}: retired format mechanics {token}')
        if path == root / 'crates/brn-ai/src/conversion.rs' and re.search(r'pub (?:async )?fn (?:convert|plan_for_prepared|candidate_for_prepared)\b|pub struct ConversionJob\b', text):
            failures.append('Legacy AI conversion entry remains')
        if path == root / 'crates/brn-threads-intake/src/lib.rs' and re.search(r'pub fn (?:convert|prepare|finish_candidate)\b', text):
            failures.append('Legacy intake entry remains')
        if path == root / 'crates/brn-intake/src/lib.rs' and re.search(r'pub mod (?:prepared|helper)\b|pub struct (?:Extraction|SourceNode|ImageOccurrence|IntakeUsage)\b', text):
            failures.append('Legacy helper protocol remains')
    for name, banned in [('brn-threads-core', {'zip', 'flate2', 'quick-xml', 'roxmltree', 'png', 'crc32fast'}),
                         ('brn-intake', HELPER_DEPS), ('brn-threads-intake', {'png', 'quick-xml'})]:
        path = root / f'crates/{name}/Cargo.toml'
        if not path.exists():
            continue
        manifest = tomllib.loads(path.read_text())
        for section in ('dependencies', 'dev-dependencies', 'build-dependencies'):
            for dep in banned.intersection(manifest.get(section, {})):
                failures.append(f'{name}: retired dependency {dep}')
        if name == 'brn-intake':
            for feature in ('helper', 'office-differential'):
                if feature in manifest.get('features', {}):
                    failures.append(f'brn-intake: retired feature {feature}')
            if any(b.get('name') == 'brn-intake-helper' for b in manifest.get('bin', [])):
                failures.append('Legacy helper binary registration remains')
    lock = root / 'Cargo.lock'
    if lock.exists():
        for package in tomllib.loads(lock.read_text()).get('package', []):
            if package['name'].startswith('betteroffice-') or package['name'] == 'xberg' or (package['name'], package['version']) == ('mail-parser', '0.11.8'):
                failures.append(f'Root legacy-only lock identity remains: {package["name"]} {package["version"]}')
    for folder, pattern in [('scripts', '*.sh'), ('.github', '*.yml')]:
        for path in (root / folder).rglob(pattern):
            text = path.read_text()
            if re.search(r'cargo[^\n]*--bin\s+brn-intake-helper\b|brew install poppler|BRN_PDF(?:TOTEXT|IMAGES|TOPPM)=', text):
                failures.append(f'{path.relative_to(root)}: retired reader build/setup')
            if path.parent.name == 'workflows':
                failures.extend(workflow_failures(str(path.relative_to(root)), text))
    return failures


if __name__ == '__main__':
    failures = check(Path(__file__).resolve().parent.parent)
    if failures:
        print('\n'.join(failures), file=sys.stderr)
        sys.exit(1)
    print('Current Xberg intake / legacy reader retirement check passed')
