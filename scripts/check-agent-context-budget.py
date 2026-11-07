#!/usr/bin/env python3
"""Keep always-loaded agent context from growing unnoticed.

Measures bytes of AGENTS.md and the shipped agent guidance in agent/.
Default: fail unless scripts/agent-context-budget.json equals the measurement
(run --write to update it). --compare BASE: fail when any value grew past BASE,
unless ALLOW_AGENT_CONTEXT_INCREASE=true.
"""
import argparse
import json
import os
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parent.parent
BUDGET = ROOT / 'scripts/agent-context-budget.json'


def measure(root=ROOT):
    files = [root / 'AGENTS.md', *sorted((root / 'agent').rglob('*.md'))]
    return {path.relative_to(root).as_posix(): path.stat().st_size for path in files if path.is_file()}


def grown(current, base):
    return {key: (base.get(key, 0), value) for key, value in current.items() if value > base.get(key, 0)}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument('--write', action='store_true')
    parser.add_argument('--compare', type=Path, metavar='BASE_JSON')
    opts = parser.parse_args(argv)
    measured = measure()
    if opts.write:
        BUDGET.write_text(json.dumps(measured, indent=2) + '\n')
        return 0
    if opts.compare:
        growth = grown(measured, json.loads(opts.compare.read_text()))
        for key, (before, after) in growth.items():
            print(f'{key}: {before} -> {after} bytes (+{after - before})')
        if growth and os.environ.get('ALLOW_AGENT_CONTEXT_INCREASE') != 'true':
            print('Agent context grew. Trim it, or add the agent-context-increase label to accept the growth.')
            return 1
        print('Agent context is within budget.')
        return 0
    if json.loads(BUDGET.read_text()) != measured:
        print('scripts/agent-context-budget.json is stale; run python3 scripts/check-agent-context-budget.py --write')
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
