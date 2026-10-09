# The Hopping Cursor

A desktop application for sharing a mouse, keyboard, clipboard, and files between your own computers on a local network. A Rust engine, native Windows and macOS input adapters, and a Tauri/React control interface. Independently implemented from public product research; no proprietary code or assets used.

**Status: development alpha.** The encrypted engine and desktop controls are implemented. Windows x64 and Intel/Apple Silicon Mac installers have built successfully on native GitHub runners. Most engine tests use an in-memory OS adapter; isolated Mac pasteboard tests exercise native APIs; user testing now confirms basic Windows↔Mac control, while broader input/permission acceptance still requires the hardware checklist below. This is not yet a production replacement for a mature software KVM.

![Native Mac 0.1.2 workspace with a connected Windows PC and three arranged displays](docs/images/workspace.jpg)

Actual macOS app window captured on 2026-10-10, with a connected two-monitor PC.
The sidebar fills the window height; this is not the browser sample preview.

**Continue development on Windows:** [Windows handoff](docs/WINDOWS_HANDOFF.md).
The latest 0.1.2 source is consolidated into `main` as an alpha checkpoint.
User testing confirms basic control in both directions. Windows→Mac mouse feel,
the E/emoji keyboard bug and connection/setup friction remain open; see below.

## What's built

- Noise XX encryption, persistent identities, matching safety codes, explicit approval on both computers, pinned keys, and revocation. No cloud account.
- mDNS discovery as an untrusted hint, manual private-IP connection, remembered authenticated addresses, trusted-only reconnection, deterministic simultaneous-connection recovery, heartbeat RTT, and input recovery on disconnect.
- Edge-based cursor routing with offset/stacked displays, coordinate scaling, hidden displays, configurable edge delay, sensitivity, and per-monitor hop shortcuts.
- Windows hooks/SendInput and macOS CGEventTap/CGEventPost adapters; keyboard forwarding, common physical-key mapping, mouse buttons/scroll, injected-event filtering, held-key handoff, and Ctrl+Alt+Escape emergency recovery.
- Plain text, HTML, and PNG clipboard exchange on separate encrypted bulk connections, type controls, per-peer permissions, bounce suppression, and source clipboard privacy-marker checks.
- Streamed file transfers, app-window drag/drop, recipient approval, progress, cancellation, SHA-256 verification, duplicate-name protection, quotas, and bounded concurrency. Opt-in file clipboard synchronization and explicit “Send copied files” / “Copy file for pasting” actions bridge native file lists without trusting remote paths.
- Desktop pairing, draggable/keyboard-accessible layout editor, saved layout profiles, hotplug refresh, stable macOS UUID / Windows monitor interface identifiers, single-instance window reopening, computer management, transfers, preferences, diagnostics, and tray controls. Cursor lock and basic edge-blocking game mode.
- Windows NSIS and Intel/Apple Silicon macOS DMG installers and build workflow in GitHub Actions. Windows packages are unsigned; Mac bundles use ad hoc signatures without Developer ID or notarization. Build success is not evidence of physical two-computer validation.

## What's still missing

Native Finder/Explorer file dragging across screen edges, folder/file-promise support; hardware display dimming; synchronized lock/wake/session state; raw relative gaming input; secure desktop/UAC/login input on Windows; media/vendor keys; code signing/notarization; measured latency/throughput and external security review. There is no commercial license/billing system in this alpha.

The research is in [replica/recon.md](replica/recon.md). Native limitations and acceptance steps are in [docs/PLATFORM_TESTING.md](docs/PLATFORM_TESTING.md). Security design and trust boundaries are in [docs/SECURITY.md](docs/SECURITY.md).

## Try the desktop app

Requirements: Node.js 22+, current stable Rust, and native build prerequisites. Windows: Microsoft C++ Build Tools and WebView2. macOS: Xcode Command Line Tools. Both computers need the app, a private LAN, and access through their firewalls.

```sh
git clone https://github.com/Itskorrah/The-hopping-cursor.git
cd The-hopping-cursor
npm ci
npm run desktop
```

On macOS, use **Grant permissions / Retry** to request **Accessibility** and **Input Monitoring**, and allow **Local Network** access when prompted. Approve these in System Settings, then retry or restart if macOS requests it. Start on a private test LAN with non-sensitive text and files. Use a local mouse on each computer during testing.

Mac bundles use an ad hoc signature for bundle integrity. They have no verified
publisher or notarization. After rebuilding, macOS may require you to remove and
re-add this app's privacy permission entry. Fully quit with **⌘Q** before reopening;
closing the window keeps the engine running. Stable Apple Development / Developer
ID signing is still needed for dependable permission continuity and distribution.

1. Open Computers on both machines. Discovery uses mDNS; if needed, use Add computer with a local address such as `192.168.1.20:24810`.
2. Pair the computer, compare the **entire safety code** on both screens, and approve it on **both**. One-sided approval does not permit sharing.
3. Arrange your displays with touching edges and save. Click **Share arrangement**, then review and **Apply arrangement** in Workspace on the other computer. Sharing never silently replaces its saved layout.
4. Confirm both apps are 0.1.2 and **Resume** sharing on both if paused. Move toward a shared edge. Your keyboard follows the cursor. Use **Ctrl+Alt+Escape** on the source computer to bring input back immediately.
5. Send files from File transfers or drop them into the app window. The receiver accepts the offer. Completed files go to the receiving folder in Preferences. To send files copied in Finder/Explorer, use **Send copied files**. A verified received file offers **Copy file for pasting** into another folder.

