# Architecture

Rust workspace: `hop-core` is reusable; `hopping-cursor` is the headless CLI;
Tauri 2 embeds the same service behind narrowly scoped IPC commands. React and
TypeScript render desktop controls. No remotely hosted backend, database,
account provider, analytics, private target API, or plaintext LAN fallback.

Persistent JSON holds trusted public keys, preferences, display arrangement and
an OS-protected identity reference/public key. Native startup migrates the same
private key to Mac Keychain or user-bound Windows DPAPI only after read-back
verification. Missing protected storage fails closed. Headless/legacy configs
retain the file-stored key. Unix permissions are owner-only; Windows inherits
the application-data ACL. Do not include config/backups in support exports.
Local malware in the same user context is not contained.

mDNS advertisements are untrusted hints. TCP Noise_XX_25519_ChaChaPoly_SHA256
handshakes prove persistent key possession. Device IDs derive from public keys.
Fingerprint approval is required on both sides before accepting input/data.
Any changed pinned key fails closed. Advertisements expose names/addresses,
not keystrokes or clipboard data. The full SHA-256 pair safety code is symmetric.

Each connection has a bounded encrypted frame size and a Postcard message
codec that rejects trailing data. Input uses a dedicated control connection
(TCP_NODELAY), bounded queues and heartbeat watchdogs. A separate reader task
preserves partial frame reads across outbound traffic. Emergency Leave/release
messages bypass the normal queue and invalidate stale queued input. Native
callbacks read an immutable routing snapshot instead of acquiring the disk
configuration lock. Session encryption buffers are reused and plaintext buffers
are zeroized when dropped. Bulk
clipboard/file traffic uses independent authenticated Noise connections to
avoid holding up input. Incoming/outgoing bulk work is capped independently
at four transfers per direction; consent waiting releases handshake admission. This does not remove TCP head-of-line blocking caused
by packet loss within a control connection; QUIC is a future experiment.

Native Windows low-level hooks ignore injected events and use SendInput.
macOS CGEventTap/CGEventPost also mark injection to avoid feedback. Input is
suppressed locally only while controlling an authenticated peer. An emergency
Ctrl+Alt+Escape chord and disconnect watchdog release pressed synthetic keys.
Protected-input/permission/display-loss and timing-gap gates release ownership
conservatively. Source clipboard privacy markers are respected.
Native secure desktop input, arbitrary OS file dragging, lock/wake sync, and
hardware brightness are not represented as implemented until platform-tested.

Topology maps native display coordinates into arranged workspace coordinates.
Only visible, adjacent, online displays may receive an edge hop. The remote
virtual cursor follows physical motion while the source cursor parks locally.
Peer input is accepted only after an Enter lease from that peer; disconnect
and heartbeat expiry release that lease and any synthetic held keys.

Incoming files require consent by default. Basenames only, exclusive `.part`
creation, ordered chunks, advertised-size enforcement, hash verification,
no overwrites, and cleanup on every failed transfer. Per-file quotas limit accepted sizes; disk-full paths still require validation.
Duplicate incoming IDs cannot replace pending consent. Explicit native file-list
clipboard actions send regular files through the same policy; only verified
local received paths are published back to the OS clipboard. No executable files are auto-opened.

Build order: protocol/config/topology tests → two-process encrypted vertical
slice → native adapters → clipboard/transfers → IPC interface → installer and
CI → platform checks and honest parity report.
