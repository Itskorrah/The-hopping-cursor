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

## Local continuation — 2026-10-09

Continued the imported history without rebuilding the product. Commits 1091ac5
and a076002 add protected native identities, deterministic reconnect/discovery,
stable display IDs and hotplug reconciliation, permission/session recovery,
bounded bulk work and duplicate-ID consent isolation, explicit native file
clipboard actions and source privacy-marker checks. Details and limitations
are in docs/DEVELOPMENT_STATUS.md; historical counts above describe the cloud
build and should not be read as the current suite.

Current local checks: 42 Mac Rust tests, 10 browser tests, strict desktop/core
Clippy and cross-target adapter checks, production frontend and unsigned native
Mac debug bundle. A bounded AddressSanitizer parser campaign completed 1,770,886
cases in 61 seconds. Native Mac file URLs/privacy flags use isolated pasteboards;
actual Keychain migration preserved the prior public identity. Physical peers,
Finder/Explorer workflows, native permissions and soak/performance remain gaps.

All five jobs in CI run 37880413374 passed on b0344df, including Windows DPAPI
and disposable clipboard checks. Refreshed bundled notices after native dependency
additions; CI now regenerates the inventory and rejects stale notices.

A subsequent Linux fuzz campaign hit the original 512 MiB process RSS cap.
Replayed its saved synthetic input successfully, preserved it as a decoder test
and corpus seed, and retained an explicit 512 MiB single-input replay in CI.
Campaigns use libFuzzer's bounded 2048 MiB default budget without reducing
AddressSanitizer coverage or quarantine. Details are in fuzz/README.md.

The longer local campaign passed 5,299,006 inputs in 181 seconds. Mac CI then
exposed an empty file-URL read-back on both ARM runner jobs. Added explicit
AppKit linkage and file-URL-only read options; both jobs now pass twenty isolated
clipboard repetitions plus the full suite. BUG-007 records the evidence and
remaining uncertainty about the original empty read. The latest local app
launch appears to await a native macOS security approval.
