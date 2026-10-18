import test from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const script = resolve(dirname(fileURLToPath(import.meta.url)), "verify-local.mjs");
const invoke = (...args) => spawnSync(process.execPath, [script, ...args], { encoding: "utf8" });

test("help lists invocation, prerequisites and leaves the checkout untouched", () => {
  const result = invoke("--help");
  assert.equal(result.status, 0);
  assert.match(result.stdout, /--dry-run/);
  assert.match(result.stdout, /--native/);
  assert.doesNotMatch(result.stdout, /All \d+ local steps passed/);
});

test("dry run lists every standard verification step without executing Cargo", () => {
  const result = invoke("--dry-run");
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /cargo fmt --all -- --check/);
  assert.match(result.stdout, /cargo test --locked/);
  assert.match(result.stdout, /run typecheck/);
  assert.match(result.stdout, /run test:ui/);
  assert.match(result.stdout, /7 commands listed; none executed/);
  assert.doesNotMatch(result.stdout, /All \d+ local steps passed/);
});

test("unexpected options fail closed without running any verification step", () => {
  const result = invoke("--unknown");
  assert.equal(result.status, 2);
  assert.match(result.stderr, /Unknown argument/);
  assert.doesNotMatch(result.stdout, /Rust formatting/);
});

test("native packaging refuses unsupported platforms", () => {
  if (process.platform !== "linux") return;
  const result = invoke("--native", "--dry-run");
  assert.equal(result.status, 2);
  assert.match(result.stderr, /only supported on Windows or macOS/);
  assert.doesNotMatch(result.stdout, /Dry-run complete/);
});

test("multiple dry-run options are idempotent", () => {
  const result = invoke("--dry-run", "--dry-run");
  assert.equal(result.status, 0);
  assert.match(result.stdout, /none executed/);
});
