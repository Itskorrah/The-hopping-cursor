# Test evidence and remaining plan

Latest Windows-native 0.1.3 pass: **59 core tests + one separately run native
pointer probe + 16 browser tests pass**. Both Mac core targets check; physical
Mac runtime/feel acceptance remains pending. See [TEST_0.1.3](../docs/TEST_0.1.3.md)
for the concrete new regressions, baseline failure and toolchain. The historical
0.1.2 evidence below is not a claim that Mac 0.1.3 was executed.

Automated core checks: key mapping round trips, physical scan positions,
offset/hidden topology, invalid layouts, safe filenames, actual two-service
encrypted pairing and trust, acceptance/refusal, duplicate files and byte
integrity, clipboard exchange, held-key handoff, emergency release, disconnect,
revocation, quotas, and diagnostics exclusion. The suite currently contains
55 Rust tests on macOS: 21 unit/security/native tests and 34 encrypted-network integration tests.
Loopback services disable multicast; a separate opt-in two-service mDNS test
checks untrusted discovery and rename refresh. Linux omits the two native pasteboard and two native-store migration tests
(counts differ by platform; record native results). Windows also includes a DPAPI
round-trip/tamper/non-overwrite test and an explicitly opted-in disposable-runner
CF_HDROP test; it never replaces a developer clipboard during default tests.
Additional regressions reject ciphertext tampering/replay, malformed/trailing
messages, preserve fragmented reads during outbound traffic, bypass saturated
queues for recovery, keep native callbacks responsive while configuration is
locked, consume hop chords, cancel pending offers on pause, and restore saved
computers/file sending after restart without waiting for rediscovery.

Automated browser checks: preview distinction, persistent pause/preferences,
mandatory pairing comparison, keyboard arrangement overlap errors, hidden
display persistence, five-page axe accessibility checks, and 390px viewport.
Console errors and 5xx responses fail tests. Profiles and permission controls
also receive direct regression coverage, along with monitor shortcut controls.
All 14 browser tests passed locally, including axe checks across all five pages.

Remaining physical acceptance beyond the user-reported basic two-way handoff: held-input/cursor recovery, UIPI, clipboard
HTML/PNG interoperability, mixed DPI, OS window drag/drop, tray behavior,
sleep/wake/hotplug, long soak tests, end-to-end latency/throughput. See
`docs/PLATFORM_TESTING.md` for actionable steps and pass criteria.

The corrected local Mac bundle passes strict recursive signature validation.
Native diagnostics confirm capture/injection/clipboard permission availability;
Pause/Resume and Bring input here pass IPC smoke checks with zero paired peers.
These do not establish physical cursor/key delivery or emergency-chord behavior.
The package guard rejects the earlier invalid Mac DMG and detaches on failure.

2026-10-09 regressions cover verified-address preservation, trusted-only
reconnect candidates, simultaneous control connections followed by verified file
transfer, inbound address persistence, hotplug release/peer metadata refresh,
profile reconciliation, and identity-migration success/failure. The native Mac
app launched and its real Keychain migration preserved the public identity.

Additional regressions cover copied-file consent, verified local publication,
bulk pressure with live control heartbeats, duplicate-ID consent isolation,
shutdown cleanup, protected-session suspension,
wake/timing-gap ownership cleanup, and all-displays-lost recovery. Two native Mac
checks use isolated pasteboards for privacy flags and Unicode file identities.
The longer bounded AddressSanitizer parser run executed 5,299,006 cases in 181
seconds without a crash/assertion (549 MiB peak RSS); see fuzz/README.md for
reproduction and the earlier Linux resource-cap failure. Mac CI also repeats
both isolated pasteboard checks twenty times per job after BUG-007.
This does not establish native injection, cross-app file paste, OS lock/wake
synchronization, or exhaustive protocol security.

2026-10-10 Windows handoff: user reports successful basic 0.1.2 physical control
in both directions, with Mac-source movement/typing smooth. BUG-018 (E/emoji),
BUG-019 (Windows-source motion) and BUG-020 (initial readiness/recovery) remain
open; no new fixes or native benchmark were run during documentation/merge work.
The README screenshot is a real native app window with the PC connected, visually
checked for full-height sidebar. See docs/WINDOWS_HANDOFF.md for reproduction and
native test priorities; this checkpoint is an owner-authorized alpha merge.

## 0.1.4 keyboard handoff acceptance

- Automated: native tagged synthetic delivery with null/non-text/edit/null focus; independent keyboard delivery during mouse-thread pressure; atomic entry publication; 32 immediate first-key and 32 shared-keyboard handoffs; held Shift/new key order; aggregate diagnostics rendering/accessibility.
- Physical pending: same Mac editor field with Windows desktop/non-text/Notepad/Chrome as source; immediate first letter and modifiers in both directions; no source duplicate typing; pause/reconnect/emergency cleanup. See docs/TEST_0.1.4.md.
- Diagnostics contain counts and source focus/caret booleans, no typed text, key identities, app names or event history. Native synthetic input does not establish physical Mac behavior.


## 0.1.5 scroll and sustained reliability

- Portable: fractional/roundoff/release/direction conversion; first wheel after
  lease entry in both directions; exact mixed scroll/key/button order and no
  source scroll; denied/invalid input counters and content-free export; queue
  coalescing/barriers/pressure/wait statistics.
- Native Windows: signed vertical/horizontal sub-notch hook packets and injected
  duplicate rejection, plus existing focus/pressure/parking probes run serially.
- Synthetic stress: timed release-build encrypted loopback with alternating
  handoffs, held keys/buttons, 1 MiB transfers and repeated connection recovery;
  aggregate timing and final stage parity; opt-in bounded parser mutations.
- Mac: typed ABI, fractional constructor and event-filter tests compile for both
  architectures; native execution and actual two-computer delivery remain pending.
- Physical: Natural on/off; immediate first wheel, rapid/reversed/small scroll,
  multiple windows and displays, trackpad return direction, clipboard/file load,
  pause/reconnect/emergency and multi-hour use. See docs/TEST_0.1.5.md.

## PR #3 active Windows source → Mac shortcuts — 2026-10-26

- F02-H-PR3: Left Alt+Tab; held repeat; Shift reverse; balanced modifier release;
  source stays Windows on return; global and per-Mac switches persist.
- F02-E-PR3: unrelated-key interruption, AltGr/IME, full normal queue and urgent
  handback, Pause/Quit/disconnect/revoke, display/session interruption.
- Automated scope: source routing/unit regressions, encrypted integration,
  preview settings/accessibility and the full-queue emergency-capture regression.
- Physical scope remains pending: Windows shell interception, Mac CGEvent
  delivery/visibility, real layouts and a 15-minute paired trial. Idle SharedKey
  translation is a documented limitation, not a completed universal mode.
- Commands/results: [TEST_PR3.md](../docs/TEST_PR3.md).
  User/Mac order: [PR3_HANDOVER.md](../docs/PR3_HANDOVER.md).
