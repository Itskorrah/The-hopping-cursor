# Windows/macOS acceptance checklist

Use two computers on a private test network. Start with harmless documents and
text. Keep each local mouse available and learn Ctrl+Alt+Escape before testing.
An installer build is not evidence that input behavior has passed this checklist.

1. Install/open on Windows 10+ and macOS 12+, Intel/Apple Silicon as available.
   Grant macOS Accessibility and Input Monitoring; allow Local Network access.
   Restart after permission changes. Allow TCP 24810 and mDNS UDP 5353 on the
   private firewall profile. Do not run two app/CLI engines on the same port.
2. Confirm discovery. Test local IP connection when multicast is blocked.
   Compare complete safety codes on both screens, approve both, confirm connected.
   An unapproved device must never move the cursor, paste, or save a file.
3. Arrange side-by-side, vertically, and with offsets. Cross only shared edges.
   Test every local display, multiple remote displays, Retina and Windows scaling.
   Hidden displays must not become hopping destinations. Restart after hotplug
   in this first version; stable hardware identification/hotplug refresh is pending.
4. Verify typing, arrows, function keys, left/right modifiers, international
   layouts, scroll direction, mouse buttons, dragging text, and held shortcuts
   across boundaries. Test Command/Control translation on and off. Vendor/media
   keys and AltGr layouts need additional coverage.
5. Pull Ethernet/disable Wi-Fi while controlling the other computer. Both sides
   must release synthetic held keys; source input must return within the watchdog
   window. Repeat during a held click and held modifier. Test emergency chord,
   source sleep, target sleep, app Quit, and receiver local-input takeover.
6. Copy plain text, Unicode, HTML from a rich editor, and images in both
   directions. Disable each sharing type and check it stops. Clipboard data above
   8 MiB serialized size must be skipped. Confirm no repeated bounce/pasteboard
   changes while idle. Do not use real credentials for these tests.
7. Send/drop files into this app. Accept and decline, test empty files, large
   files, duplicates, slow network, quota refusal, disk full, cancellation, and
   disconnect. Confirm exact bytes, unique names, and partial cleanup. Test a
   filesystem without hard links to ensure a clear error rather than data loss.
8. Unpair while controlling/transferring; permission must be revoked. Restart
   both computers and verify identities, preferences, and layouts persist.
9. Verify tray Open, Pause/Resume, Bring input here, and Quit. Closing the main
   window leaves the tray engine running; Quit stops it.
10. Measure end-to-end cursor latency using a high-speed camera, and file
    throughput from bytes/time. Report hardware, network, sample size, p50/p95,
    and packet-loss conditions. Heartbeat RTT is a different measurement.

Still out of scope: UAC/Windows login/secure desktop input, native Finder/Explorer
file dragging across monitor edges, file pasteboard bridging, dimming, session
lock/wake synchronization, relative/raw gaming input, and system-wide media keys.