Select a monitor to assign **Ctrl+Alt+1–9** hop shortcuts or use **Jump here**. Save named layout profiles for different desks. Pair permissions separately control input, clipboard, and files. **Pause** returns input locally, stops sharing, and cancels pending/active file transfers.

Identity keys migrate into macOS Keychain or user-bound Windows DPAPI storage without changing pairings. A locked/missing protected identity fails startup rather than silently generating a new one. Linux/headless test configurations retain permission-restricted file storage.

Closing the window keeps the engine in the tray. **Quit** from the tray stops sharing and releases input. Display changes are checked every two seconds; input returns locally and peers reconnect with fresh display metadata. Saved profiles are reconciled against attached displays. Driver/port changes and identical monitors still need hardware validation. Default networking is TCP **24810**, mDNS UDP **5353**. No port forwarding is needed. Guest Wi-Fi/client isolation may block peers.

Build installers locally:

```sh
npm run desktop:build
```

For platform prerequisites, Windows NSIS builds and a local fallback when hosted
runners are unavailable, see [docs/BUILDING.md](docs/BUILDING.md).

For a local Mac debug bundle, run `npm run tauri -w apps/desktop -- build --debug --bundles app`.
Keep signing enabled. Verify the bundle before requesting permissions:

```sh
codesign --verify --deep --strict --verbose=2 "target/debug/bundle/macos/The Hopping Cursor.app"
```

The ad hoc default follows [Tauri's signing guidance](https://v2.tauri.app/distribute/sign/macos/#ad-hoc-signing).

For the current Windows test package, download [0.1.2-test.1](https://github.com/Itskorrah/The-hopping-cursor/releases/tag/v0.1.2-test.1), extract the ZIP and run its setup executable after fully quitting the previous app. Keep identities/settings. It is unsigned; its local cross-build provenance is included. Both computers must be updated.

For historical native-runner packages, open [GitHub Actions](https://github.com/Itskorrah/The-hopping-cursor/actions), select a successful **Build and verify** run, and download the Windows x64, macOS ARM64, or macOS Intel artifact. The workflow builds on native runners. Successful build artifacts are development packages, not signed public releases.

Extract the artifact and find the `.exe` or `.dmg` under `target/.../release/bundle/`. The archive also includes the application license and dependency notices.

## Browser interface preview

```sh
npm ci
npm run dev
```

Open `http://localhost:1420`. The visible preview banner explains that the computers are samples. Pairing examples, layouts/profiles, and preferences use local browser storage. Real OS input/network/file operations run only in the desktop app. No fabricated latency values are displayed.

## Headless engine and CLI

Linux supports encrypted protocol/transfer development and integration tests; it does **not** support global input capture in this version.

```sh
cargo run -p hopping-cursor -- identity
cargo run -p hopping-cursor -- daemon
```

For manually configured peers, compare each public identity directly on the other computer, then run this on **both** with the other machine's public key:

```sh
cargo run -p hopping-cursor -- trust --public-key OTHER_PUBLIC_KEY --address 192.168.1.20:24810 --name "Other computer"
cargo run -p hopping-cursor -- daemon --connect 192.168.1.20:24810
```

Use `--config-dir /absolute/path` before the command for an isolated development instance. `daemon --port 24811` overrides the listen port for that run. The `send --peer DEVICE_ID path/to/file` command starts a temporary engine; avoid running it concurrently against an actively edited desktop configuration.

## Verify

```sh
cargo test
cargo clippy -p hop-core -p hopping-cursor --all-targets -- -D warnings
cargo fmt --all -- --check
npm run build
npx playwright install chromium
npm run test:ui
```

The current suite has **55 Rust tests on macOS and 14 browser tests**. Rust checks include real encrypted TCP services, ciphertext tampering/replay rejection, fragmented packets during simultaneous input/heartbeats, saturated-queue emergency release, pairing/revocation, recipient consent, byte-for-byte transfers, duplicate filenames, clipboard exchange, held-key handoff, hop shortcuts, and pause/disconnect recovery. Browser tests cover preferences, pairing consent, arrangement, profiles, permissions, shortcuts, accessibility, and small-screen layout. The suite includes two native Mac checks using isolated pasteboards for file URLs and privacy markers without replacing your clipboard. These checks do not substitute for cross-application or physical multi-computer testing.

Dependency audit results and remaining warnings are recorded in [docs/SECURITY.md](docs/SECURITY.md). The prioritized path from this alpha to a release is in [docs/ROADMAP.md](docs/ROADMAP.md); the research feature matrix is in [replica/features.csv](replica/features.csv).

## Project map

```text
crates/hop-core/             Protocol, trust, routing, native adapters, transfers
crates/hop-cli/              Headless/native engine CLI
apps/desktop/src/            React desktop controls and explicit browser preview
apps/desktop/src-tauri/      Tauri IPC, tray, packaging, app icons
replica/                    Public research, architecture, skills, parity tracking
docs/                       Security model and physical-platform test checklist
.agents/skills/             Your Replica Universal pack and frontend design skill
.github/workflows/          Core/browser checks and native installer builds
```

The Replica pack was installed from [your repository](https://github.com/Itskorrah/replica-skill-universal). Additional skill discovery used [Vercel find-skills](https://skills.sh/vercel-labs/skills/find-skills); details are in [replica/skills.md](replica/skills.md).

Original application code is MIT licensed. Installed skills preserve their own licenses; dependencies and fonts retain their respective licenses. [Bundled notices](apps/desktop/public/THIRD_PARTY_NOTICES.txt) accompany installer artifacts and the app assets. Regenerate with `python3 scripts/generate-notices.py` after changing dependencies. The name is provisional; trademark/domain checks have not been performed.

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
