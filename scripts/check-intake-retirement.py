#!/usr/bin/env python3
"""Check Threads ships maintained converters and no old persistence engine."""
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
manifest = tomllib.loads((root / "crates/brn-threads-core/Cargo.toml").read_text())
for dependency in ("zip", "flate2", "quick-xml", "roxmltree", "png", "crc32fast"):
    if dependency in manifest.get("dependencies", {}):
        failures.append(f"Core still directly owns format dependency {dependency}")
for path in (root / "crates/brn-store", root / "crates/brn-workflow", root / "crates/brn/src/cli"):
    if path.exists():
        failures.append(f"Retired engine path remains: {path.relative_to(root)}")
if failures:
    print("\n".join(failures), file=sys.stderr)
    sys.exit(1)
print("Maintained intake / Threads engine retirement check passed")
