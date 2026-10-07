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

Input uses a separate bounded control queue; file and clipboard data use fresh
authenticated sessions. Frames are at most 65,535 bytes, transfer chunks at most
32 KiB. The control watchdog releases ownership after roughly 1.5 seconds of
missing traffic, subject to OS scheduling. A stalled write has a shorter timeout.
Ctrl+Alt+Escape forces local recovery independently of network delivery. The
receiver releases its own synthetic keys when a connection closes.

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

Private identity storage is local JSON with owner-only Unix permissions and
the application-data directory's inherited Windows ACL. It is not yet backed
by Keychain/DPAPI. Same-user malware can steal that key and local input; this
application does not claim to protect a compromised endpoint. Diagnostic exports
exclude identity keys and content. Diagnostic messages can include device names
and connection addresses: review before sharing them.

Native adapters have been cross-compiled but require actual OS/hardware testing.
There has been no external cryptographic audit, fuzz campaign, signing, or
notarization. Use a private test LAN for initial validation. Report vulnerabilities
privately to the repository owner rather than publishing sensitive exploit data.
