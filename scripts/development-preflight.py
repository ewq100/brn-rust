#!/usr/bin/env python3
"""Read-only checkout/tool/router inventory; never installs or changes settings."""
import argparse
import json
import os
import re
import sys
from pathlib import Path
import shutil
import subprocess
from verification_evidence import ROOT, identity


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--native', action='store_true', help='also require protoc; on macOS, report Command Line Tools')
    parser.add_argument('--router-skill', type=Path, help='override the optional local developing-product-feature skill path')
    opts = parser.parse_args()
    tools = ['git', 'bash', 'python3', 'cargo', 'rustup'] + (['protoc'] if opts.native else [])
    if opts.native and sys.platform == 'darwin':
        tools.append('xcrun')
    found = {name: shutil.which(name) for name in tools}
    skill_root = Path(os.environ.get('CODEX_HOME', str(Path.home() / '.codex'))) / 'skills'
    router = opts.router_skill or skill_root / 'developing-product-feature/SKILL.md'
    report = {'identity': identity(), 'tools': found,
              'workflow': str(ROOT / 'docs/development/workflow.md'),
              'router_skill': {'path': str(router), 'available': router.is_file(), 'required': False},
              'pr_skill': {'path': str(skill_root / 'pr/SKILL.md'),
                           'available': (skill_root / 'pr/SKILL.md').is_file(),
                           'required': False, 'scope': 'explicit owner request when writing PRs'},
              'toolchain_pin': (ROOT / 'rust-toolchain.toml').read_text().strip(), 'pinned_toolchain_installed': False}
    pin = re.search(r'channel\s*=\s*"([^"]+)"', report['toolchain_pin'])
    if found['rustup']:
        # Listing installed toolchains does not install the repository's pinned version.
        result = subprocess.run(['rustup', 'toolchain', 'list'], text=True, capture_output=True)
        report['installed_toolchains'] = result.stdout.strip().splitlines()
        report['toolchain_inventory_exit'] = result.returncode
        report['pinned_toolchain_installed'] = result.returncode == 0 and any(
            pin and item.startswith(pin[1] + '-') for item in report['installed_toolchains'])
    if opts.native and shutil.which('xcrun'):
        result = subprocess.run(['xcrun', '--find', 'clang'], text=True, capture_output=True)
        report['macos_clang'] = result.stdout.strip() if result.returncode == 0 else None
    print(json.dumps(report, indent=2))
    native_ready = not (opts.native and sys.platform == 'darwin') or bool(report.get('macos_clang'))
    return 0 if all(found.values()) and report['pinned_toolchain_installed'] and native_ready else 1


if __name__ == '__main__':
    raise SystemExit(main())
