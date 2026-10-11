#!/usr/bin/env python3
"""Publish the approved current-main preview using an Actions-scoped token only."""
import argparse
import base64
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
from urllib.error import HTTPError
from urllib.parse import quote
from urllib.request import Request, urlopen

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("packaging", ROOT / "scripts/package-macos-preview.py")
packaging = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packaging)
REQUIRED = (
    "Core and CLI (ubuntu-24.04)",
    "Core and CLI (macos-15)",
    "Native UI build and state (macos-15)",
    "Native retrieval and combined build (macos-15)",
)


def validate_ci(run, jobs, current_main, repository):
    sha = run.get("head_sha", "")
    if (not re.fullmatch(r"[0-9a-f]{40}", sha) or sha != current_main
            or run.get("head_repository", {}).get("full_name") != repository
            or run.get("event") != "push" or run.get("head_branch") != "main"
            or run.get("path") != ".github/workflows/ci.yml"
            or run.get("name") != "BRN CI" or run.get("status") != "completed"):
        raise ValueError("Release requires completed same-repository main-push CI on current main")
    for name in REQUIRED:
        matches = [job for job in jobs if job.get("name") == name]
        if (len(matches) != 1 or matches[0].get("status") != "completed"
                or matches[0].get("conclusion") != "success"
                or matches[0].get("head_sha") != sha):
            raise ValueError(f"Required current-attempt job has not passed: {name}")
    return sha


def validate_event(event, run):
    trigger = event.get("workflow_run", {})
    if (event.get("action") != "completed" or trigger.get("id") != run.get("id")
            or trigger.get("head_sha") != run.get("head_sha")
            or trigger.get("run_attempt") != run.get("run_attempt")):
        raise ValueError("Trigger is stale or does not identify the current CI attempt")


def validate_changed_manifest(sha):
    changed = subprocess.run(
        ["git", "diff", "--name-only", f"{sha}^1", sha, "--", "release/preview.json"],
        cwd=ROOT, check=True, capture_output=True, text=True).stdout.splitlines()
    if "release/preview.json" not in changed:
        raise ValueError("This commit did not explicitly approve a preview manifest change")


def gate_output(path, eligible, sha):
    if path:
        with path.open("a") as output:
            output.write(f"eligible={'true' if eligible else 'false'}\nsource_sha={sha}\n")


def validate_delivery(directory, approved, sha):
    directory = Path(directory)
    if (directory / ".incomplete").exists():
        raise ValueError("Distribution publication was interrupted")
    name = f"BRN-Threads-{approved['version']}-macos-arm64"
    receipt = directory / f"{name}.manifest.json"
    delivered = json.loads(receipt.read_text())
    if (any(delivered.get(key) != value for key, value in approved.items())
            or delivered.get("source_sha") != sha
            or delivered.get("signing") != "ad-hoc; not notarized"):
        raise ValueError("Delivery identity differs from the approved preview and source")
    archive = directory / f"{name}.zip"
    if delivered.get("archive") != {"name": archive.name, "sha256": packaging.digest(archive),
                                   "bytes": archive.stat().st_size}:
        raise ValueError("Archive identity or checksum differs from its manifest")
    expected = "".join(f"{packaging.digest(path)}  {path.name}\n" for path in (archive, receipt))
    sums = directory / "SHA256SUMS"
    if sums.read_text() != expected:
        raise ValueError("Distribution checksum receipt differs")
    return [archive, receipt, sums]


class Github:
    def __init__(self, repository):
        if os.environ.get("GITHUB_ACTIONS") != "true" or not os.environ.get("GITHUB_TOKEN"):
            raise ValueError("Release API requires the workflow's ephemeral GITHUB_TOKEN")
        if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository):
            raise ValueError("Invalid repository")
        self.repository = repository
        self.prefix = f"https://api.github.com/repos/{repository}"
        self.token = os.environ["GITHUB_TOKEN"]

    def request(self, method, path, payload=None, binary=False, missing_ok=False):
        url = path if path.startswith("https://uploads.github.com/") else self.prefix + path
        headers = {"Authorization": f"Bearer {self.token}", "Accept": "application/vnd.github+json",
                   "X-GitHub-Api-Version": "2022-11-28"}
        body = None
        if payload is not None:
            body = payload if binary else json.dumps(payload).encode()
            headers["Content-Type"] = "application/octet-stream" if binary else "application/json"
        try:
            with urlopen(Request(url, data=body, headers=headers, method=method), timeout=120) as response:
                content = response.read()
                return json.loads(content) if content else None
        except HTTPError as error:
            if missing_ok and error.code == 404:
                return None
            raise ValueError(f"GitHub {method} failed with HTTP {error.code}") from None

    def ci(self, event):
        run = self.request("GET", f"/actions/runs/{int(event['workflow_run']['id'])}")
        validate_event(event, run)
        jobs = []
        page = 1
        while True:
            result = self.request("GET", f"/actions/runs/{run['id']}/attempts/{run['run_attempt']}/jobs?per_page=100&page={page}")
            jobs.extend(result["jobs"])
            if len(result["jobs"]) < 100:
                break
            page += 1
        main = self.request("GET", "/git/ref/heads/main")["object"]["sha"]
        return validate_ci(run, jobs, main, self.repository)

    def approved_at(self, sha, approved):
        current = self.request("GET", "/git/ref/heads/main")["object"]["sha"]
        if current != sha:
            raise ValueError("Main advanced; preview remains unpublished")
        file = self.request("GET", f"/contents/release/preview.json?ref={sha}")
        content = json.loads(base64.b64decode(file["content"]).decode())
        if content != approved:
            raise ValueError("Current-main approved manifest identity changed")

    def existing(self, version, sha):
        tag = self.request("GET", f"/git/ref/tags/{quote(version, safe='')}", missing_ok=True)
        if tag and (tag["object"].get("type") != "commit" or tag["object"].get("sha") != sha):
            raise ValueError("Version tag already identifies a different source; never retarget")
        # The by-tag endpoint exposes published releases. Authenticated listings
        # also expose drafts, including drafts that have not created a Git tag.
        matches = []
        page = 1
        while True:
            releases = self.request("GET", f"/releases?per_page=100&page={page}")
            matches.extend(item for item in releases if item.get("tag_name") == version)
            if len(releases) < 100:
                break
            page += 1
        if len(matches) > 1:
            raise ValueError("Multiple releases identify this version; refusing ambiguous ownership")
        release = matches[0] if matches else None
        if release and (release.get("target_commitish") != sha or not release.get("prerelease")):
            raise ValueError("Version release already identifies a different preview")
        if tag and not release:
            raise ValueError("Version tag exists without an owned preview release")
        return release


