"""The CI Gate must never pass because a selected job was accidentally skipped."""

import os
import unittest
from unittest.mock import patch

from ci_gate import JOB_NAMES, main, verify_gate


def scope(**selected):
    return {name: "true" if selected.get(name, False) else "false" for name in JOB_NAMES}


def results(**actual):
    return {name: actual.get(name, "skipped") for name in JOB_NAMES}


class GateTests(unittest.TestCase):
    def test_manual_full_requires_all_verifications(self):
        self.assertEqual(
            verify_gate(
                scope(format=True, audit=True, rust=True, android=True),
                results(format="success", audit="success", rust="success", android="success"),
                "success",
            ),
            [],
        )

    def test_android_only_and_format_only_pass_with_intentional_skips(self):
        self.assertEqual(
            verify_gate(scope(android=True), results(android="success"), "success"),
            [],
        )
        self.assertEqual(
            verify_gate(scope(format=True), results(format="success"), "success"),
            [],
        )

    def test_no_selected_checks_remains_valid_for_fuzz_only_change(self):
        self.assertEqual(verify_gate(scope(), results(), "success"), [])

    def test_selected_skipped_never_passes(self):
        for job in JOB_NAMES:
            with self.subTest(job=job):
                selected = scope(**{job: True})
                if job == "rust":
                    selected["format"] = "true"
                actual = results(**({"format": "success"} if job == "rust" else {}))
                self.assertTrue(verify_gate(selected, actual, "success"))

    def test_failed_cancelled_and_unknown_outcomes_never_pass(self):
        for status in ("failure", "cancelled", "neutral", "pending", "", None):
            with self.subTest(status=status):
                errors = verify_gate(
                    scope(android=True), results(android=status), "success"
                )
                self.assertTrue(any("android" in error for error in errors))

    def test_unselected_job_running_unexpectedly_does_not_mask_routing_error(self):
        errors = verify_gate(scope(), results(android="success"), "success")
        self.assertTrue(any("android" in error for error in errors))

    def test_missing_and_malformed_routing_outputs_fail_closed(self):
        for value in (None, "", "True", "success", "false\ntrue"):
            with self.subTest(value=value):
                requested = scope()
                requested["audit"] = value
                self.assertTrue(verify_gate(requested, results(), "success"))

    def test_scoping_failure_cannot_be_ignored(self):
        for status in ("failure", "cancelled", "skipped", ""):
            with self.subTest(status=status):
                self.assertTrue(verify_gate(scope(), results(), status))

    def test_rust_requires_format_selected_and_successful(self):
        self.assertTrue(verify_gate(scope(rust=True), results(rust="success"), "success"))
        self.assertTrue(
            verify_gate(
                scope(rust=True, format=True),
                results(rust="success", format="skipped"),
                "success",
            )
        )

    def test_cli_process_exits_failure_for_missing_or_skipped_selected_job(self):
        environ = {
            "RESULT_CHANGES": "success",
            **{f"SELECT_{name.upper()}": scope(android=True)[name] for name in JOB_NAMES},
            **{f"RESULT_{name.upper()}": results()[name] for name in JOB_NAMES},
        }
        with patch.dict(os.environ, environ, clear=True):
            self.assertEqual(main(), 1)
            os.environ["RESULT_ANDROID"] = "success"
            self.assertEqual(main(), 0)


if __name__ == "__main__":
    unittest.main()
