#!/usr/bin/env python3
"""Check shipped code, not historical documents, trials or rejection tests."""
from pathlib import Path
import re
import sys

root = Path(__file__).resolve().parent.parent
pattern = re.compile(r"brn[-_]provider|codex_home|--codex")
paths = [root / "Cargo.toml"]
paths += list((root / "crates").rglob("*.toml"))
paths += [p for p in (root / "crates").rglob("*.rs") if "tests" not in p.parts]
# Trial verification is standalone historical evidence, not a product launcher.
paths += [p for p in (root / "scripts").glob("*.sh")
          if p.name != "verify-trial.sh"]
paths += [p for p in (root / "scripts").glob("*.py")
          if p.name != Path(__file__).name]
failures = []
for path in sorted(paths):
    text = path.read_text()
    if path.suffix == ".rs":
        # Trailing private test modules may explicitly reject retired flags.
        text = re.split(r"#\[cfg\(test\)\]\nmod \w+\s*\{", text, maxsplit=1)[0]
    for number, line in enumerate(text.splitlines(), 1):
        if pattern.search(line):
            failures.append(f"{path.relative_to(root)}:{number}:{line}")
if failures:
    print("\n".join(failures))
    print("Production App Server reference remains", file=sys.stderr)
    sys.exit(1)
print("Production provider retirement check passed")
