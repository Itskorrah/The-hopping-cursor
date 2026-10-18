# Windows → Mac shortcut bridge: physical acceptance

**Status: not yet accepted on real paired hardware.** Existing UI/browser tests
cannot prove Windows' app switcher is suppressed or macOS' switcher activates.

## Traced path and chosen approach

The existing Windows `WH_KEYBOARD_LL` hook forwards HID input via the service
only when the active encrypted ownership lease points at a paired Mac. The
hook returns non-zero after the service consumes an event; a separate Windows
keyboard hook is unnecessary. Previous modifier translation only exchanged
Control/Command, not Alt/Tab. The new stateful route converts an already-held
Left Option to Command at Left Alt+Tab, carries the Command modifier over
repeated Tab, and balances it on Alt-up. An unrelated key switches back to
Option. Right Alt/AltGr and Control+Alt combinations are excluded from the **new
shortcut bridge**. This does not prove international-character compatibility:
some Windows AltGr layouts synthesise Left Ctrl+Right Alt, and the pre-existing
global Ctrl-to-Command mapping may still affect characters. Keep that defect
in the open native acceptance queue. Do not assert all AltGr input is unaffected.

Reference design checks: Microsoft's `LowLevelKeyboardProc` specification
requires fast hook return, allows non-zero consumption on normal desktop
events and warns that timed-out hooks can be silently removed. Apple's Mac
keyboard shortcuts specification defines Command+Tab. The encrypted control
lease and existing urgent Leave/ReleaseAll recovery remain authoritative.
Evolve's evidence-first workflow and pstack's investigation/bug-fix principles
guided a constrained change; no unreviewed helper scripts were executed.

## Physical Windows + Mac test matrix

1. On each computer install a build of this PR. Pair, compare the safety
   code, grant only normal macOS Accessibility/Input Monitoring permissions,
   and use the same private LAN. Keep a separate recovery keyboard/mouse.
2. With the Windows pointer controlling the Mac and both mapping switches
   enabled, press **Left Alt+Tab**. Confirm the **Mac app switcher appears**,
   Windows app switcher does **not** appear, and the Mac foreground app changes.
3. Hold Left Alt and press Tab 5+ times; add Left Shift for reverse cycling.
   Release Tab then Alt; check Command, Shift and Option are not stuck.
4. Return input to Windows and press Alt+Tab: Windows must behave normally.
   Repeat on a Mac input session with the **global** toggle off and a separate
   run with that peer's toggle off. Verify overrides are isolated per peer.
5. Type international characters using **Right Alt/AltGr**, Ctrl+Alt, dead
   keys and IMEs. Test explicit Mac Option shortcuts and accidental modifier
   repeats, different scan codes and multi-keyboard layouts.
6. Press a non-Tab key while Alt+Tab is held, followed by Alt release. Test
   **Ctrl+Alt+Esc** recovery, Pause, Quit, edge handback and peer revocation
   mid-chord; Mac must release injected modifiers in every case.
7. Interrupt Wi-Fi/Ethernet during held Alt/Command; test disconnect, sleep,
   wake, OS session lock/unlock, Windows UAC/security transitions and revoked
   Mac input access. Never attempt secure-attention interception/bypass.
8. Repeat on Intel and Apple Silicon Macs if available; test multi-monitor
   DPI/arrangements, mouse scroll, clipboard text/HTML/PNG, user-approved file
   copying, collisions and cancellations in separate checks.
9. Complete a **15+ minute soak** of normal remote typing/hopping/cycling.
   Record OS/build/commit, timestamped observations and anonymous hook timing/
   count diagnostics. Do not log keystrokes, apps, clipboard or file contents.

## Test commands and evidence limits

```sh
cargo fmt --all -- --check
cargo clippy --locked -p hop-core -p hopping-cursor --all-targets -- -D warnings
cargo test --locked
npm ci
npm run build
npm run test:ui
```

Also run native Windows and Mac builds on dedicated runners. Native Windows
`WM_SYSKEY*` interception must be checked with a physical keyboard; synthetic
hook packets cannot substitute. Real `CGEventPost` and cursor visibility
must be checked on macOS. Distinguish runner scheduling failures (zero steps),
test failures, browser-preview assertions and physical user acceptance.

This change does not bypass protected keyboard sequences or alter OS security
settings. The Mac background cursor hide currently uses a version-sensitive
undocumented WindowServer facility and requires independent retesting.
