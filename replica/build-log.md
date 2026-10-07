# Implementation record — 2026-10-07

S01–S06 implemented as an original Tauri desktop interface with explicit,
separate browser preview data. Pairing, layout, profile management, transfers,
preferences, diagnostics, per-monitor hop shortcuts, and per-peer permissions
call the shared Rust engine.
Core, native adapters, and interface are independently written.

The Linux execution environment supports actual encrypted TCP integration
tests and Chromium UI tests. Windows, Apple Silicon, and Intel Mac engine
compile checks passed. Full desktop cross-compilation here lacks native
resource tooling/Apple SDK. GitHub Actions run 37643330418 successfully built
Windows x64 NSIS, Apple Silicon DMG, and Intel Mac DMG installers on native
runners. Its core formatting check exposed a post-edit formatting issue,
corrected before the final update; final CI is recorded through the repository
Actions history.

Local validation: 28 Rust tests, strict Clippy, formatting, TypeScript/Vite build,
and all 10 Playwright browser tests passed. Security regressions include
tampering/replay, fragmented reads under outbound traffic, and full-queue input
recovery. Native callbacks use published routing snapshots without waiting for
disk configuration writes; a dedicated regression holds that lock during
capture. Encryption buffers are reused. A separate-process CLI smoke test
delivered and verified a 256 KiB file. Dependency audits found no listed vulnerability entries; upstream
maintenance/GLib warnings are documented in docs/SECURITY.md.

Important intentional differences: no account/cloud membership; explicit
out-of-band safety-code authentication; file approval and quota by default;
independent input/bulk connections; no fabricated performance numbers.

No production-ready/better-than-original claim is made. Physical native input,
native cross-screen OS file dragging, power/dimming, and signing remain gaps.
