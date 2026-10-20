"""Fail closed if any GitHub Actions check selected by ci_scope did not run."""

from __future__ import annotations

import os
from collections.abc import Mapping

JOB_NAMES = ("format", "audit", "rust", "android")


def verify_gate(
    selected: Mapping[str, str],
    results: Mapping[str, str],
    changes_result: str,
) -> list[str]:
    """Return all routing/status mismatches; an empty list means a passing gate."""
    errors: list[str] = []
    if changes_result != "success":
        errors.append(f"scope selection did not succeed: {changes_result!r}")

    for job in JOB_NAMES:
        requested = selected.get(job)
        actual = results.get(job)
        if requested not in ("true", "false"):
            errors.append(f"{job}: selection was missing or invalid: {requested!r}")
            continue
        # An unselected job must be deliberately skipped. A selected job must
        # succeed: skipped, neutral, pending, cancelled and unknown all fail.
        expected = "success" if requested == "true" else "skipped"
        if actual != expected:
            errors.append(
                f"{job}: selected={requested}, expected {expected!r}, got {actual!r}"
            )

    # The Rust job depends on formatting; a broken routing rule may otherwise
    # skip Rust even when the selector asked for it.
    if selected.get("rust") == "true" and selected.get("format") != "true":
        errors.append("rust requested without prerequisite format check")

    return errors


def main() -> int:
    selections = {job: os.environ.get(f"SELECT_{job.upper()}") for job in JOB_NAMES}
    results = {job: os.environ.get(f"RESULT_{job.upper()}") for job in JOB_NAMES}
    errors = verify_gate(
        selected=selections,
        results=results,
        changes_result=os.environ.get("RESULT_CHANGES", ""),
    )
    for error in errors:
        print(f"::error::{error}")
    if errors:
        return 1
    print("Every selected check succeeded; every unselected check was skipped.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
