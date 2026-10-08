#!/usr/bin/env python3
"""Check P2 ships maintained adapters and historical record readers, not Word interpreters."""
from pathlib import Path
import re
import sys
import tomllib

root = Path(__file__).resolve().parent.parent
failures = []
legacy = root / "crates/brn-store/src/work/inbox_source"
for name in ("docx.rs", "docx"):
    if (legacy / name).exists():
        failures.append(f"retired production interpreter remains: {legacy / name}")
for path in (root / "crates").rglob("*.rs"):
    if "tests" in path.parts or "tests" in path.name:
        continue
    text = path.read_text()
    text = re.split(r"#\[cfg\(test\)\]\nmod \w+\s*\{", text, maxsplit=1)[0]
    for token in ("convert_docx_source", "convert_docx_original", "roxmltree::", "schemas.openxmlformats.org/wordprocessingml", "schemas.openxmlformats.org/drawingml"):
        if token in text:
            failures.append(f"{path.relative_to(root)}: retired format mechanics {token}")
manifest = tomllib.loads((root / "crates/brn-store/Cargo.toml").read_text())
for dependency in ("zip", "flate2", "quick-xml", "roxmltree", "png", "crc32fast"):
    if dependency in manifest.get("dependencies", {}):
        failures.append(f"Store still directly owns format dependency {dependency}")
if failures:
    print("\n".join(failures), file=sys.stderr)
    sys.exit(1)
print("Maintained intake / historical-reader retirement check passed")
