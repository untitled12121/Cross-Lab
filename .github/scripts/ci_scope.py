"""Choose the smallest safe GitHub Actions scope for a Cross-Lab change."""

import argparse
import os
from pathlib import Path
import re
import subprocess
from dataclasses import dataclass


@dataclass(frozen=True)
class Checks:
    format: bool = False
    rust: bool = False
    android: bool = False
    audit: bool = False
    full: bool = False

    def outputs(self) -> str:
        return "".join(
            f"{key}={str(getattr(self, key)).lower()}\n"
            for key in ("format", "rust", "android", "audit", "full")
        )


FULL = Checks(True, True, True, True, True)
EMPTY = Checks()


def is_document(path: str) -> bool:
    return path.startswith("docs/") or path.endswith((".md", ".mdx"))


def choose_checks(paths: list[str]) -> Checks:
    """Unknown build inputs fail open to full checks, not an accidental skip."""
    changes = [path for path in paths if not is_document(path)]
    if not changes:
        return EMPTY
    rust = android = audit = full = False
    for path in changes:
        if path.startswith("fuzz/"):
            # The independently path-scoped fuzz.yml owns these changes.
            continue
        if path.startswith("apps/desktop/") or path.startswith(("apps/sim/", "experiments/", "xtask/")):
            rust = True
        elif path.startswith("apps/android/"):
            android = True
            if path.endswith((".gradle", ".gradle.kts")) or path.endswith("gradle-wrapper.properties"):
                full = True
        elif path.startswith(("crates/", "transports/")):
            rust = android = full = True
            if path.startswith(("crates/crypto/", "crates/identity/", "crates/policy/",
                                "crates/protocol/", "crates/identity-store/",
                                "crates/policy-store/")):
                audit = True
        elif path in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml") or path.startswith((
            ".cargo/", "scripts/", "design/", ".github/", "config/"
        )):
            rust = android = full = True
            if path in ("Cargo.toml", "Cargo.lock") or path.startswith(".github/workflows/"):
                audit = True
        else:
            return FULL
    return Checks(rust, rust, android, audit, full)


def manual_checks(scope: str) -> Checks:
    return {
        "full": FULL,
        "rust": Checks(True, True, False, True, True),
        "android": Checks(False, False, True, False, True),
        "format": Checks(True),
        "audit": Checks(audit=True),
    }[scope]


def changed_files(base: str, head: str) -> list[str]:
    sha = re.compile(r"[0-9a-f]{40}\Z")
    if not sha.fullmatch(base) or not sha.fullmatch(head) or base == "0" * 40:
        raise ValueError("missing or invalid Git change boundary")
    result = subprocess.check_output(
        ["git", "diff", "--name-only", "--diff-filter=ACMRT", "-z", base, head]
    )
    return [os.fsdecode(path) for path in result.split(b"\0") if path]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--event", required=True)
    parser.add_argument("--profile", default="full")
    parser.add_argument("--base", default="")
    parser.add_argument("--head", default="")
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    if args.event == "workflow_dispatch":
        checks = manual_checks(args.profile)
    else:
        try:
            checks = choose_checks(changed_files(args.base, args.head))
        except (ValueError, OSError, subprocess.CalledProcessError) as error:
            print(f"Cannot safely classify diff ({error}); running all checks.")
            checks = FULL

    with args.output.open("a", encoding="utf-8") as output:
        output.write(checks.outputs())
    print(f"CI scope: {checks}")


if __name__ == "__main__":
    main()
