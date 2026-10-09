# Reproduced bugs and fixes — 2026-10-09

### BUG-001: Parallel network suite fails during pairing

- Severity: S2
- Flow: encrypted pairing / reconnect
- Build: 9e825fe; Apple Silicon macOS 26.5, Rust 1.99
- Steps: run `cargo test --locked` with default parallelism.
- Expected: independent TCP tests pair and transfer reliably.
- Actual: reset connections, introduction timeouts, stale addresses after restart.
- Evidence: local baseline and diagnostic verification logs; 0/14 then 1/14 network
  tests passed. Serial execution had passed during setup.
- Fix: tests now isolate multicast; discovered candidates no longer auto-connect,
  hints do not overwrite verified addresses, inbound endpoints persist, and
  simultaneous channels converge. Separate real discovery/rename check passes.
- Status: fixed in 1091ac5 (native CI checks in 8c8de86); parallel suite passes.

### BUG-002: Strict Clippy fails in native Mac clipboard conversion

- Severity: S3
- Steps: strict Clippy on macOS with Rust 1.99.
- Expected: clean native adapter checks.
- Actual: chunks_exact_to_as_chunks lint fails two PNG conversion paths.
- Fix: typed slice chunks; strict native Clippy also added to CI.
- Status: fixed in 1091ac5 (native CI checks in 8c8de86).

### BUG-003: Connected count includes the local computer

- Severity: S3
- Steps: open native Workspace with no paired peers.
- Expected: zero paired and connected computers.
- Actual: summary claims one paired computer.
- Fix: show the actual connected peer count.
- Status: fixed in 1091ac5 (native CI checks in 8c8de86).

### BUG-004: Missing displays leave stale routing eligible

- Severity: S2
- Steps: route to a peer; make the source display backend return no monitors;
  refresh displays and poll permissions.
- Expected: release input and suspend routing while retaining the saved layout.
- Actual: refresh returned an error before releasing ownership.
- Fix: gate input on usable displays independently of permission polling;
  restore routing only after valid displays return. Regression passes.
- Status: fixed in the session/clipboard hardening batch.

### BUG-005: Shutdown retains connected status and permits new connections

- Severity: S2
- Steps: pair two test services, shut one down, inspect its snapshot and attempt
  a manual connection.
- Expected: disconnected status, no new control sessions or live reader tasks.
- Actual: the new regression fails on retained connected status; connection
  admission had no stop check and early task exit bypassed reader cleanup.
- Fix: stop admission, clear peer state and channels, and run task cleanup even
  if shutdown happened before the control task was scheduled.
- Status: fixed in the session/clipboard hardening batch; regression passes.

### BUG-006: Reused transfer ID disrupts pending consent

- Severity: S2
- Steps: pair a peer, send an offer, then send another authenticated data offer
  with the same ID and a different filename while the first awaits approval.
- Expected: reject the duplicate and keep the original offer/decision intact.
- Actual: the regression fails; duplicate state/decision replacement disrupts
  the first transfer and returns a consent response to the duplicate session.
- Fix: reserve incoming IDs atomically for the full operation and reject IDs
  still present in transfer history. Cleanup releases reservations on every exit.
- Status: fixed in the session/clipboard hardening batch; original transfer
  completes byte-for-byte after duplicate rejection in the regression.

### BUG-007: Mac file clipboard round-trip returns no files in CI

- Severity: S2
- Flow: explicit native file clipboard bridge
- Build: 6dcbb2a; both macOS 14 ARM runner jobs (one builds the Intel package).
- Steps: run the parallel native Rust suite; write two existing Unicode files
  to a unique pasteboard and read them using the native adapter.
- Expected: two paths referring to the original file identities.
- Actual: successful write followed by zero read-back paths.
- Evidence: run 37882551696 failed the same assertion in both Mac jobs. The
  earlier CI run and 50 isolated / 12 complete local repetitions had passed.
- Change: explicitly link AppKit's file-URL-reading constant, provide the
  documented file-URL-only option, reject unavailable pasteboards, and retain
  item-count diagnostics. The exact reason for the previous empty read has not
  been independently isolated.
- Status: regression now passes after 4b016e8; both Mac jobs in run 37883109120
  pass the full suite and twenty repeated isolated checks each. Physical Finder
  and cross-application validation remain required.

### BUG-008: Accessibility switch enabled but the debug app remains untrusted

- Severity: S2
- Flow: Mac native permission setup / packaging
- Steps: grant Accessibility to the local app, remove/re-add its entry, and
  restart the bundle. Input Monitoring becomes available, but injection remains
  unavailable despite the Accessibility switch being enabled.
- Evidence: the app-specific macOS TCC log reports a code-requirement mismatch.
  Strict bundle validation reports missing sealed resources. The earlier ARM
  installer has the same signature defect; the Intel installer is unsigned.
- Cause: no Mac signing identity was configured; the local `--no-sign` build
  also bypassed bundle signing. A bare linker signature does not seal the bundle.
- Change: explicitly configure Tauri's ad hoc identity `-`. The corrected local
  bundle passes strict recursive verification, binds Info.plist/resources and
  uses `com.itskorrah.hoppingcursor` as its signing identifier. This does not
  establish a verified publisher, notarization, or stable grants across rebuilds.
- Status: bundle correction verified locally; native permission recovery and
  new installer signature verification are in progress. The corrected process
  is waiting on Keychain access to its existing identity before opening.
- Guidance: [Tauri ad hoc signing](https://v2.tauri.app/distribute/sign/macos/#ad-hoc-signing)
  and [Apple on TCC signing identity](https://developer.apple.com/forums/thread/730043).

## To check on physical hardware

OS permissions, hotplug/dock identities, native input, pasteboards, secure desktop,
long-duration reliability and latency. These are verification gaps, not invented
bug reproductions. See docs/PLATFORM_TESTING.md.
