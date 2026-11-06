"""Approved-release preflight uses recorded API-shaped synthetic bytes, offline."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("published_update", ROOT / "scripts/prepare-published-macos-update.py")
published = importlib.util.module_from_spec(spec)
spec.loader.exec_module(published)


class PublishedPreparationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.output = Path(self.temp.name) / "prepared"
        self.feed = {}
        self.releases = {}
        self.manifests = {}
        self.calls = []
        for number in (1, 2):
            version = f"v0.2.0-alpha.{number}"
            approved = json.loads((ROOT / "release/preview.json").read_text())
            approved.update(version=version, build_number=number)
            source = str(number) * 40
            manifest = dict(approved, source_sha=source)
            name = f"BRN-Threads-{version}-macos-arm64.manifest.json"
            base = f"https://github.com/{published.REPO}/releases/download/{version}/"
            data = json.dumps(manifest).encode()
            sums = f"{hashlib.sha256(data).hexdigest()}  {name}\n".encode()
            release = dict(tag_name=version, draft=False, published_at="2026-11-06T12:00:00Z", target_commitish=source,
                           assets=[dict(name=n, browser_download_url=base+n, size=len(content), digest="sha256:"+hashlib.sha256(content).hexdigest())
                                   for n, content in ((name, data), ("SHA256SUMS", sums))])
            self.releases[version] = release
            self.manifests[version] = manifest
            self.feed[base+name] = data
            self.feed[base+"SHA256SUMS"] = sums
            self.feed[f"https://raw.githubusercontent.com/{published.REPO}/{source}/release/preview.json"] = json.dumps(approved).encode()
        self.version_a = "v0.2.0-alpha.1"
        self.version_b = "v0.2.0-alpha.2"

    def tearDown(self):
        self.temp.cleanup()

    def download(self, url, destination, maximum):
        self.calls.append(url)
        prefix = f"https://api.github.com/repos/{published.REPO}/releases/tags/"
        data = json.dumps(self.releases[url.removeprefix(prefix)]).encode() if url.startswith(prefix) else self.feed[url]
        self.assertLessEqual(len(data), maximum)
        destination.write_bytes(data)

    def prepare(self):
        return published.prepare_pair(self.version_a, self.version_b, self.output, self.download)

    def test_approved_legacy_pair_is_blocked_without_downloading_or_running_code(self):
        report = self.prepare()
        self.assertEqual(report["status"], "blocked")
        self.assertEqual(report["A"]["status"], "blocked")
        self.assertEqual(report["B"]["status"], "blocked")
        self.assertTrue(all(not url.endswith(".zip") for url in self.calls))
        self.assertEqual(json.loads((self.output / "preparation.json").read_text()), report)

    def test_draft_or_wrong_tag_is_not_approved_artifact(self):
        self.releases[self.version_a]["draft"] = True
        self.assertIn("published release", self.prepare()["error"])

    def test_corrupt_download_is_rejected(self):
        key = next(key for key in self.feed if key.endswith(".manifest.json"))
        self.feed[key] += b" "
        self.assertIn("digest/size", self.prepare()["error"])

    def test_ambiguous_assets_are_rejected(self):
        self.releases[self.version_a]["assets"].append(copy.deepcopy(self.releases[self.version_a]["assets"][0]))
        self.assertIn("ambiguous", self.prepare()["error"])

    def test_release_must_name_exact_source_commit(self):
        self.releases[self.version_a]["target_commitish"] = "main"
        self.assertIn("source identity", self.prepare()["error"])

    def test_manifest_requires_source_approval(self):
        key = next(key for key in self.feed if key.endswith("release/preview.json"))
        value = json.loads(self.feed[key])
        value["build_number"] = 55
        self.feed[key] = json.dumps(value).encode()
        self.assertIn("source-approved", self.prepare()["error"])

    def test_output_collision_is_refused(self):
        self.output.mkdir()
        with self.assertRaisesRegex(ValueError, "absent absolute"):
            self.prepare()


if __name__ == "__main__":
    unittest.main()
