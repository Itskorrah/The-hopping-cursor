#!/usr/bin/env node
/**
 * Local verification for development checkouts when GitHub Actions cannot run.
 * No dependency installation, network fetch, signing, publishing or config edits.
 */
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const help = `Usage: node scripts/verify-local.mjs [--dry-run] [--native] [--help]

Runs local regression tests, formatting, Clippy, TypeScript, UI build and Playwright.
--native   Additionally build a native Windows NSIS / macOS DMG package.
--dry-run  Show every command without executing or checking toolchains.

Prerequisites: Node.js 22+, stable Rust with rustfmt/Clippy, and npm ci already
completed. Playwright Chromium and native platform prerequisites are required
for the corresponding tests. No credentials or OS security settings are changed.`;
const args = new Set(process.argv.slice(2));
const unknown = [...args].filter((arg) => !["--dry-run", "--native", "--help"].includes(arg));
if (unknown.length) {
  console.error(`Unknown argument(s): ${unknown.join(", ")}\n${help}`);
  process.exit(2);
}
if (args.has("--help")) {
  console.log(help);
  process.exit(0);
}
if (!["Cargo.toml", "package.json", "apps/desktop/package.json"].every((p) => existsSync(resolve(root, p)))) {
  console.error(`Expected The Hopping Cursor repository root at ${root}.`);
  process.exit(2);
}
const dryRun = args.has("--dry-run");
const native = args.has("--native");
const npm = process.platform === "win32" ? "npm.cmd" : "npm";
const commands = [
  ["Verifier smoke tests", process.execPath, ["--test", "scripts/verify-local.test.mjs"]],
  ["Rust formatting", "cargo", ["fmt", "--all", "--", "--check"]],
  ["Rust lint", "cargo", ["clippy", "--locked", "-p", "hop-core", "-p", "hopping-cursor", "--all-targets", "--", "-D", "warnings"]],
  ["Rust unit and integration tests", "cargo", ["test", "--locked"]],
  ["TypeScript type checking", npm, ["run", "typecheck"]],
  ["Frontend production build", npm, ["run", "build"]],
  ["Browser end-to-end tests", npm, ["run", "test:ui"]],
];
if (native) {
  if (process.platform === "win32") {
    commands.push(["Native Windows NSIS package", npm, ["run", "tauri", "-w", "apps/desktop", "--", "build", "--target", "x86_64-pc-windows-msvc", "--bundles", "nsis", "--", "--locked"]]);
  } else if (process.platform === "darwin") {
    commands.push(["Native macOS DMG package", npm, ["run", "tauri", "-w", "apps/desktop", "--", "build", "--bundles", "dmg", "--", "--locked"]]);
  } else {
    console.error("--native packaging is only supported on Windows or macOS; this Linux environment is for core/UI tests.");
    process.exit(2);
  }
}
const failures = [];
for (const [label, binary, argv] of commands) {
  console.log(`\n=== ${label} ===\n$ ${binary} ${argv.join(" ")}`);
  if (dryRun) continue;
  const result = spawnSync(binary, argv, {
    cwd: root,
    stdio: "inherit",
    shell: process.platform === "win32" && binary === "npm.cmd",
    windowsHide: true,
  });
  if (result.error || result.status !== 0) {
    failures.push({ label, status: result.status, error: result.error?.message });
    console.error(`FAILED: ${label} (${result.error?.message || `exit ${result.status ?? "unknown"}`})`);
  }
}
if (dryRun) {
  console.log(`\nDry-run complete: ${commands.length} commands listed; none executed.`);
} else if (failures.length) {
  console.error(`\n${failures.length}/${commands.length} steps failed or could not start:`);
  for (const failed of failures) {
    console.error(`- ${failed.label}: ${failed.error || `exit ${failed.status ?? "unknown"}`}`);
  }
  console.error("Missing prerequisites? Run npm ci; install Playwright Chromium and the documented native toolchains separately. No verification pass claimed.");
  process.exitCode = 1;
} else {
  console.log(`\nAll ${commands.length} local steps passed. Physical two-computer input acceptance remains separate.`);
}
