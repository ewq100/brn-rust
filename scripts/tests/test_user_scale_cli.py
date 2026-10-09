"""Cancellation-boundary regression; synthetic children only, no BRN fixture."""
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
SCRIPT = Path(__file__).resolve().parents[1] / "qualify-user-scale-cli.py"
spec = importlib.util.spec_from_file_location("user_scale_cli", SCRIPT)
scale = importlib.util.module_from_spec(spec)
spec.loader.exec_module(scale)


class Child:
    pid = 42424
    returncode = None
    killed = False
    reaped = False

    def poll(self):
        return self.returncode

    def wait(self, timeout=None):
        if not self.killed or timeout is not None:
            raise AssertionError("Cancelled command continued into ordinary waiting")
        self.reaped = True
        self.returncode = -signal.SIGKILL
        return self.returncode


class SpawnCancellationTests(unittest.TestCase):
    def test_creation_and_registration_cancellation_kills_and_reaps(self):
        registration = next(index for index, line in enumerate(SCRIPT.read_text().splitlines(), 1)
                            if line.strip() == "self.active = process")
        for boundary in ("before-return", "before-registration", "real-child-before-return"):
            signals = (signal.SIGTERM,) if boundary.startswith("real") else (
                signal.SIGALRM, signal.SIGINT, signal.SIGTERM)
            for signum in signals:
                with self.subTest(boundary=boundary, signal=signum), tempfile.TemporaryDirectory() as directory:
                    root = Path(directory)
                    campaign = scale.Campaign(root, root, time.monotonic())
                    campaign.binary = Path(sys.executable).resolve()
                    campaign.binary_info = scale.file_info(campaign.binary)
                    campaign.expected_sha = campaign.binary_info["sha256"]
                    campaign.logs = campaign.data = campaign.credentials = root
                    campaign.env = {"PATH": "/usr/bin:/bin"}
                    child, fired = Child(), []
                    real_popen = subprocess.Popen

                    def admit(*args, **kwargs):
                        nonlocal child
                        if boundary.startswith("real"):
                            child = real_popen([sys.executable, "-c", "import time; time.sleep(10)"], **kwargs)
                        if boundary != "before-registration":
                            fired.append(signum)
                            os.kill(os.getpid(), signum)
                        return child

                    def trace(frame, event, _arg):
                        if (not fired and event == "line" and frame.f_code is scale.Campaign.run.__code__
                                and frame.f_lineno == registration):
                            fired.append(signum)
                            os.kill(os.getpid(), signum)
                        return trace

                    def kill(pid, delivered):
                        self.assertEqual((pid, delivered), (child.pid, signal.SIGKILL))
                        child.killed = True

                    old_handler, old_trace = signal.getsignal(signum), sys.gettrace()
                    signal.signal(signum, campaign.interrupt)
                    try:
                        with patch.object(scale.subprocess, "Popen", side_effect=admit) as popen:
                            if boundary.startswith("real"):
                                with self.assertRaises(scale.QualificationError):
                                    campaign.run("cancelled", [], "status")
                            else:
                                with patch.object(scale.os, "killpg", side_effect=kill):
                                    if boundary == "before-registration":
                                        sys.settrace(trace)
                                    with self.assertRaises(scale.QualificationError):
                                        campaign.run("cancelled", [], "status")
                            self.assertEqual(popen.call_count, 1)
                        self.assertEqual(fired, [signum])
                        self.assertIsNone(campaign.active)
                        self.assertFalse(campaign.spawning)
                        self.assertIsNone(campaign.pending_signal)
                        self.assertEqual(child.returncode, -signal.SIGKILL)
                        if not boundary.startswith("real"):
                            self.assertTrue(child.killed and child.reaped)
                        receipt = json.loads((root / "001-cancelled.json").read_text())
                        self.assertEqual(receipt["outcome"], "failed")
                        self.assertEqual(receipt["returncode"], -signal.SIGKILL)
                        self.assertNotIn("process_wall_seconds", receipt)
                    finally:
                        sys.settrace(old_trace)
                        signal.signal(signum, old_handler)
                        if boundary.startswith("real") and child.poll() is None:
                            os.killpg(child.pid, signal.SIGKILL)
                            child.wait(timeout=5)


if __name__ == "__main__":
    unittest.main()
