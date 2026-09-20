#!/usr/bin/env python3
"""Install the pinned Android SDK toolchain used by Cross-Lab.

The script installs into a user-owned SDK directory chosen by the Rust builder.
It never modifies system packages or accepts Android SDK licenses silently.
"""

from __future__ import annotations

import hashlib
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import tempfile
import urllib.request
import zipfile

TOOLS_VERSION = "15859902"
PACKAGES = (
    "platform-tools",
    "platforms;android-37",
    "build-tools;36.0.0",
    "ndk;28.2.13676358",
)
DOWNLOADS = {
    ("Linux", "x86_64"): (
        f"https://dl.google.com/android/repository/commandlinetools-linux-{TOOLS_VERSION}_latest.zip",
        "4e4c464f145a7512b57d088ac6c278c03c9eea610886b35a5e0804e74eedf583",
    ),
    ("Darwin", "x86_64"): (
        f"https://dl.google.com/android/repository/commandlinetools-mac_x86_64-{TOOLS_VERSION}_latest.zip",
        "c5a6378ab5cf7e0d5701921405115befff13e9ff7417fb588389338f8bd050f3",
    ),
    ("Darwin", "arm64"): (
        f"https://dl.google.com/android/repository/commandlinetools-mac_arm64-{TOOLS_VERSION}_latest.zip",
        "835b62a26162b229b441d1f6d4680383815a270809eb33522c0d480fa5002c4e",
    ),
}


def normalized_machine() -> str:
    machine = platform.machine().lower()
    if machine in {"amd64", "x64"}:
        return "x86_64"
    if machine in {"aarch64", "arm64"}:
        return "arm64"
    return machine


def confirm_license() -> None:
    if os.environ.get("CROSSLAB_ANDROID_SDK_LICENSE_ACCEPTED") == "1":
        return

    print("Cross-Lab needs the Android SDK command-line tools.")
    print("Review the Android SDK license before continuing:")
    print("  https://developer.android.com/studio/terms")
    answer = input("Type 'yes' to download the Android SDK tools: ").strip().lower()
    if answer != "yes":
        raise SystemExit("Android SDK setup cancelled; no SDK files were installed.")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def download_tools(root: Path) -> Path:
    key = (platform.system(), normalized_machine())
    try:
        url, expected_sha = DOWNLOADS[key]
    except KeyError as error:
        raise SystemExit(
            f"Android command-line tool bootstrap is not configured for {key[0]} {key[1]}."
        ) from error

    confirm_license()
    print("crosslab: downloading pinned Android command-line tools")

    with tempfile.TemporaryDirectory(prefix="crosslab-android-sdk-") as temp_dir:
        temp = Path(temp_dir)
        archive = temp / "commandline-tools.zip"
        with urllib.request.urlopen(url) as response, archive.open("wb") as output:
            shutil.copyfileobj(response, output)

        actual_sha = sha256(archive)
        if actual_sha != expected_sha:
            raise SystemExit(
                "Android command-line tools checksum mismatch; refusing to install."
            )

        with zipfile.ZipFile(archive) as source:
            source.extractall(temp / "unpacked")

        extracted = temp / "unpacked" / "cmdline-tools"
        if not extracted.is_dir():
            raise SystemExit("Android command-line tools archive layout is invalid.")

        destination = root / "cmdline-tools" / "latest"
        destination.parent.mkdir(parents=True, exist_ok=True)
        if destination.exists():
            shutil.rmtree(destination)
        shutil.copytree(extracted, destination)

    return root / "cmdline-tools" / "latest" / "bin" / "sdkmanager"


def sdkmanager(root: Path) -> Path:
    executable = root / "cmdline-tools" / "latest" / "bin" / "sdkmanager"
    return executable if executable.is_file() else download_tools(root)


def run(command: list[str]) -> None:
    try:
        subprocess.run(command, check=True)
    except subprocess.CalledProcessError as error:
        raise SystemExit(error.returncode) from error


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: setup-android-sdk.py <sdk-root>")

    root = Path(sys.argv[1]).expanduser()
    root.mkdir(parents=True, exist_ok=True)
    manager = sdkmanager(root)

    print("crosslab: Android SDK licenses may require confirmation")
    run([str(manager), f"--sdk_root={root}", "--licenses"])

    print("crosslab: installing pinned Android SDK packages")
    run([str(manager), f"--sdk_root={root}", *PACKAGES])
    print("crosslab: Android SDK setup complete")


if __name__ == "__main__":
    main()
