# Windows experience pass — 0.1.3, 2026-10-10

Development alpha, based on main `5be6253`. Windows-native engineering and
testing; physical Mac receiver and two-computer acceptance remain open.

## Changes and evidence

| Area | Change | Evidence and limits |
| --- | --- | --- |
| BUG-018 | Rebuild Mac modifiers from authenticated held HID keys; preserve Caps Lock/keypad classification, reject inherited Fn/Globe and receiver modifiers. | Portable synthetic flag regression passes; both Mac targets check. The user's physical E/emoji case still needs the updated Mac receiver. |
| BUG-019 | While remote, park Windows source pointer inside its monitor and forward Windows-processed screen displacement; Raw Input still detects local outward edge motion. | Actual Windows hooks, parking/restoration and synthetic-hook reclaim probes pass. Mouse gain, feel, precision, source cursor visibility and real remote drags require hardware acceptance. No latency benchmark claimed. |
| Motion pressure | Replace only consecutive pending absolute positions within one ownership epoch. Keys, buttons, Enter and focus messages are barriers; the normal queue stays bounded. Priority release/epoch invalidation remain independent. | 16,000 queued samples preserve final positions and key order. An 8,000-event encrypted integration regression fails on the original 0.1.2 queue and passes here, preserving clicks, typing and ownership. |
| BUG-020 | Track whether capture actually started; refresh cannot turn a failed hook installation into ready input. Expose capture/display readiness. UI identifies pause/session/permissions/capture/displays/connection/input grants/lock/game mode/shared edges. Serialize periodic polling and discard older responses. Add a full Quit action in the window. | Failed-capture/restart regression and browser blocker cases pass. The installed 0.1.2 app was independently observed saying Workspace ready with zero connected computers. The user described general friction, not a reproducible Connect/Pause sequence. |
| Windows setup | Portable LLVM/MSVC SDK activation, no machine installation. UTF-8 notice generation fixes a native Windows cp1252 failure. Isolated protocol tests listen only on loopback. | Native Rust/core/desktop checks and NSIS build pass. Notice generation is identical to the checked-in inventory after fixing encoding. Production still listens on LAN interfaces. |

## Verification

On Windows 10 Pro x64, Node 24.1.0, Rust 1.99.0 MSVC, LLVM 22.1.8,
xwin 0.10.0, CRT 14.44.17.14, Windows SDK 10.0.26100 and installed WebView2:

- `cargo test --locked -p hop-core -p hopping-cursor`: **59 passed** (23 unit/security + 36 encrypted integration).
- Four default ignores: destructive real clipboard test, LAN multicast discovery,
  and two interactive native probes. Both native probes were run separately:
  **2 passed**, real hooks installed, parked monitor center and original position restored.
  A directly supplied synthetic native-hook packet also verifies source-coordinate
  reclaim and its first displacement; it does not measure real mouse gain.
  That reclaim probe caught a late visibility update warping the pointer back;
  removing the redundant visibility-path warp fixes it. Ten repeated probes pass.
- Strict Clippy for core/CLI and desktop, all targets: **passed**.
- `cargo fmt --all -- --check`: **passed**.
- `cargo check --locked -p hop-core --target aarch64-apple-darwin` and
  `--target x86_64-apple-darwin`: **passed**. These are compile checks, not Mac execution.
- Frontend production build and **16 Playwright tests passed**, including
  all-page axe checks, runtime errors and 5xx checks in the workspace suite.
- Native Windows x64 NSIS installer build: **passed**, version resources 0.1.3.
  LLVM reports omitted Microsoft PDB debug symbols; release links successfully.
  Cargo 1.99 also warns about the pre-existing desktop lib/bin output naming.
  Neither is a product runtime verification claim.
- Official LLVM archive SHA-256 and xwin released archive checksum verified.
- A separate archived 0.1.2 checkout with only the new motion-burst test fails
  when its original 512-entry queue saturates. The delivered source passes it.

The Windows installer was installed successfully with exit code 0. Native
accessibility text confirms v0.1.3, the saved three-screen workspace and a
connection-needed status with the Mac offline. Screenshot capture failed with
`window capture timed out: timed out waiting on channel` after a fresh-handle
retry; native UI click/visual verification is incomplete.

Logs, baseline reproduction and tool downloads live in ignored `.local/`.
They contain synthetic test traffic, build output and aggregate results; no
physical key contents, clipboard contents or protected identities were logged.

## Remaining physical acceptance

1. Update the Mac from main with its existing signing configuration; preserve
   identities/settings. Both sides should show 0.1.3 for this acceptance pass.
2. Compare full pairing codes; existing approved pairing should reconnect. Review
   and apply any deliberately changed arrangement on the receiving computer.
3. In a blank editor on Mac, type E and ordinary letters from Windows, test both
   modifier mapping settings, left/right modifiers and shortcuts, then reverse direction.
4. Compare slow precise motion, rapid sweeps, diagonals, scroll and held drags
   in both directions across all three displays and different scaling settings.
   Confirm the source cursor stays hidden and clicks never affect both machines.
5. Ctrl+Alt+Escape, Pause/Resume, physical takeover, disconnect, restart in either
   order, sleep/wake and hotplug must release held keys/buttons and recover.
6. Check native text/HTML/PNG and opt-in file clipboard behavior with harmless
   files. Native Finder/Explorer cross-edge file dragging is still unimplemented.

No new physical Mac result, absolute latency number, production signing,
secure-desktop control or complete cross-platform premium acceptance is claimed.
