# Test evidence and remaining plan

Automated core checks: key mapping round trips, physical scan positions,
offset/hidden topology, invalid layouts, safe filenames, actual two-service
encrypted pairing and trust, acceptance/refusal, duplicate files and byte
integrity, clipboard exchange, held-key handoff, emergency release, disconnect,
revocation, quotas, and diagnostics exclusion. The suite currently contains
26 Rust tests: 13 unit/security tests and 13 encrypted-network integration tests.
Additional regressions reject ciphertext tampering/replay, malformed/trailing
messages, preserve fragmented reads during outbound traffic, bypass saturated
queues for recovery, consume hop chords, and cancel pending offers on pause.

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
