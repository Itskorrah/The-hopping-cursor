# Test evidence and remaining plan

Automated core checks: key mapping round trips, physical scan positions,
offset/hidden topology, invalid layouts, safe filenames, actual two-service
encrypted pairing and trust, acceptance/refusal, duplicate files and byte
integrity, clipboard exchange, held-key handoff, emergency release, disconnect,
revocation, quotas, and diagnostics exclusion. The suite currently contains
33 Rust tests on macOS: 17 unit/security tests and 16 encrypted-network integration tests.
Loopback services disable multicast; a separate opt-in two-service mDNS test
checks untrusted discovery and rename refresh. Linux omits two OS-store migration tests.
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
