# Architecture

Rust workspace: `hop-core` is reusable; `hopping-cursor` is the headless CLI;
Tauri 2 embeds the same service behind narrowly scoped IPC commands. React and
TypeScript render desktop controls. No remotely hosted backend, database,
account provider, analytics, private target API, or plaintext LAN fallback.

Persistent JSON configuration holds the private identity, trusted public keys,
preferences, and display arrangement. Owner-only Unix permissions; atomic
replacement. Windows inherits the user's application-data directory ACL.
Do not include this configuration in support exports. OS keychain integration
is future hardening; local malware in the same user context is not contained.

mDNS advertisements are untrusted hints. TCP Noise_XX_25519_ChaChaPoly_SHA256
handshakes prove persistent key possession. Device IDs derive from public keys.
Fingerprint approval is required on both sides before accepting input/data.
Any changed pinned key fails closed. Advertisements expose names/addresses,
not keystrokes or clipboard data. The full SHA-256 pair safety code is symmetric.

Each connection has a bounded encrypted frame size. Input uses a dedicated
control connection (TCP_NODELAY), bounded queues and heartbeat watchdogs. Bulk
clipboard/file traffic uses independent authenticated Noise connections to
avoid holding up input. This does not remove TCP head-of-line blocking caused
by packet loss within a control connection; QUIC is a future experiment.

Native Windows low-level hooks ignore injected events and use SendInput.
macOS CGEventTap/CGEventPost also mark injection to avoid feedback. Input is
suppressed locally only while controlling an authenticated peer. An emergency
Ctrl+Alt+Escape chord and disconnect watchdog release pressed synthetic keys.
Native secure desktop input, arbitrary OS file dragging, lock/wake sync, and
hardware brightness are not represented as implemented until platform-tested.

Topology maps native display coordinates into arranged workspace coordinates.
Only visible, adjacent, online displays may receive an edge hop. The remote
virtual cursor follows physical motion while the source cursor parks locally.
Peer input is accepted only after an Enter lease from that peer; disconnect
and heartbeat expiry release that lease and any synthetic held keys.

Incoming files require consent by default. Basenames only, exclusive `.part`
creation, ordered chunks, advertised-size enforcement, hash verification,
no overwrites, and cleanup on every failed transfer. Configurable quota protects
disk space. No executable files are auto-opened.

Build order: protocol/config/topology tests → two-process encrypted vertical
slice → native adapters → clipboard/transfers → IPC interface → installer and
CI → platform checks and honest parity report.
