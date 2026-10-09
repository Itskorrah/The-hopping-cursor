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
- Add explicit native file clipboard actions: read Finder file URLs / Explorer
  CF_HDROP lists, send through existing consented transfers, and copy only verified
  local received files back for pasting. Folders, file promises, automatic file
  clipboard synchronization, and cross-edge OS dragging remain unimplemented.
- Bound incoming/outgoing bulk work to four transfers each; pending consent no
  longer holds control handshake slots. Failed clipboard sends can retry.
  Duplicate incoming IDs cannot replace an existing consent prompt.
- Suspend sharing during Mac Secure Event Input / unavailable Windows input
  desktop, detect permission loss and Mac tap disablement, and clear stale held
  state after sleep/scheduling/clock gaps. Missing displays release input while
  preserving saved layouts. This does not synchronize OS power or lock state.
- Respect Mac concealed/transient/generated markers and Windows clipboard
  history/cloud exclusion flags; discard clipboard changes during a read.
  Unmarked sensitive content cannot be detected reliably.
- Ensure shutdown marks peers disconnected, rejects new manual connections,
  and aborts reader tasks even when stop precedes task scheduling.
- Add a bounded parser fuzz harness with synthetic seeds and AddressSanitizer.
- Fix current stable Rust's macOS clipboard lint; check native Clippy in CI.

## Verified locally

- 42 Rust tests pass in parallel on Apple Silicon macOS: 19 unit/security/native and
  23 real encrypted TCP integration tests with an in-memory OS adapter.
- Native Mac file-URL round trips and privacy-marker checks pass on isolated
  pasteboards, including Unicode filesystem identities; the user's clipboard
  is not replaced by these tests. Finder/Explorer destination behavior still
  needs cross-application acceptance testing.
- A 61-second AddressSanitizer parser campaign executed 1,770,886 cases without
  a crash/assertion, peak RSS 353 MiB. This is a bounded smoke campaign.
- The separate two-service multicast discovery/rename test passes on this LAN.
- Strict Clippy passes for Apple Silicon, Intel Mac, and Windows x64 core/CLI.
  Cross-compilation verifies types and API declarations; it is not Windows or
  Intel runtime evidence.
- Frontend production build and all 10 Playwright tests pass, including axe on
  five pages and a 390px viewport. These exercise the labelled browser preview.
- Native Mac debug bundle builds and launches with real local display data.
  Actual Keychain migration preserved the preexisting public identity and removed
  the private key from the real JSON config. The last native window check reported
  missing input permission; the request button was exercised. Cross-computer
  input was not tested. The updated bundle builds locally; a later native window
  check was blocked because the Mac was locked.

## Native CI evidence

[Run 37880413374](https://github.com/Itskorrah/The-hopping-cursor/actions/runs/37880413374)
passed on implementation commit b0344df: Linux core/UI, bounded parser fuzzing,
Windows x64 NSIS and Intel/Apple Silicon DMG packages. Windows also exercised
real user-bound DPAPI round-trip/tamper/non-overwrite and a disposable-runner
CF_HDROP clipboard round-trip. These do not prove physical input or Explorer
paste behavior. The package notice inventory was subsequently regenerated for
new native dependencies and is checked by CI before packaging.

## Still required

Physical Windows↔Mac/Mac↔Mac/Windows↔Windows input, permission denial/revocation,
held drags, layouts, mixed DPI, pasteboards, reconnect/sleep and multi-hour soak
validation. Native Finder/Explorer edge dragging, automatic file clipboard
synchronization, folder/file-promise support, dimming,
lock/wake synchronization, raw game input, media/vendor keys, international
layout coverage, and privileged Windows secure desktop support remain unfinished.
Signing/notarization, verified signed updates, measured latency/throughput and
independent security review require additional infrastructure or external work.
Do not infer those capabilities from passing protocol or browser tests.
