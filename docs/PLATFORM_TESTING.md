# Windows/macOS acceptance checklist

Use two computers on a private test network. Start with harmless documents and
text. Keep each local mouse available and learn Ctrl+Alt+Escape before testing.
An installer build is not evidence that input behavior has passed this checklist.

1. Install/open on Windows 10+ and macOS 12+, Intel/Apple Silicon as available.
   Grant macOS Accessibility and Input Monitoring; allow Local Network access.
   Restart after permission changes. Allow TCP 24810 and mDNS UDP 5353 on the
   private firewall profile. Do not run two app/CLI engines on the same port.
   Verify the Mac app bundle with `codesign --verify --deep --strict --verbose=2`.
   Ad hoc builds may need their permission entry refreshed after rebuilding;
   confirm capture/injection in Diagnostics rather than relying on switches alone.
   Fully quit with ⌘Q before reopening. Check locked Keychain recovery without
   deleting identities or weakening their access controls.
2. Confirm discovery. Test local IP connection when multicast is blocked.
   Compare complete safety codes on both screens, approve both, confirm connected.
   An unapproved device must never move the cursor, paste, or save a file.
3. Arrange side-by-side, vertically, and with offsets. Cross only shared edges.
   Test every local display, multiple remote displays, Retina and Windows scaling.
   Hidden displays must not become hopping destinations. Hotplug refresh polls every
   two seconds and releases input; test undocking/all-displays-missing and restored
   monitor IDs without restarting. Verify saved layouts survive temporary loss.
   Save/load named profiles and test Ctrl+Alt+1–9 monitor bindings, including
   offline/hidden destinations and key-up cleanup after a hop chord.
4. Verify typing, arrows, function keys, left/right modifiers, international
   layouts, scroll direction, mouse buttons, dragging text, and held shortcuts
   across boundaries. Test Command/Control translation on and off. Vendor/media
   keys and AltGr layouts need additional coverage.
5. Pull Ethernet/disable Wi-Fi while controlling the other computer. Both sides
   must release synthetic held keys; source input must return within the watchdog
   window. Repeat during a held click and held modifier. Test emergency chord,
   source sleep, target sleep, app Quit, and receiver local-input takeover. Test Mac
   Secure Keyboard Entry and Windows lock/UAC transitions; sharing must suspend,
   held input must release, and paired connections should recover after return.
   This does not grant control of Windows secure desktop or synchronize locks.
6. Copy plain text, Unicode, HTML from a rich editor, and images in both
   directions. Disable each sharing type and check it stops. Clipboard data above
   8 MiB serialized size must be skipped. Confirm no repeated bounce/pasteboard
   changes while idle. Use synthetic content marked concealed/transient/generated
   on Mac or excluded from Windows history/cloud sharing: it must not forward.
   Do not use real credentials for these tests; unmarked passwords are ordinary text.
7. Send/drop files into this app. Accept and decline, test empty files, large
   files, duplicates, slow network, quota refusal, disk full, cancellation, and
   disconnect. Confirm exact bytes, unique names, and partial cleanup. Test a
   filesystem without hard links to ensure a clear error rather than data loss.
   Pause and revoke file permission while awaiting acceptance and during a large
   transfer: outstanding offers must expire and incomplete files must disappear.
   Copy regular files in Finder/Explorer, select a paired recipient in Transfers,
   and click **Send copied files**. On receipt, use **Copy file for pasting** and
   paste into another folder. Test Unicode, spaces, duplicate copies, moved
   received files, clipboard contention, per-peer file/clipboard denial, and
   consent cancellation. Folders and virtual files must fail clearly.
8. Unpair while controlling/transferring; permission must be revoked. Restart
   both computers and verify identities, preferences, and layouts persist.
9. Verify tray Open, Pause/Resume, Bring input here, and Quit. Closing the main
   window leaves the tray engine running; Quit stops it.
10. Measure end-to-end cursor latency using a high-speed camera, and file
    throughput from bytes/time. Report hardware, network, sample size, p50/p95,
    and packet-loss conditions. Heartbeat RTT is a different measurement.

Still out of scope: UAC/Windows login/secure desktop input, native Finder/Explorer
file dragging across monitor edges, automatic file-clipboard synchronization, dimming, session
lock/wake synchronization, relative/raw gaming input, and system-wide media keys. Explicit file-list clipboard
bridging is implemented but still needs the cross-application checks above.

## Three-screen regression retest (0.1.1)

Update both computers, keeping their identities/settings. On both workspaces,
use OS display order to repair reversed enumeration, then arrange Mac → Windows
primary → Windows secondary and save. Confirm the Primary label matches the
middle monitor. Layouts are independent. Open harmless blank editors on both.

The [0.1.1 Windows test ZIP](https://github.com/Itskorrah/The-hopping-cursor/releases/tag/v0.1.1-test.1)
contains the new installer. Fully quit the previous PC app from its tray menu,
install, reopen and confirm v0.1.1. This package was cross-built on macOS and
has not yet passed actual Windows installation/runtime acceptance. The locally
installed Mac app is already 0.1.1; its permission/IPC checks pass.

- Mac right edge enters the middle PC display; continue to the right PC display.
- While controlling the PC, Mac pointer stays hidden/parked and clicks, scroll,
  typing and shortcuts do not affect Mac applications. Check the aggregate
  locally suppressed-event count increases without recording content.
- Windows primary left edge enters the Mac; remote clicks and keys affect the
  Mac alone. Physical movement between the PC's own monitors does not wrap.
- Hold a harmless drag or modifier across both PC monitors; it must remain held
  until physically released. Test both screen-edge return and Ctrl+Alt+Esc.
- Pause, disconnect, revoke permissions and quit: source cursor must become
  visible again and held remote input must release.

The tests model portable routing and directly exercise the Mac filter using
constructed events that are never posted. They do not establish physical hook,
background cursor visibility or foreground-application suppression behavior.
