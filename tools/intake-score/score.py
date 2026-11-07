#!/usr/bin/env python3
"""Deterministic, provider-free scorer for the checked AI-Off import service."""
import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path
import platform
import socket
import statistics
import subprocess
import tempfile
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
KINDS = {k.lower(): k for k in ["Pdf", "Docx", "Pptx", "Xlsx", "Eml", "Html", "Text", "Markdown"]}
MODES = ["xberg", "xberg_brn"]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def dump(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n")


def canonical(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False)


def fidelity(expected, inventory):
    """Consume occurrences once. Text matches exact spans within visible blocks.

    Structure is typed and exact, including the whole ordered table matrix.
    Only manifest-named image fields are compared, never an inferred figure count.
    """
    available = list(inventory["elements"])
    blocks = list(inventory["blocks"])
    retained, missing = [], []
    for index, want in enumerate(expected):
        found = False
        if want["type"] == "text":
            for i, block in enumerate(blocks):
                if want["text"] in block:
                    blocks[i] = block.replace(want["text"], "", 1)
                    found = True
                    break
        else:
            for i, actual in enumerate(available):
                if all(actual.get(key) == value for key, value in want.items()):
                    available.pop(i)
                    found = True
                    break
        retained.append(found)
        if not found:
            missing.append({"index": index, **want})
    return {"score": sum(retained) / len(expected), "retained": retained,
            "missing": missing, "unexpected_structure": available}


def spread(samples):
    return {"median": statistics.median(samples), "min": min(samples), "max": max(samples),
            "spread": max(samples) - min(samples), "samples": samples}


def verify_frozen():
    frozen = json.loads((HERE / "frozen.json").read_text())
    for name, digest in frozen["files"].items():
        if sha(ROOT / name) != digest:
            raise ValueError("frozen scorer/fixture changed: " + name)
    manifests = json.loads((HERE / "manifests.json").read_text())["fixtures"]
    split = json.loads((HERE / "split.json").read_text())
    ids = [f["id"] for f in manifests]
    if len(ids) != len(set(ids)) or set(split["tuning"]) & set(split["held-out"]):
        raise ValueError("duplicate or overlapping fixture IDs")
    if sorted(ids) != sorted(split["tuning"] + split["held-out"]):
        raise ValueError("split does not partition manifests")
    for f in manifests:
        if f["id"] not in split[f["split"]] or sha(ROOT / f["path"]) != f["sha256"] or not f["expected"]:
            raise ValueError("fixture/split mismatch: " + f["id"])
    return manifests


class Probe:
    def __init__(self, binary):
        # The driver has no provider dependency. Helpers clear their environment,
        # install the existing OS network sandbox and have no URL-input route.
        self.process = subprocess.Popen([binary], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                        stderr=subprocess.PIPE, text=True, env={})

    def run(self, request):
        self.process.stdin.write(json.dumps(request) + "\n")
        self.process.stdin.flush()
        line = self.process.stdout.readline()
        if not line:
            raise RuntimeError("probe failed: " + self.process.stderr.read()[:2000])
        return json.loads(line)

    def close(self):
        self.process.stdin.close()
        if self.process.wait(timeout=30):
            raise RuntimeError("probe exited unsuccessfully")


def inventory(binary, result):
    return json.loads(subprocess.check_output([binary], input=json.dumps(result).encode(), env={}, timeout=30))


def network_witness(helper, scratch):
    forbidden = scratch / "forbidden.txt"
    forbidden.write_text("synthetic forbidden file")
    # A live loopback listener distinguishes sandbox denial from ECONNREFUSED.
    # The sandboxed connect must fail; no external network or provider is used.
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        listener.listen()
        out = subprocess.check_output([helper, "--self-test-restrictions", str(forbidden),
                                      str(listener.getsockname()[1])], env={}, timeout=30)
    value = json.loads(out)
    if value != {"file_denied": True, "network_denied": True}:
        raise ValueError("helper restriction witness failed")
    return value


def compare(base, final):
    """Report losses beyond the hand manifests as a separate conservative gate."""
    expected_losses = [i for i, (b, f) in enumerate(zip(base["retained"], final["retained"])) if b and not f]
    unexpected = []
    for key in ["elements", "blocks"]:
        old = Counter(map(canonical, base["inventory"][key]))
        new = Counter(map(canonical, final["inventory"][key]))
        unexpected += [{"kind": key, "element": json.loads(value), "count": count}
                       for value, count in (old - new).items()]
    return {"newly_lost_expected": expected_losses, "unexpected_losses": unexpected}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--arms", type=Path, required=True,
                        help='JSON mapping arm names to absolute probe, stock and brn executable paths')
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--split", choices=["tuning", "held-out"], required=True)
    parser.add_argument("--runs", type=int, default=5)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.runs < 5:
        parser.error("at least five runs required")
    fixtures = [f for f in verify_frozen() if f["split"] == args.split]
    arms = json.loads(args.arms.read_text())
    if not arms:
        parser.error("at least one arm required")
    for arm in arms.values():
        for path in arm.values():
            if not Path(path).is_absolute() or not Path(path).is_file():
                parser.error("absolute existing executable paths required")
    result = {"schema": 1, "split": args.split, "runs": args.runs,
              "frozen_sha256": sha(HERE / "frozen.json"),
              "machine": platform.platform(), "clock": "perf_counter wall seconds",
              "timing_policy": {
                  "cold": "new host process and fresh scratch per import; includes host startup/read/conversion; OS file caches NOT evicted",
                  "warm": "persistent preheated host process, fresh helper and scratch per import; read/conversion wall clock inside host",
                  "order": "fixture then temperature then repetition then helper mode then arm, ABAB for each mode; identical source bytes and mtimes",
                  "scope": "checked AI-Off conversion, not SQLite publication or desktop interaction"},
              "artifacts": {name: {key: {"path": path, "sha256": sha(path)} for key, path in arm.items()}
                            for name, arm in arms.items()},
              "inventory_sha256": sha(args.inventory), "restrictions": {}, "rows": [], "comparisons": []}
    with tempfile.TemporaryDirectory(prefix="brn-intake-score-", dir="/private/tmp" if platform.system() == "Darwin" else None) as temporary:
        scratch = Path(temporary).resolve()
        probes = {name: Probe(arm["probe"]) for name, arm in arms.items()}
        try:
            for name, arm in arms.items():
                result["restrictions"][name] = {key: network_witness(arm[key], scratch) for key in ["stock", "brn"]}
            for fixture in fixtures:
                rows = {}
                for name in arms:
                    for mode in MODES:
                        rows[name, mode] = {"arm": name, "mode": mode, "format": fixture["kind"],
                                            "fixture": fixture["id"], "cold": [], "warm": []}
                for temperature in ["cold", "warm"]:
                    # Unmeasured warmup, with the same file and arm, never on held-out
                    # before the final held-out command itself.
                    for repetition in range(-1 if temperature == "warm" else 0, args.runs):
                        for mode in MODES:
                            for name, arm in arms.items():
                                request = {"path": str((ROOT / fixture["path"]).resolve()),
                                           "kind": KINDS[fixture["kind"]], "backend": mode,
                                           "stock": arm["stock"], "brn": arm["brn"], "scratch": str(scratch / "scratch")}
                                start = time.perf_counter()
                                if temperature == "cold":
                                    output = json.loads(subprocess.check_output([arm["probe"]], input=(json.dumps(request)+"\n").encode(), env={}, timeout=45))
                                    seconds = time.perf_counter() - start
                                else:
                                    output = probes[name].run(request)
                                    seconds = output["seconds"]
                                row = rows[name, mode]
                                if repetition < 0:
                                    continue
                                row[temperature].append(seconds)
                                payload = {key: value for key, value in output.items() if key != "seconds"}
                                if "payload" in row and row["payload"] != payload:
                                    raise ValueError("nondeterministic conversion: " + fixture["id"])
                                row["payload"] = payload
                for row in rows.values():
                    output = row.pop("payload")
                    inv = inventory(str(args.inventory), output["result"]) if "result" in output else {"elements": [], "blocks": []}
                    row.update(fidelity(fixture["expected"], inv))
                    row["inventory"] = inv
                    row["gaps"] = output.get("result", {}).get("coverage", {}).get("gaps", [])
                    row["error"] = output.get("error")
                    for temp in ["cold", "warm"]:
                        row[temp] = spread(row[temp])
                    result["rows"].append(row)
                if len(arms) == 2:
                    base_name, final_name = arms
                    for mode in MODES:
                        base, final = rows[base_name, mode], rows[final_name, mode]
                        comp = {"fixture": fixture["id"], "mode": mode, **compare(base, final)}
                        comp["speed"] = {temp: {"paired_median_gain": statistics.median([b-f for b, f in zip(base[temp]["samples"], final[temp]["samples"])]),
                                                 "required_gain": max(base[temp]["spread"], final[temp]["spread"])} for temp in ["cold", "warm"]}
                        result["comparisons"].append(comp)
            # All existing hostile DOCX inputs must refuse under every arm/mode.
            result["hostile"] = []
            for path in sorted((ROOT / "crates/brn-intake/tests/fixtures").glob("hostile-*.docx")):
                for mode in MODES:
                    for name, arm in arms.items():
                        output = probes[name].run({"path": str(path), "kind": "Docx", "backend": mode,
                                                   "stock": arm["stock"], "brn": arm["brn"], "scratch": str(scratch / "scratch")})
                        result["hostile"].append({"fixture": path.name, "arm": name, "mode": mode, "refused": "error" in output, "error": output.get("error")})
        finally:
            for probe in probes.values():
                probe.close()
    groups = defaultdict(list)
    for row in result["rows"]:
        for fmt in [row["format"], "all"]:
            groups[row["arm"], row["mode"], fmt].append(row)
    result["summary"] = [{"arm": arm, "mode": mode, "format": fmt,
                          "fidelity": statistics.mean(r["score"] for r in rows),
                          "cold_seconds": statistics.median(r["cold"]["median"] for r in rows),
                          "warm_seconds": statistics.median(r["warm"]["median"] for r in rows)}
                         for (arm, mode, fmt), rows in groups.items()]
    result["guardrails"] = {"hostile_refuse": all(r["refused"] for r in result["hostile"]),
                            "network_denied": True, "encrypted_pdf": "deferred, unmeasured, no support/refusal claim",
                            "no_new_expected_loss": not any(r["newly_lost_expected"] for r in result["comparisons"]),
                            "no_unexpected_loss": not any(r["unexpected_losses"] for r in result["comparisons"]),
                            "tests_and_app_size": "separate required receipts"}
    dump(args.output, result)
    print(json.dumps({"output": str(args.output), "summary": [s for s in result["summary"] if s["format"] == "all"], "guardrails": result["guardrails"]}, indent=2))
    if not all(result["guardrails"][k] for k in ["hostile_refuse", "no_new_expected_loss", "no_unexpected_loss"]):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
