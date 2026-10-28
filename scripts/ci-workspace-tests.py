#!/usr/bin/env python3
"""Run the unchanged offline suite; annotate bounded Rust test identities only."""
import re
import subprocess
import sys

COMMAND = ["cargo", "test", "--workspace", "--no-fail-fast", "--locked", "--offline"]
FAILED_TEST = re.compile(r"^test ([A-Za-z0-9_:]{1,240}) \.\.\. FAILED$")
ANSI = re.compile(r"\x1b\[[0-9;]*m")
MAX_NAMES = 50


def run(command=COMMAND, output=None):
    output = output or sys.stdout
    names = set()
    with subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT) as child:
        for raw in child.stdout:
            # Stream the normal test output, without saving a second raw artifact.
            line = raw.decode("utf-8", errors="replace")
            output.write(line)
            output.flush()
            match = FAILED_TEST.fullmatch(ANSI.sub("", line.rstrip("\r\n")))
            if match and len(names) < MAX_NAMES:
                names.add(match.group(1))
        status = child.wait()
    if status:
        for name in sorted(names):
            output.write(f"::error title=Workspace test failed::{name}\n")
        if not names:
            output.write("::error title=Workspace test failed::No Rust test identity captured; inspect build or process failure.\n")
    # Shell convention for signal termination; never turn failure into success.
    return status if status >= 0 else 128 - status


if __name__ == "__main__":
    sys.exit(run())
