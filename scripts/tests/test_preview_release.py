"""Release eligibility and publication ordering without network or credentials."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("preview_release", ROOT / "scripts/release-preview.py")
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)
SHA = "a" * 40
REPOSITORY = "example/brn"


class EligibilityTests(unittest.TestCase):
    def setUp(self):
        self.run = dict(id=12, run_attempt=2, name="BRN CI", path=".github/workflows/ci.yml",
                        head_sha=SHA, head_repository={"full_name": REPOSITORY},
                        event="push", head_branch="main", status="completed", conclusion="failure")
        self.jobs = [dict(name=name, status="completed", conclusion="success", head_sha=SHA)
                     for name in release.REQUIRED]
        self.event = dict(action="completed", workflow_run=copy.deepcopy(self.run))

    def test_four_passed_required_jobs_allow_informational_platform_failure(self):
        self.jobs.append(dict(name="Core and CLI (windows-2025)", status="completed", conclusion="failure", head_sha=SHA))
        self.assertEqual(release.validate_ci(self.run, self.jobs, SHA, REPOSITORY), SHA)
        release.validate_event(self.event, self.run)

    def test_fork_pr_stale_sha_incomplete_or_wrong_workflow_refuse(self):
        for changes in ({"head_repository": {"full_name": "fork/brn"}}, {"event": "pull_request"},
                        {"head_branch": "other"}, {"status": "in_progress"},
                        {"path": ".github/workflows/other.yml"}, {"name": "Another CI"}):
            with self.subTest(changes=changes), self.assertRaises(ValueError):
                release.validate_ci(dict(self.run, **changes), self.jobs, SHA, REPOSITORY)
        with self.assertRaises(ValueError):
            release.validate_ci(self.run, self.jobs, "b" * 40, REPOSITORY)
        for key, value in (("id", 13), ("run_attempt", 3), ("head_sha", "b" * 40)):
            with self.subTest(key=key), self.assertRaises(ValueError):
                release.validate_event(self.event, dict(self.run, **{key: value}))

    def test_missing_failed_skipped_stale_duplicate_required_jobs_refuse(self):
        bad = [self.jobs[1:], self.jobs + [self.jobs[0]]]
        for value in ("failure", "skipped", "cancelled", None):
            bad.append([dict(self.jobs[0], conclusion=value)] + self.jobs[1:])
        bad.append([dict(self.jobs[0], head_sha="b" * 40)] + self.jobs[1:])
        bad.append([dict(self.jobs[0], status="queued")] + self.jobs[1:])
        for jobs in bad:
            with self.subTest(jobs=jobs), self.assertRaises(ValueError):
                release.validate_ci(self.run, jobs, SHA, REPOSITORY)

    def test_unchanged_or_unapproved_manifest_refuses(self):
        with patch.object(release.subprocess, "run") as run:
            run.return_value.stdout = ""
            with self.assertRaises(ValueError):
                release.validate_changed_manifest(SHA)
            run.return_value.stdout = "release/preview.json\n"
            release.validate_changed_manifest(SHA)
        approved = json.loads((ROOT / "release/preview.json").read_text())
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "approval.json"
            for key, value in (("enabled", False), ("version", "v0.1.0"), ("build_number", True),
                               ("minimum_macos", "13.0"), ("features", ["native-ui", "native-retrieval"])):
                path.write_text(json.dumps(dict(approved, **{key: value})))
                with self.subTest(key=key), self.assertRaises(ValueError):
                    release.packaging.read_manifest(path)

    def test_no_local_token_discovery(self):
        with patch.dict(release.os.environ, {"GITHUB_ACTIONS": "false", "GITHUB_TOKEN": "synthetic"}):
            with self.assertRaises(ValueError):
                release.Github(REPOSITORY)


class FakeGithub:
    def __init__(self, advance=False, corrupt_upload=False, existing=None):
        self.events = []
        self.approval_checks = 0
        self.advance = advance
        self.corrupt_upload = corrupt_upload
        self.release = existing
        self.uploaded = []

    def approved_at(self, sha, approved):
        self.approval_checks += 1
        self.events.append("approval")
        if self.advance and self.approval_checks == 2:
            raise ValueError("Main advanced")

    def ci(self, event):
        self.events.append("ci")
        return SHA

    def existing(self, version, sha):
        self.events.append("existing")
        return self.release

    def request(self, method, path, payload=None, binary=False, **kwargs):
        if method == "POST" and path == "/releases":
            self.events.append("draft")
            self.release = dict(payload, id=1, upload_url="https://uploads.github.com/repos/example/brn/releases/1/assets{?name}")
            return self.release
        if method == "GET":
            return self.uploaded
        if method == "POST":
            self.events.append("upload")
            asset = dict(name=path.split("name=")[1], size=len(payload),
                         digest="sha256:" + hashlib.sha256(payload).hexdigest())
            if self.corrupt_upload:
                asset["digest"] = "sha256:" + "0" * 64
            self.uploaded.append(asset)
            return asset
        if method == "PATCH":
            self.events.append("publish")
            return dict(draft=False, prerelease=True, html_url="https://example.invalid/release")
        raise AssertionError((method, path))


class PublicationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name).resolve()
        self.approved = release.packaging.read_manifest(ROOT / "release/preview.json")
        name = f"BRN-Threads-{self.approved['version']}-macos-arm64"
        self.archive = self.root / f"{name}.zip"
        self.archive.write_bytes(b"synthetic archive bytes")
        self.receipt = self.root / f"{name}.manifest.json"
        self.receipt.write_text(json.dumps(dict(self.approved, source_sha=SHA, signing="ad-hoc; not notarized",
            archive=dict(name=self.archive.name, sha256=release.packaging.digest(self.archive), bytes=self.archive.stat().st_size))))
        (self.root / "SHA256SUMS").write_text("".join(f"{release.packaging.digest(path)}  {path.name}\n" for path in (self.archive, self.receipt)))

    def tearDown(self):
        self.temp.cleanup()

    def test_publish_only_after_uploads_and_final_exact_identity_guard(self):
        api = FakeGithub()
        self.assertEqual(release.publish(api, {}, self.approved, SHA, self.root), "https://example.invalid/release")
        self.assertEqual(api.events, ["approval", "ci", "existing", "draft", "upload", "upload", "upload",
                                      "ci", "approval", "existing", "publish"])

    def test_main_advance_or_bad_upload_leaves_draft_unpublished(self):
        for api in (FakeGithub(advance=True), FakeGithub(corrupt_upload=True)):
            with self.subTest(api=api), self.assertRaises(ValueError):
                release.publish(api, {}, self.approved, SHA, self.root)
            self.assertNotIn("publish", api.events)
            self.assertTrue(api.release["draft"])

    def test_wrong_source_corrupt_archive_checksum_or_incomplete_receipt_refuses(self):
        with self.assertRaises(ValueError):
            release.validate_delivery(self.root, self.approved, "b" * 40)
        before = self.archive.read_bytes()
        self.archive.write_bytes(b"tampered")
        with self.assertRaises(ValueError):
            release.validate_delivery(self.root, self.approved, SHA)
        self.archive.write_bytes(before)
        (self.root / ".incomplete").write_text(SHA)
        with self.assertRaises(ValueError):
            release.validate_delivery(self.root, self.approved, SHA)
        (self.root / ".incomplete").unlink()
        (self.root / "SHA256SUMS").write_text("wrong")
        with self.assertRaises(ValueError):
            release.validate_delivery(self.root, self.approved, SHA)

    def test_existing_published_release_is_never_overwritten(self):
        api = FakeGithub(existing={"draft": False})
        with self.assertRaises(ValueError):
            release.publish(api, {}, self.approved, SHA, self.root)
        self.assertNotIn("draft", api.events)
        self.assertNotIn("publish", api.events)

    def test_tag_or_release_identity_is_never_retargeted(self):
        api = object.__new__(release.Github)
        for responses in ([{"object": {"type": "commit", "sha": "b" * 40}}],
                          [None, [{"tag_name": self.approved["version"], "target_commitish": "b" * 40, "prerelease": True}]],
                          [{"object": {"type": "commit", "sha": SHA}}, []]):
            with patch.object(api, "request", side_effect=responses), self.assertRaises(ValueError):
                api.existing(self.approved["version"], SHA)

    def test_exact_existing_draft_resumes_without_replacing_assets(self):
        draft = dict(id=7, draft=True, prerelease=True, target_commitish=SHA,
                     tag_name=self.approved["version"],
                     upload_url="https://uploads.github.com/repos/example/brn/releases/7/assets{?name}")
        api = FakeGithub(existing=draft)
        api.uploaded = [dict(name=path.name, size=path.stat().st_size,
                             digest="sha256:" + release.packaging.digest(path))
                        for path in release.validate_delivery(self.root, self.approved, SHA)]
        release.publish(api, {}, self.approved, SHA, self.root)
        self.assertNotIn("draft", api.events)
        self.assertNotIn("upload", api.events)
        self.assertEqual(api.events[-1], "publish")

    def test_deleted_replaced_published_or_retargeted_draft_blocks_final_patch(self):
        draft = dict(id=7, draft=True, prerelease=True, target_commitish=SHA,
                     tag_name=self.approved["version"],
                     upload_url="https://uploads.github.com/repos/example/brn/releases/7/assets{?name}")
        for refreshed in (None, dict(draft, id=8), dict(draft, draft=False),
                          dict(draft, target_commitish="b" * 40), dict(draft, prerelease=False),
                          dict(draft, tag_name=self.approved["version"] + "-other")):
            api = FakeGithub(existing=draft)
            with self.subTest(refreshed=refreshed), patch.object(api, "existing", side_effect=[draft, refreshed]):
                with self.assertRaises(ValueError):
                    release.publish(api, {}, self.approved, SHA, self.root)
                self.assertNotIn("publish", api.events)


class ReleaseLookupTests(unittest.TestCase):
    def setUp(self):
        self.api = object.__new__(release.Github)
        self.version = "v0.1.0-preview.1"
        self.draft = dict(id=7, tag_name=self.version, target_commitish=SHA,
                          prerelease=True, draft=True)

    def test_untagged_draft_on_later_page_is_found(self):
        unrelated = [dict(tag_name=f"unrelated-{i}") for i in range(100)]
        with patch.object(self.api, "request", side_effect=[None, unrelated, [self.draft]]) as request:
            self.assertEqual(self.api.existing(self.version, SHA), self.draft)
        self.assertEqual([call.args[1] for call in request.call_args_list],
                         [f"/git/ref/tags/{self.version}", "/releases?per_page=100&page=1",
                          "/releases?per_page=100&page=2"])

    def test_duplicate_version_across_pages_refuses(self):
        first = [self.draft] + [dict(tag_name=f"unrelated-{i}") for i in range(99)]
        with patch.object(self.api, "request", side_effect=[None, first, [dict(self.draft, id=8)]]):
            with self.assertRaises(ValueError):
                self.api.existing(self.version, SHA)

    def test_published_state_preserved_and_nonpreview_identity_refuses(self):
        published = dict(self.draft, draft=False)
        with patch.object(self.api, "request", side_effect=[None, [published]]):
            self.assertEqual(self.api.existing(self.version, SHA), published)
        with patch.object(self.api, "request", side_effect=[None, [dict(self.draft, prerelease=False)]]):
            with self.assertRaises(ValueError):
                self.api.existing(self.version, SHA)


if __name__ == "__main__":
    unittest.main()
