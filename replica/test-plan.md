# Test evidence and remaining plan

Automated core checks: key mapping round trips, physical scan positions,
offset/hidden topology, invalid layouts, safe filenames, actual two-service
encrypted pairing and trust, acceptance/refusal, duplicate files and byte
integrity, clipboard exchange, held-key handoff, emergency release, disconnect,
revocation, quotas, and diagnostics exclusion. The suite currently contains
42 Rust tests on macOS: 19 unit/security/native tests and 23 encrypted-network integration tests.
Loopback services disable multicast; a separate opt-in two-service mDNS test
checks untrusted discovery and rename refresh. Linux omits the two native pasteboard and two native-store migration tests
(38 tests total). Windows also includes a DPAPI
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
All 10 browser tests passed locally, including axe checks across all five pages.

Not run on real hardware: native hooks, UIPI, macOS permissions/taps, clipboard
HTML/PNG interoperability, mixed DPI, OS window drag/drop, tray behavior,
sleep/wake/hotplug, long soak tests, end-to-end latency/throughput. See
`docs/PLATFORM_TESTING.md` for actionable steps and pass criteria.

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
The bounded AddressSanitizer parser run executed 1,770,886 cases in 61 seconds
without a crash/assertion (353 MiB peak RSS); see fuzz/README.md for reproduction.
This does not establish native injection, cross-app file paste, OS lock/wake
synchronization, or exhaustive protocol security.
