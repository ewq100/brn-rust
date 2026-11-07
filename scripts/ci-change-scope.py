#!/usr/bin/env python3
"""Decide whether a PR's changed paths (one per stdin line) need the Rust CI lanes."""
import sys

# Rust builds embed or read Markdown under these paths, so they are never prose-only.
CODE_PREFIXES = ('crates/', 'agent/', 'tools/')
# GitHub's PR files API stops listing at 3,000 files; treat a full page set as incomplete.
API_FILE_LIMIT = 3000


def rust_needed(paths):
    if not paths or len(paths) >= API_FILE_LIMIT:
        return True
    return any(not path.endswith('.md') or path.startswith(CODE_PREFIXES) or '/fixtures/' in path
               for path in paths)


def main():
    paths = [line.strip() for line in sys.stdin if line.strip()]
    print(f"rust={'true' if rust_needed(paths) else 'false'}")
    return 0


if __name__ == '__main__':
    sys.exit(main())
