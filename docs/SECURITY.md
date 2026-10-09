# Security model

All LAN payloads use `Noise_XX_25519_ChaChaPoly_SHA256` through Snow. There is no
unencrypted mode. An XX handshake proves key possession but does not by itself
identify a trustworthy computer: users must compare the full symmetric safety
code directly on both displays and approve both peers. Persistent public keys
are pinned. Device IDs derive from public keys; mDNS names are only hints.

The UI displays identities learned inside the encrypted handshake. Approval
includes the exact expected key so a changed UI candidate cannot silently be
approved. Untrusted sessions stop after introduction. Previously trusted peers
with a different key fail closed. Revocation is checked while transferring and
on control heartbeats. Initial metadata advertisements expose device name,
platform, address, and identifier on the LAN.

Input uses a separate bounded control queue and an independent reader task;
frame reads cannot be interrupted by outgoing traffic. Emergency release
bypasses a full queue and invalidates older queued input. File and clipboard data use fresh
authenticated sessions. Frames are at most 65,535 bytes, transfer chunks at most
32 KiB. The control watchdog releases ownership after roughly 1.5 seconds of
missing traffic, subject to OS scheduling. A stalled write has a shorter timeout.
Ctrl+Alt+Escape forces local recovery independently of network delivery. The
receiver releases its own synthetic keys when a connection closes. Pause
and file-permission revocation cancel active/pending file offers; permissions
are rechecked during streaming and before publication.

Incoming data is restricted to explicit message types. Only the current
controller may inject input. Coordinates must fit an actual display, input
values must be finite and bounded, file chunks must arrive in sequence, and
the advertised size and SHA-256 digest must match. Files require consent by
default; filenames cannot contain paths or Windows reserved names. Temporary
files use exclusive creation; verified files are published without replacement
using hard links. The receiving filesystem must support hard links. Files never
auto-open. Failed partial files are removed on normal error/cancellation paths;
an abrupt process/OS crash may leave `.hop-*.part` files for manual cleanup.

IPv4 listening and local/private IP connections are supported in this version.
Remote internet traversal is not implemented. Bindings to all local interfaces
still require a host firewall on shared networks. Metadata and account names
are not anonymous. Clipboard forwarding is enabled by default after deliberate
pairing and can be disabled by type or peer; copied passwords are clipboard data
too. Sensitive pasteboard flags and password-manager exclusion are future work.

Native identities use Keychain/DPAPI as described below. Headless/test and legacy
configurations use owner-only Unix files or inherited Windows ACLs. Same-user
malware can access local input and process memory; this application does not
claim to protect a compromised endpoint. Diagnostic exports
exclude identity keys and content. Diagnostic messages can include device names
and connection addresses: review before sharing them.

Native Windows and Intel/Apple Silicon Mac installers have built on GitHub's
native runners, but still require actual OS/hardware testing.
There has been no external cryptographic audit, fuzz campaign, signing, or
notarization. Use a private test LAN for initial validation. Report vulnerabilities
privately to the repository owner rather than publishing sensitive exploit data.

## Dependency audit — 2026-10-07

`npm audit` reports zero vulnerabilities, including development dependencies.
`cargo audit` reports zero vulnerability entries. Bincode was replaced with
Postcard after its maintenance advisory; maximum serialized frame size is
enforced during encoding, and trailing/malformed messages are rejected.

Cargo still reports upstream maintenance/unsoundness warnings in the desktop
dependency graph: `fxhash` (RUSTSEC-2025-0057), `proc-macro-error`
(RUSTSEC-2024-0370), and GLib's `VariantStrIter` (RUSTSEC-2024-0429). GLib is part
of Tauri's Linux GTK stack; this application currently distributes only Windows
and macOS desktop packages. These warnings are recorded rather than globally
suppressed. Track upstream Tauri/GTK migrations before adding Linux desktop
support. A dependency scan does not constitute an application security audit.

## Protected identity migration and discovery hardening (2026-10-09)

Native startup migrates the same private key into a macOS Keychain generic
password or a user-bound Windows DPAPI blob. The OS store is read back before
atomically replacing the JSON with a reference and public key. Store failure
leaves the original file intact; missing/locked/wrong-platform protected storage
fails closed without creating a replacement identity. Config copies alone no
longer suffice to restore native identities. Existing backups may still contain
legacy keys. The key exists in process memory while the engine is running;
Keychain/DPAPI does not defend against a compromised logged-in account.

Discovery no longer initiates unrequested pairing or overwrites authenticated
endpoint metadata. Reconnect uses paired identities only, tries remembered
addresses before bounded discovery hints, and verifies the expected identity
before exchanging introductions. Concurrent control connections converge on the
one initiated by the lower identity; superseded tasks cannot mark its replacement
disconnected. Bulk traffic still uses separate encrypted connections.
