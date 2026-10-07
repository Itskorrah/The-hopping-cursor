# Implementation record — 2026-10-07

S01–S06 implemented as an original Tauri desktop interface with explicit,
separate browser preview data. Pairing, layout, profile management, transfers,
preferences, diagnostics, and per-peer permissions call the shared Rust engine.
Core, native adapters, and interface are independently written.

The Linux execution environment supports actual encrypted TCP integration
tests and Chromium UI tests. Windows and Apple Silicon engine compile checks
passed. Full desktop cross-compilation here is blocked by missing native
resource tooling/Apple SDK, so native GitHub Actions builds are required.

Important intentional differences: no account/cloud membership; explicit
out-of-band safety-code authentication; file approval and quota by default;
independent input/bulk connections; no fabricated performance numbers.

No production-ready/better-than-original claim is made. Physical native input,
native cross-screen OS file dragging, power/dimming, and signing remain gaps.
