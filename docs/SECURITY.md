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
authenticated sessions. Incoming and outgoing bulk work are limited to four
connections each; waiting consent does not retain control handshake slots.
Frames are at most 65,535 bytes, transfer chunks at most
32 KiB. The control watchdog releases ownership after roughly 1.5 seconds of
missing traffic, subject to OS scheduling. A stalled write has a shorter timeout.
Ctrl+Alt+Escape forces local recovery independently of network delivery. The
receiver releases its own synthetic keys when a connection closes. Pause
and file-permission revocation cancel active/pending file offers; permissions
are rechecked during streaming and before publication.

Incoming data is restricted to explicit message types. Only the current
controller may inject input. Coordinates must fit an actual display, input
values must be finite and bounded, file chunks must arrive in sequence, and
the advertised size and SHA-256 digest must match. Active/recent transfer identities cannot be reused to replace consent state.
Files require consent by default; filenames cannot contain paths or Windows reserved names. Temporary
files use exclusive creation; verified files are published without replacement
using hard links. The receiving filesystem must support hard links. Files never
auto-open. Failed partial files are removed on normal error/cancellation paths;
an abrupt process/OS crash may leave `.hop-*.part` files for manual cleanup.

IPv4 listening and local/private IP connections are supported in this version.
Remote internet traversal is not implemented. Bindings to all local interfaces
still require a host firewall on shared networks. Metadata and account names
are not anonymous. Clipboard forwarding is enabled by default after deliberate
pairing and can be disabled by type or peer. Reads skip Mac concealed, transient,
generated, and known password-manager markers, plus Windows history/cloud
exclusion formats. Clipboard revision and markers are rechecked after reading.
This conservative policy follows the [NSPasteboard marker specification](https://nspasteboard.org/)
and [Microsoft's clipboard formats](https://learn.microsoft.com/en-us/windows/win32/dataxchg/clipboard-formats#cloud-clipboard-and-clipboard-history-formats).
Unmarked passwords or secrets remain ordinary clipboard data: disable clipboard
sharing when source applications do not provide reliable privacy markers.

File clipboard actions are explicit, require file and clipboard peer permissions,
and retain recipient consent. Only verified received local regular-file paths
can be published for OS pasting. Remote filenames never become arbitrary OS file
URLs or executable commands. Folders, virtual file promises and automatic
file-clipboard synchronization remain unsupported.

Protected input sessions, permission loss, missing displays, disabled Mac event
taps and timing gaps conservatively release ownership. Mac Secure Event Input
and Windows input-desktop checks are safety gates, not full session notification
coverage or privileged desktop access. Physical lock/sleep behavior still needs
validation; no lock/wake synchronization is implemented.

Native identities use Keychain/DPAPI as described below. Headless/test and legacy
configurations use owner-only Unix files or inherited Windows ACLs. Same-user
malware can access local input and process memory; this application does not
claim to protect a compromised endpoint. Diagnostic exports
exclude identity keys and content. Diagnostic messages can include device names
and connection addresses: review before sharing them.

Native Windows and Intel/Apple Silicon Mac installers have built on GitHub's
native runners, but still require actual OS/hardware testing.
Mac bundles use ad hoc signatures for integrity, checked by strict bundle
verification in CI. Developer ID / AuthentiCode signing, notarization and an
external cryptographic audit remain unfinished. A
short AddressSanitizer parser fuzz campaign passed 5,299,006 cases in 181 seconds;
see [the harness and limits](../fuzz/README.md). It does not establish exhaustive
protocol or native-adapter assurance. Use a private test LAN for initial validation. Report vulnerabilities
privately to the repository owner rather than publishing sensitive exploit data.

## Dependency audit — 2026-10-09

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

### Protocol 2 and copied-file batches

0.1.2 requires protocol 2 on both computers. Authenticated focus and idle-keyboard
messages are gated by local input permission and session availability. Layout
proposals validate known displays, dimensions, duplicate IDs and overlaps; they
cannot write settings until the recipient applies them. Focus messages carry
public computer IDs, not input contents. Shared-key state is cleared on recovery
and peer disconnect and is never exported.

File clipboard synchronization is opt-in. A sender requires file and clipboard
permission, limits each batch to 100 regular files and rejects symlinks/folders
for automatic sending. Each incoming file retains ordinary explicit consent
unless the receiver deliberately enabled auto-accept. Native clipboard publication
requires the entire verified batch, current permissions and an unchanged recipient
clipboard. Newer batches supersede older pending ones. Temporary partial files
retain cleanup; completed downloads remain normal received files. This implements
copy only, never source deletion or shell move feedback. Physical Finder/Explorer,
OS clipboard races and independent security review remain acceptance work.
