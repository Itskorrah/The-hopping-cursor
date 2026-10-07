#!/usr/bin/env python3
"""Regenerate bundled notices from the locked native dependency graph and npm packages.

Run after npm ci with current stable Rust available. Upstream notices omitted
from published crates are preserved in license-overrides.json. No target-product
code or assets are read. This inventory is not a legal review of a release.
"""
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGETS = ("x86_64-pc-windows-msvc", "aarch64-apple-darwin")
NPM = ("react", "react-dom", "scheduler", "lucide-react", "@fontsource/dm-sans", "@tauri-apps/api", "@tauri-apps/plugin-dialog")
PREFIXES = ("LICENSE", "LICENCE", "COPYING", "NOTICE", "COPYRIGHT")

def notice_files(directory):
    files = []
    for path in sorted(directory.iterdir()):
        if not path.name.upper().startswith(PREFIXES):
            continue
        if path.is_file():
            files.append(path)
        elif path.is_dir():
            files.extend(sorted(p for p in path.rglob("*") if p.is_file()))
    return files

def section(label, body):
    return "\n" + "=" * 72 + "\n" + label + "\n" + "=" * 72 + "\n" + body.strip() + "\n"

def main():
    output = ["The Hopping Cursor — third-party source notices\n\n"
              "Dependency versions are pinned by Cargo.lock and package-lock.json.\n"
              "This inventory includes build dependencies and both supported native\n"
              "platforms; not every listed package is present in each binary.\n"
              "Unmodified Rust source packages are available from the source URLs\n"
              "below. Local package notices are preserved verbatim where provided.\n"
              "Upstream and standard license texts supplement packages that omit\n"
              "license files. Installed agent skills keep their separate licenses.\n"]
    output.append(section("Original application code — MIT", (ROOT / "LICENSE").read_text()))
    for name in NPM:
        directory = ROOT / "node_modules" / name
        package = json.loads((directory / "package.json").read_text())
        files = notice_files(directory)
        if not files:
            raise RuntimeError("Missing npm notices: " + name)
        text = "\n".join("\n--- " + p.name + " ---\n" + p.read_text(errors="replace") for p in files)
        output.append(section(name + "@" + package["version"], text))

    packages = {}
    for target in TARGETS:
        metadata = json.loads(subprocess.check_output(
            ["cargo", "metadata", "--locked", "--format-version", "1", "--filter-platform", target], cwd=ROOT))
        active = {node["id"] for node in metadata["resolve"]["nodes"]}
        for package in metadata["packages"]:
            if package["id"] in active and package["source"] is not None:
                packages[package["id"]] = package
    overrides = json.loads((ROOT / "scripts/license-overrides.json").read_text())
    for package in sorted(packages.values(), key=lambda p: (p["name"], p["version"])):
        directory = Path(package["manifest_path"]).parent
        files = notice_files(directory)
        if package.get("license_file"):
            path = directory / package["license_file"]
            if path.is_file() and path not in files:
                files.append(path)
        key = package["name"] + "@" + package["version"]
        body = "Declared license: " + str(package.get("license")) + "\n"
        body += "Declared authors: " + "; ".join(package.get("authors", [])) + "\n"
        body += "Source: https://crates.io/api/v1/crates/" + package["name"] + "/" + package["version"] + "/download\n"
        body += "Repository: " + str(package.get("repository")) + "\n"
        if files:
            body += "\n".join("\n--- " + str(p.relative_to(directory)) + " ---\n" + p.read_text(errors="replace") for p in files)
        elif key in overrides:
            body += "\nUpstream notice: " + overrides[key]["source_url"] + "\n" + overrides[key]["content"]
        else:
            # Some historical packages declare SPDX terms but publish no notice.
            expression = package.get("license", "")
            chosen = "MPL-2.0" if expression == "MPL-2.0" else "MIT" if "MIT" in expression else None
            if chosen is None:
                raise RuntimeError("Review missing license text for " + key)
            body += "\nStandard SPDX text (no upstream notice published):\n" + (ROOT / "scripts/license-templates" / (chosen + ".txt")).read_text()
        output.append(section(key, body))
    for name in ("MIT", "Apache-2.0", "MPL-2.0"):
        output.append(section("Standard SPDX license text: " + name, (ROOT / "scripts/license-templates" / (name + ".txt")).read_text()))
    path = ROOT / "apps/desktop/public/THIRD_PARTY_NOTICES.txt"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(output))
    print("Wrote native/font/UI notice inventory:", len(packages), "Rust packages;", path.stat().st_size, "bytes")

if __name__ == "__main__":
    main()
