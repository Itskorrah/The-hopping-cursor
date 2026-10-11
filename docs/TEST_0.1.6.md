# Mac source cursor handoff — 0.1.6, 2026-10-11

The owner reports that the Mac pointer sometimes remains visible at the screen
edge while the trackpad controls Windows. This update addresses two demonstrated
local weaknesses. Physical single-cursor acceptance remains pending.

## Correction

Public Quartz cursor hiding normally requires the foreground application.
The old adapter called it while running behind other applications and treated a
successful return as sufficient. It also recorded the parking point immediately
after posting a synthetic mouse move, which can be processed later. The recorded
point could therefore remain at the boundary.

The adapter now enables `SetsCursorInBackground` on **its own** WindowServer
connection while it owns a hide. Symbols are resolved dynamically, setting/read
errors are checked, and the enabled property is read back before hiding. This is
an **undocumented macOS facility**, not a supported public API guarantee. If it
is absent or incompatible, forwarding fails and the engine returns control
locally. Retest on future macOS versions; App Store compatibility is not claimed.

The cursor is warped synchronously into the current display's interior, without
posting a synthetic event. The adapter retains the original position, owns one
balanced hide/show pair, and restores the previous background capability on
release, stop and destruction. Repeated enable/disable calls do not add hides.
A failed show retains the owned hide for a later recovery attempt. A failed
parking warp triggers local input release. Ordinary edge and explicit jump paths
skip queued parking moves only when the adapter owns this operation; the Windows
fallback remains intact.

Injection markers, pressed-key cleanup, pairing, encryption, control/bulk
separation and priority emergency release are retained. No typed input, clipboard
content, files or private identities are logged. No OS security settings or
scroll preferences are changed.

References: [Apple cursor control and balanced hide/show](https://developer.apple.com/library/archive/documentation/GraphicsImaging/Conceptual/QuartzDisplayServicesConceptual/Articles/MouseCursor.html),
[synchronous cursor warp](https://developer.apple.com/documentation/coregraphics/cgwarpmousecursorposition(_:)),
[WebKit's system SPI ABI declarations](https://github.com/WebKit/WebKit/blob/main/Source/WebCore/PAL/pal/spi/cg/CoreGraphicsSPI.h).

## Regression evidence

- A background native probe failed before the change because the required
  WindowServer property stayed disabled. It passes with the change, including
  repeated/idempotent handoffs, stop, destruction and preservation of an
  already-enabled capability owned by another component in the process.
- A separate native probe failed before the change because forwarding did not
  synchronously park at the display center. It now reads the actual parked
  position and verifies restoration without posting an event.
- The engine regression covers edge and explicit jump entry with both native
  parking and the legacy fallback. It caught the remaining queued move in the
  jump path; all four cases pass after correction. The remote Enter barrier still
  precedes native forwarding.

The two native probes are opt-in and run serially with the installed sharing app
quit. They briefly hide/warp the cursor and restore it; they never post typing or
clicks. They verify capability and position/lifecycle behavior, **not a visual
physical crossing**. `CGCursorIsVisible` is deprecated/unsupported and is not
used as proof. Workspace's ownership-derived visibility status is also not a
measurement of the OS pointer.

```sh
cargo test --locked -- --test-threads=1
# Quit the sharing app before the following real-cursor probes:
cargo test --locked -p hop-core --lib platform::macos::tests::mac_ -- --ignored --test-threads=1
cargo clippy --locked -p hop-core -p hopping-cursor -p hopping-cursor-desktop --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
npm run build
npm run test:ui
```

Default suite: **75 Rust tests passed** (36 unit/native API checks and 39
encrypted integration tests). The two separate native cursor probes and all
**18 browser tests** pass. Default ignored tests include cursor probes, real
mDNS and the timed soak; those are not silently counted as passing.

| Check | Observed result |
| --- | --- |
| Native host | Apple Silicon, macOS 26.5. Both cursor probes ran on this host; no Intel/Windows runtime result is implied. |
| Static checks | Formatting and strict all-target/all-feature Clippy pass for native ARM engine/CLI/desktop and cross-compiled Intel Mac/Windows engine/CLI. |
| Build and integrity | Frontend production build and signed Mac debug bundle pass. Finished and installed bundles pass recursive strict signature verification; designated requirement matches the previous installed app. Not a notarized public release. |
| Native startup/IPC | Installed 0.1.6 opens with capture/injection Available and the paired Windows PC automatically connected. Jump to its primary display reports PC ownership; Bring input here returns local ownership without an error. These are control-path checks, not visual pointer or physical key delivery measurements. |
| Preserved state | Every saved configuration field matches its pre-update fingerprint, including protected identity reference, pairing, settings, layout, profiles and shortcuts. The prior 0.1.5 app is retained as a local rollback backup. |
| Packaging notices | Generated dependency/font notices match the repository. The host's default Python 3.9 lacks the writer's newline option; the check passes with bundled Python 3.12 and the project toolchain activated. |

Hosted CI is separate from these local results; no successful hosted run is
claimed for this update. The earlier Windows tests/soak in
[TEST_0.1.5.md](TEST_0.1.5.md) remain historical evidence.

## Physical acceptance

Use Mac **0.1.6** with Windows **0.1.5**. Protocol 2 is unchanged; this Mac-only
correction needs no PC reinstall, pairing reset or layout reset.

1. Resume both apps and keep local controllers available. Start with harmless
   editors and the same saved Mac → Windows primary → Windows secondary layout.
2. With a different Mac app in front, cross Mac → Windows at least 20 times,
   returning across the edge. Repeat with the Hopping Cursor window in front
   and closed to the tray. Only the destination pointer should be visible;
   source clicks and typing must stay suppressed.
3. Test explicit **Jump here**, modifier/button-held crossings, movement between
   the two Windows displays, and Windows → Mac controller takeover. No stranded
   cursor or stuck held input should remain.
4. While Windows owns input, test **Bring input here**, Pause and source
   **Ctrl+Alt+Esc**. Each must restore the Mac pointer and local control. Test
   peer disconnect and app Quit with a local controller available.
5. Repeat with fullscreen apps, sleep/wake, a permission change and display
   hot-plug. These wider platform cases remain required acceptance work.

Report the direction, foreground app, return/recovery action and whether a
pointer remained visible. Existing diagnostics provide aggregate stages only;
never add raw event/content logging to investigate this issue.
