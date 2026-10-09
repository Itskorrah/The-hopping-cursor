# Development evidence — 2026-10-09

The project remains a development alpha. This pass continues the existing Rust/
Tauri implementation and preserves peer identities, explicit approval, pinned
keys, encrypted transport, separate bulk traffic, and emergency release.

## Implemented in this pass

- Reconnect only paired identities; verify the expected key identity before
  exchanging introductions. Prefer remembered endpoints; keep bounded mDNS
  hints separate from authenticated metadata. Persist inbound listening ports.
- Resolve simultaneous connections consistently; stale connection tasks cannot
  disconnect their replacement. Cancellation also covers reconnect attempts.
- Isolate parallel loopback tests from host multicast. A dedicated real-mDNS
  test exercises discovery and rename without automatically granting trust.
- Refresh display lists every two seconds, release input on change, reconcile
  active layouts and profile application, and reconnect with fresh metadata.
- Use Mac display UUIDs and Windows monitor interface paths when available.
  Migrate current local legacy display references, profiles, and shortcuts.
  Virtual displays retain a fallback identifier. Hardware/driver changes can
  still affect identity; identical monitors and docks require physical tests.
- Refresh discovery advertisements on rename. Open the existing desktop window
  on second launch/Dock reopen. Add an explicit native permission-request/retry
  action; the engine never silently requests permission in the background.
- Migrate native identity keys to macOS Keychain or user-bound Windows DPAPI.
  Verify read-back before removing the key from JSON. Failed migration preserves
  the original; inaccessible protected keys fail closed without rotation.
- Fix current stable Rust's macOS clipboard lint; check native Clippy in CI.

## Verified locally

- 33 Rust tests pass in parallel on Apple Silicon macOS: 17 unit/security and
  16 real encrypted TCP integration tests with an in-memory OS adapter.
- The separate two-service multicast discovery/rename test passes on this LAN.
- Strict Clippy passes for Apple Silicon, Intel Mac, and Windows x64 core/CLI.
  Cross-compilation verifies types and API declarations; it is not Windows or
  Intel runtime evidence.
- Frontend production build and all 10 Playwright tests pass, including axe on
  five pages and a 390px viewport. These exercise the labelled browser preview.
- Native Mac debug bundle builds and launches with real local display data.
  Actual Keychain migration preserved the preexisting public identity and removed
  the private key from the real JSON config. macOS still reports missing input
  permission; the request button was exercised. Cross-computer input was not.

## Still required

Physical Windows↔Mac/Mac↔Mac/Windows↔Windows input, permission denial/revocation,
held drags, layouts, mixed DPI, pasteboards, reconnect/sleep and multi-hour soak
validation. Native Finder/Explorer edge dragging and file clipboard, dimming,
lock/wake synchronization, raw game input, media/vendor keys, international
layout coverage, and privileged Windows secure desktop support remain unfinished.
Signing/notarization, verified signed updates, measured latency/throughput and
independent security review require additional infrastructure or external work.
Do not infer those capabilities from passing protocol or browser tests.
