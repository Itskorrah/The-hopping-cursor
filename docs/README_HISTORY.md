# Development history — early alpha checkpoints

> This page archives the technical release notes previously embedded in the
> root README. They record the evidence and limitations **at the time of each
> checkpoint**, not current test results or release guarantees.
> For today's high-level view, return to the [README](../README.md);
> for detailed dated evidence use [development status](DEVELOPMENT_STATUS.md),
> [test reports](TEST_0.1.6.md), and the [roadmap](ROADMAP.md).

## Initial setup verification (historical, 2026-10-09)

42 Rust tests and 10 Playwright browser tests pass on Apple Silicon macOS.
Strict Clippy passes locally, and the Windows/Intel adapters are cross-checked.
The native debug app launches; Keychain migration preserved the existing public
identity and removed the private key from JSON. This does not validate native
input across physical computers. Native CI checks and packages all three targets.
After correcting Mac bundle signing and refreshing privacy entries, native
diagnostics show capture, injection and clipboard access Available. Pause/Resume
and Bring input here pass local IPC checks with zero paired peers. Mac packages
are now checked inside their finished disk images before CI uploads them.

The loopback tests disable multicast so independent tests cannot discover one
another. Run the dedicated real-mDNS check separately on an isolated private LAN:

```sh
cargo test --locked --test discovery -- --ignored
```

Recovery also suspends input for unavailable displays/permissions and protected input sessions, and clears stale ownership/keys after timing gaps. This is safety recovery, not synchronized lock/wake control.

A bounded AddressSanitizer parser fuzz run completed 5,299,006 cases in 181 seconds without a crash; see [fuzz/README.md](fuzz/README.md) for scope and commands.

The CLI supports `daemon --no-discovery` for manual/remembered connections.
See [docs/DEVELOPMENT_STATUS.md](docs/DEVELOPMENT_STATUS.md) for detailed evidence.

## Input-routing update (historical 0.1.1, 2026-10-09)

The three-screen Mac → Windows primary → Windows secondary hardware trial
exposed incorrect monitor order, local Windows wraparound, and duplicate Mac
input. Version 0.1.1 preserves native monitor geometry for newly discovered
computers, leaves local monitor movement to the OS, and keeps held input leased
when crossing two screens on the same remote computer. Mac capture uses an
active session filter, a marked private injection source and a hidden, parked
source cursor; Windows source cursors are restored on release. Unmapped keys
are suppressed on the source while controlling a peer. Diagnostics expose
only an aggregate suppressed-event count, never key or event contents.

47 Rust tests and 11 browser tests pass locally; strict checks pass for Windows
and both Mac architectures. These fixes still require a physical retest on both
updated apps. Do not use the 0.1.0 installer to evaluate Windows-side fixes.
Existing pairing identities and saved layouts are retained.

For an existing arrangement, click **Use OS display order**, inspect the
**Primary** label, then save. Configure the same physical layout on both
computers: Mac on the left, Windows primary in the middle, Windows secondary
on the right. Layouts are saved independently; saving on one computer does not
modify the other.

The [Windows 0.1.1 test download](https://github.com/Itskorrah/The-hopping-cursor/releases/tag/v0.1.1-test.1)
is available from the repair branch. It was cross-built locally on macOS because
GitHub blocked hosted jobs before they started. Archive integrity, packaged app
bytes (allowing only Tauri's NSIS bundle marker), x64 architecture and 0.1.1
version resources pass verification. It is unsigned and still needs Windows
installation/native input testing at that checkpoint. It is superseded by 0.1.2.
The ZIP includes update instructions, license, notices and build metadata.
No paid runners or billing changes were used.

## Shared-desk update (0.1.2, 2026-10-09)

Both apps must use **0.1.2**. Protocol 2 adds authenticated cursor focus,
idle-keyboard forwarding and explicit arrangement offers. Old protocol 1 apps
are rejected; pairings and layouts are retained. The latest test installer is
[Windows 0.1.2](https://github.com/Itskorrah/The-hopping-cursor/releases/tag/v0.1.2-test.1).
The implementation is consolidated into `main` as a development alpha; the downloadable package remains a test prerelease.

Windows reads hardware mouse deltas through Raw Input so motion continues at a
clipped screen edge. Hooks still suppress ordinary source clicks/keys. A tiny
non-activating parking window owns the hidden Windows cursor without changing
system cursor themes. Peers coordinate cursor visibility when entering, leaving,
switching physical mice and recovering. Keyboards on idle peers follow the
shared destination; moving a physical mouse or clicking reclaims that computer.
Diagnostics show the controller, destination and intended local cursor visibility.
These state checks cannot prove what a physical Windows desktop renders.

Enable **Preferences → File clipboard** on both computers to copy 1–100 regular
files in Finder/Explorer, accept their transfer, then paste them on the other OS.
Files are fully downloaded and verified before native clipboard publication.
Existing quotas, per-peer file **and** clipboard permissions, recipient consent,
copy-only semantics, duplicate-name protection and cancellation remain in force.
Newer clipboard edits are preserved; superseded batches do not replace newer
copies. This can require a wait before pasting large files. Folders, file promises,
cut/move semantics and cross-screen native dragging are still unsupported.

55 Rust tests, 14 browser tests, strict Clippy for Windows and both Mac targets,
Rustfmt, frontend build and parser harness compile checks pass locally.
User-reported 0.1.1 Mac→Windows trackpad/typing/Enter/text copy-paste worked;
Windows→Mac and the stationary Windows cursor did not at that checkpoint.
On 2026-10-10 the user confirmed 0.1.2 basic control now works in both directions.
Mac→Windows feels smooth; Windows→Mac mouse movement feels clunky and pressing
E on the Windows keyboard opens the Mac emoji picker. Initial connection required
reconnect and Pause/Resume experimentation. These remain open bugs, not completed
fixes. Explicit single-cursor, held-input, high-DPI, clipboard-file and recovery
acceptance remains pending.
See [the update checklist](docs/TEST_0.1.2.md) and
[native drag research](docs/NATIVE_DRAG_DESIGN.md). No new latency benchmark or
Windows secure-desktop support is claimed.

Protected identity access now runs off the desktop UI thread. While a system
approval is pending, the window shows startup guidance and Quit remains available.
Identity access failures keep sharing unavailable and preserve the existing key.
If the OS file clipboard is busy after receiving a verified batch, Diagnostics
explains that the saved files remain available through File transfers.

## Windows continuation (2026-10-10)

Start from current `main`; [docs/WINDOWS_HANDOFF.md](docs/WINDOWS_HANDOFF.md)
contains prerequisites, commands, source pointers, known bugs and a reusable
Codex prompt. Priorities are keyboard correctness (BUG-018), Windows→Mac pointer
consistency (BUG-019), and clear, dependable setup/reconnection (BUG-020). The Mac
remains the receiver/test partner; Windows-local testing does not establish Mac
injection correctness. Hosted CI is currently account-blocked before jobs start;
use local verification/builds without changing billing.
