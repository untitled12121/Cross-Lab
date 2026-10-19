"""Pure routing checks: CI must never silently drop sensitive or unknown changes."""

import unittest

from ci_scope import Checks, choose_checks, manual_checks


class ScopeTests(unittest.TestCase):
    def test_documentation_only_does_not_schedule_builds(self):
        self.assertEqual(
            choose_checks(["AGENTS.md", "docs/development/CURRENT.md", "README.mdx"]),
            Checks(),
        )

    def test_android_compose_only(self):
        self.assertEqual(
            choose_checks(["apps/android/app/src/main/kotlin/dev/crosslab/android/MainActivity.kt"]),
            Checks(android=True),
        )

    def test_gpui_only(self):
        self.assertEqual(
            choose_checks(["apps/desktop/src/pages/control_center/page.rs"]),
            Checks(format=True, rust=True),
        )

    def test_shared_rust_runs_both_platforms_and_deeper_checks(self):
        self.assertEqual(
            choose_checks(["crates/mobile-ffi/src/lib.rs"]),
            Checks(format=True, rust=True, android=True, full=True),
        )

    def test_policy_and_lockfile_trigger_dependency_audit(self):
        expected = Checks(True, True, True, True, True)
        for path in ("crates/policy/src/lib.rs", "Cargo.lock", ".github/workflows/ci.yml"):
            with self.subTest(path=path):
                self.assertEqual(choose_checks([path]), expected)

    def test_design_assets_affect_both_platforms(self):
        self.assertEqual(
            choose_checks(["design/themes/dark.json"]),
            Checks(format=True, rust=True, android=True, full=True),
        )

    def test_fuzz_only_is_owned_by_fuzz_workflow(self):
        self.assertEqual(choose_checks(["fuzz/fuzz_targets/control_frame.rs"]), Checks())

    def test_unknown_inputs_fail_open(self):
        self.assertEqual(choose_checks(["mystery/build-config.xyz"]), Checks(True, True, True, True, True))

    def test_manual_profiles_and_outputs(self):
        self.assertEqual(manual_checks("rust"), Checks(True, True, False, True, True))
        self.assertEqual(manual_checks("android"), Checks(False, False, True, False, True))
        self.assertEqual(manual_checks("format"), Checks(format=True))
        self.assertEqual(manual_checks("audit"), Checks(audit=True))
        self.assertIn("rust=true\n", manual_checks("full").outputs())
        self.assertIn("android=false\n", manual_checks("rust").outputs())


if __name__ == "__main__":
    unittest.main()
