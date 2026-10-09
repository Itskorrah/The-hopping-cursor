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
- Status: fixed in the desktop-hardening batch; parallel suite passes.

### BUG-002: Strict Clippy fails in native Mac clipboard conversion

- Severity: S3
- Steps: strict Clippy on macOS with Rust 1.99.
- Expected: clean native adapter checks.
- Actual: chunks_exact_to_as_chunks lint fails two PNG conversion paths.
- Fix: typed slice chunks; strict native Clippy also added to CI.
- Status: fixed in the desktop-hardening batch.

### BUG-003: Connected count includes the local computer

- Severity: S3
- Steps: open native Workspace with no paired peers.
- Expected: zero paired and connected computers.
- Actual: summary claims one paired computer.
- Fix: show the actual connected peer count.
- Status: fixed in the desktop-hardening batch.

## To check on physical hardware

OS permissions, hotplug/dock identities, native input, pasteboards, secure desktop,
long-duration reliability and latency. These are verification gaps, not invented
bug reproductions. See docs/PLATFORM_TESTING.md.
