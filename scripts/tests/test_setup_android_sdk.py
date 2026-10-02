"""Regression checks for the user-local Android SDK bootstrap."""

import importlib.util
from pathlib import Path
import stat
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "setup-android-sdk.py"
SPEC = importlib.util.spec_from_file_location("crosslab_sdk_setup", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
sdk = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(sdk)


class AndroidSetupTests(unittest.TestCase):
    def test_published_platform_identifier_matches_gradle(self):
        self.assertIn("platforms/android-37.0", sdk.PACKAGES)
        self.assertNotIn("platforms;android-37", sdk.PACKAGES)

    def test_existing_partial_install_restores_executable_permission(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            bin_dir = root / "cmdline-tools" / "latest" / "bin"
            bin_dir.mkdir(parents=True)
            for name in ("sdkmanager", "android", "avdmanager"):
                path = bin_dir / name
                path.write_text("#!/bin/sh\n", encoding="utf-8")
                path.chmod(0o600)

            manager, android = sdk.sdk_tools(root)
            self.assertEqual(manager, bin_dir / "sdkmanager")
            self.assertEqual(android, bin_dir / "android")
            for name in ("sdkmanager", "android", "avdmanager"):
                self.assertTrue((bin_dir / name).stat().st_mode & stat.S_IXUSR)


if __name__ == "__main__":
    unittest.main()