def publish(api, event, approved, sha, directory):
    assets = validate_delivery(directory, approved, sha)
    api.approved_at(sha, approved)
    if api.ci(event) != sha:
        raise ValueError("CI source identity changed")
    release = api.existing(approved["version"], sha)
    if release and not release["draft"]:
        raise ValueError("Published preview is immutable; refusing to overwrite")
    if release is None:
        release = api.request("POST", "/releases", {
            "tag_name": approved["version"], "target_commitish": sha,
            "name": f"BRN Threads {approved['version']} — macOS Apple Silicon preview",
            "body": (ROOT / approved["notes_file"]).read_text() + f"\nExact source: `{sha}`.\n",
            "draft": True, "prerelease": True, "make_latest": "false"})
    existing = {asset["name"]: asset for asset in api.request("GET", f"/releases/{release['id']}/assets?per_page=100")}
    for path in assets:
        expected_digest = f"sha256:{packaging.digest(path)}"
        if path.name in existing:
            # Resume only exact existing uploads; never delete or replace any asset.
            uploaded = existing[path.name]
        else:
            url = release["upload_url"].split("{", 1)[0] + "?name=" + quote(path.name)
            uploaded = api.request("POST", url, path.read_bytes(), binary=True)
        if uploaded.get("digest") != expected_digest or uploaded.get("size") != path.stat().st_size:
            raise ValueError(f"Uploaded asset checksum/size differs: {path.name}; draft remains unpublished")
    # Recheck after all uploads. A stale draft must never become a published preview.
    if api.ci(event) != sha:
        raise ValueError("CI source changed after upload; draft remains unpublished")
    api.approved_at(sha, approved)
    refreshed = api.existing(approved["version"], sha)
    if (refreshed is None or refreshed.get("id") != release["id"]
            or refreshed.get("draft") is not True
            or refreshed.get("tag_name") != approved["version"]
            or refreshed.get("target_commitish") != sha
            or refreshed.get("prerelease") is not True):
        raise ValueError("Owned preview draft changed after upload; refusing publication")
    result = api.request("PATCH", f"/releases/{release['id']}", {"draft": False, "prerelease": True, "make_latest": "false"})
    if result.get("draft") or not result.get("prerelease"):
        raise ValueError("GitHub did not confirm prerelease publication")
    return result["html_url"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["gate", "publish"])
    parser.add_argument("--event", type=Path, required=True)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--distribution", type=Path)
    args = parser.parse_args()
    try:
        if not re.fullmatch(r"[0-9a-f]{40}", args.source_sha):
            raise ValueError("Source commit must be an exact SHA")
        event = json.loads(args.event.read_text())
        try:
            validate_changed_manifest(args.source_sha)
        except ValueError:
            if args.mode != "gate":
                raise
            gate_output(args.output, False, args.source_sha)
            print("No approved preview manifest change in this commit")
            return
        raw = json.loads((ROOT / "release/preview.json").read_text())
        if args.mode == "gate" and raw.get("enabled") is False:
            gate_output(args.output, False, args.source_sha)
            print("Preview publication is disabled")
            return
        approved = packaging.read_manifest(ROOT / "release/preview.json")
        api = Github(args.repository)
        sha = api.ci(event)
        if sha != args.source_sha:
            raise ValueError("Checkout differs from qualified current-main SHA")
        api.approved_at(sha, approved)
        release = api.existing(approved["version"], sha)
        if args.mode == "gate":
            eligible = not release or release["draft"]
            gate_output(args.output, eligible, sha)
            print("Preview is eligible" if eligible else "Exact preview was already published; no changes")
        else:
            if args.distribution is None:
                raise ValueError("Publishing requires --distribution")
            print(publish(api, event, approved, sha, args.distribution))
    except (ValueError, OSError, KeyError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"Preview release refused: {error}\n")


if __name__ == "__main__":
    main()
