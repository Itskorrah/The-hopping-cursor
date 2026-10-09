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
opens with real local display data and zero paired/connected peers; native
diagnostics and permission requests work. Run 37884039823 on a18c1d1 passed all five CI jobs, including
native Intel and ARM Mac suites and twenty isolated pasteboard repetitions per
host, Windows native checks, and all three unsigned installer builds.

Native permission setup then exposed invalid/unsealed Mac application signatures
in both the local debug bundle and earlier installer packages (BUG-008). Explicit
ad hoc signing corrects bundle integrity; the local app now reports capture,
injection and clipboard access Available after both privacy entries were refreshed
and Keychain access completed. Pause/Resume and Bring input here pass native IPC
checks, and the existing public identity is preserved. Packaging now validates
the app inside each finished DMG, with read-only mounting and cleanup on failure;
the guard rejects the earlier invalid package. Developer ID/AuthentiCode signing,
notarization and physical peer control still remain gaps.

- 2026-10-09 / 0.1.1: reproduce monitor enumeration order, Windows local
  wraparound and same-peer held-drag loss with failing tests; correct routing,
  add native source cursor lifecycle/unmapped-key suppression and private Mac
  injection source; label primary displays and add OS-order repair. 47 Rust /
  11 browser checks pass locally, cross-target strict Clippy passes. Physical
  duplicate-click validation and updated Windows installer remain pending.

- 2026-10-09 / 0.1.1 delivery: install the locally Apple Development-signed Mac
  app in the user's Applications folder, refresh its existing grants and verify
  all three native permission checks plus recovery IPC; public identity and PC
  pairing remain intact. Hosted run 37895937871 is blocked with zero steps.
  With owner-approved Microsoft SDK terms, cross-build the Windows MSVC/NSIS
  package locally, check archive/architecture/version and exact bundled bytes
  accounting only for Tauri's NSIS marker, and publish the private test release
  v0.1.1-test.1. Physical duplicate-input and two-way Windows acceptance remain open.

2026-10-09 — 0.1.2: Windows Raw Input motion sink and cursor parking window;
authenticated coordinated cursor focus; idle keyboard routing; physical-controller
reclaim; no echoed Leave to a new controller; ownership announcement after urgent
queue invalidation. Accepted arrangement sharing protects saved local work.
Opt-in native copied-file batches use consented verified streams, prevent bounce
and preserve newer clipboard edits. Diagnostics identify controller/destination;
UI gains arrangement review/file clipboard and native chrome conventions.
55 Rust / 14 browser checks pass; native runtime retest is pending. Native
Finder/Explorer edge drag, folders/file promises, measured latency, signing and
production updates remain unfinished. See docs/TEST_0.1.2.md.

0.1.2 delivery follow-up: startup sampling located the Keychain wait on the UI
thread; protected startup moved to a background thread with fail-closed IPC and
a Quit control. OS file clipboard failure now preserves received files and
reports a usable transfer-page fallback. New transfer and startup-IPC regressions
bring verification to 55 Rust and 14 browser tests.

0.1.2 package checkpoint: signed local Mac app opens with existing identity/peer
and available permissions; native recovery IPC passes and sharing stays paused.
Windows NSIS archive/AMD64/version/extracted bytes verify. Publish private
v0.1.2-test.1 from 049d2ca and verify uploaded/downloaded ZIP hash. Hosted run
37920694980 executes no steps under the existing account restriction. PR #2
remains draft pending the physical Windows→Mac/cursor/file-clipboard retest.

2026-10-10: capture the actual connected native Mac 0.1.2 window for README, with
full-height sidebar and no image editing. Record user-confirmed basic two-way
control plus open E/emoji, Windows-source feel and setup defects. Prepare
docs/WINDOWS_HANDOFF.md and consolidate the owner-authorized alpha checkpoint
into main. No protocol/native/UI implementation or installer changes in this pass.
