#!/usr/bin/env python3
"""Verify a finished Mac disk image and its packaged application signature."""

import json
import plistlib
import subprocess
import sys
from pathlib import Path


def verify(image_path: Path) -> None:
    subprocess.run(["hdiutil", "verify", str(image_path)], check=True)
    attachment = plistlib.loads(
        subprocess.check_output(
            ["hdiutil", "attach", "-readonly", "-nobrowse", "-plist", str(image_path)]
        )
    )
    mount = next(
        entity["mount-point"]
        for entity in attachment["system-entities"]
        if "mount-point" in entity
    )
    try:
        apps = list(Path(mount).glob("*.app"))
        if len(apps) != 1:
            raise ValueError(f"Expected one packaged application, found {len(apps)}")
        app = apps[0]
        subprocess.run(
            ["codesign", "--verify", "--deep", "--strict", "--verbose=2", str(app)],
            check=True,
        )
        info = plistlib.loads((app / "Contents/Info.plist").read_bytes())
        config_path = (
            Path(__file__).resolve().parents[1]
            / "apps/desktop/src-tauri/tauri.conf.json"
        )
        config = json.loads(config_path.read_text())
        if info["CFBundleIdentifier"] != config["identifier"]:
            raise ValueError("Packaged application identifier does not match config")
        if info["CFBundleShortVersionString"] != config["version"]:
            raise ValueError("Packaged application version does not match config")
        print(f"Verified packaged application: {info['CFBundleIdentifier']}")
    finally:
        subprocess.run(["hdiutil", "detach", mount], check=True)


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("Usage: verify-macos-package.py /path/to/package.dmg")
    verify(Path(sys.argv[1]).resolve())
