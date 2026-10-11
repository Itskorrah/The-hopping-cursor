# Development evidence

Latest checkpoint: [Mac 0.1.6 cursor handoff evidence](TEST_0.1.6.md), 2026-10-11.
Background cursor capability and synchronous parking regressions pass, including
lifecycle cleanup and edge/jump ordering. 75 default Rust tests, two separately
run native cursor probes and 18 browser tests pass. Strict Apple Silicon
engine/CLI/desktop checks and Intel Mac/Windows engine/CLI cross-checks pass.
Physical single-cursor acceptance is still pending; the scoped WindowServer
capability is undocumented. Windows 0.1.5 remains compatible.
The signed 0.1.6 Mac debug app is installed with unchanged signing requirement
and every configuration field preserved. Native permissions are available,
the paired PC reconnects, and Jump here/Bring input here pass local IPC checks.

The following checkpoints are historical evidence.

Latest checkpoint: [0.1.4 keyboard handoff evidence](TEST_0.1.4.md).
Separate Windows keyboard pump and atomic remote entry fix two demonstrated
local weaknesses. Aggregate keyboard-stage diagnostics and 32 repeated first-key
plus shared-keyboard handoffs pass. Owner reports 0.1.3 E/emoji now appears fixed;
the physical source-focus typing issue remains open for paired acceptance.

The following 0.1.3 and earlier checkpoints are historical evidence.

Latest checkpoint: [Windows-native 0.1.3 evidence](TEST_0.1.3.md), 2026-10-10.
Capture readiness, inherited Mac Fn flags and Windows motion processing/queue
pressure have changes and regressions. Native Windows toolchain/installer and
checks pass; Mac receiver and complete physical two-computer acceptance remain
open. The earlier dated checkpoints below remain historical evidence.

The project remains a development alpha. This pass continues the existing Rust/
Tauri implementation and preserves peer identities, explicit approval, pinned
keys, encrypted transport, separate bulk traffic, and emergency release.

Earlier sections record the initial setup checkpoint. The current **0.1.2**
implementation and delivery evidence are recorded at the end of this document.

## Implemented in the initial setup

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
- Restrict native Mac URL reads with the documented file-URL-only option and
  explicitly link AppKit's option constant. Missing pasteboards return an error.
  CI repeats isolated native pasteboard checks to catch intermittent failures.
- Ensure shutdown marks peers disconnected, rejects new manual connections,
  and aborts reader tasks even when stop precedes task scheduling.
- Add a bounded parser fuzz harness with synthetic seeds and AddressSanitizer.
  A later Linux campaign reached its initial 512 MiB whole-process RSS cap;
  the saved synthetic input replays successfully and is retained as a decoder
  regression. Campaigns now use a bounded 2048 MiB budget with sanitizer
  protections unchanged; see [fuzz evidence](../fuzz/README.md).
- Fix current stable Rust's macOS clipboard lint; check native Clippy in CI.
- Configure explicit Mac ad hoc bundle signing and strict recursive signature
  validation in CI. An enabled Accessibility switch did not grant trust to the
  earlier invalid bundle; app-specific TCC logs showed a code-requirement mismatch.
  The corrected local bundle now seals its resources/Info.plist and passes
  validation. Ad hoc signing does not provide a verified publisher or dependable
  permission continuity across rebuilds; Developer ID signing remains required.

## Verified locally

- 42 Rust tests pass in parallel on Apple Silicon macOS: 19 unit/security/native and
  23 real encrypted TCP integration tests with an in-memory OS adapter.
- Native Mac file-URL round trips and privacy-marker checks pass on isolated
  pasteboards, including Unicode filesystem identities; the user's clipboard
  is not replaced by these tests. Finder/Explorer destination behavior still
  needs cross-application acceptance testing.
- A 181-second AddressSanitizer parser campaign executed 5,299,006 cases without
  a crash/assertion, peak RSS 549 MiB. This is a bounded smoke campaign.
- The separate two-service multicast discovery/rename test passes on this LAN.
- Strict Clippy passes for Apple Silicon, Intel Mac, and Windows x64 core/CLI.
  Cross-compilation verifies types and API declarations; it is not Windows or
  Intel runtime evidence.
- Frontend production build and all 10 Playwright tests pass, including axe on
  five pages and a 390px viewport. These exercise the labelled browser preview.
- Native Mac debug bundle builds and launches with real local display data.
  Actual Keychain migration preserved the preexisting public identity and removed
  the private key from the real JSON config. An enabled Accessibility entry still
  left injection unavailable until the signing defect in BUG-008 was corrected.
  The corrected ad hoc bundle passes strict signature validation. After refreshing
  both privacy entries and completing Keychain access, it opens with one real
  display and zero paired/connected peers. Diagnostics confirm capture, injection
  and clipboard access are Available. Native Pause → Resume restores Workspace
  ready; Bring input here confirms input returned locally. The public identity
  is unchanged. These verify native startup, permissions and IPC controls;
  cross-computer input and physical emergency-chord behavior were not tested.

## Native CI evidence

[Run 37880413374](https://github.com/Itskorrah/The-hopping-cursor/actions/runs/37880413374)
passed on implementation commit b0344df: Linux core/UI, bounded parser fuzzing,
Windows x64 NSIS and Intel/Apple Silicon DMG packages. Windows also exercised
real user-bound DPAPI round-trip/tamper/non-overwrite and a disposable-runner
CF_HDROP clipboard round-trip. These do not prove physical input or Explorer
paste behavior. The package notice inventory was subsequently regenerated for
new native dependencies and is checked by CI before packaging.
The later parser replay/campaign and repeated native clipboard checks are
recorded in fuzz/README.md and replica/bugs.md; historical CI runs should be
distinguished from the current branch's Actions results.

[Run 37883109120](https://github.com/Itskorrah/The-hopping-cursor/actions/runs/37883109120)
passed all five jobs on 2bc01a4, including the refreshed notices and both Mac
clipboard repetitions. Its Mac jobs both ran on ARM hosts; the Intel artifact
was cross-compiled. CI now selects macOS 15 ARM and macOS 15 Intel runners
explicitly to exercise both native ABIs and avoid the retiring macOS 14 image.
Runner labels follow [GitHub's supported architectures](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).

[Run 37884039823](https://github.com/Itskorrah/The-hopping-cursor/actions/runs/37884039823)
passed all five jobs on a18c1d1. The native suite, strict core/CLI Clippy and twenty
isolated pasteboard repetitions passed on both actual ARM and Intel Mac hosts.
Windows native tests and CF_HDROP checks passed; all three installer targets built.
This closes the earlier Intel runtime coverage gap without establishing physical
input, Finder/Explorer paste acceptance, or multi-computer behavior.

Strict signature inspection later exposed defects in those earlier Mac packages;
they are superseded. Explicit ad hoc signing now seals Mac bundles. The workflow
mounts each finished DMG read-only, verifies its packaged app recursively and
checks its identifier/version before uploading. The guard rejects the earlier
invalid ARM package and detaches the disk image on failure. The initial guard
attempt targeted a staging app that Tauri deletes after DMG-only packaging;
verification now inspects the finished package. See current Actions results for
the latest installer checkpoint.

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

## 0.1.1 three-screen input repair — 2026-10-09

The first physical Windows/Mac report reproduced wrong monitor selection,
Windows local wraparound and duplicate source interaction. New regressions
confirm enumeration-order placement, local-to-local warping and lost held input
between remote monitors. These are corrected in source; 47 Mac Rust tests,
11 browser tests, frontend build and strict core/CLI Clippy for Windows and both
Mac targets pass locally. Native Mac filter tests construct isolated events
without injecting them into the user's desktop. Mapping, cursor suppression
and lease changes need a physical retest of both updated apps. BUG-010 remains
open for that confirmation; no new completed physical acceptance claim is made.

Version 0.1.1 retains pairing identity/configuration and protocol version 1.
Saved arrangements are preserved and repaired explicitly using OS display order;
configure the same arrangement on each computer. Mac source cursor suppression
is balanced on release; native aggregate suppression diagnostics contain no keys,
clipboard data or file contents. Hosted Windows installer generation is currently
blocked before jobs start by GitHub's account payment/spending-limit restriction;
no billing configuration or financial commitment has been changed.

The local 0.1.1 Mac app is now installed in `~/Applications/The Hopping Cursor.app`
and signed with an existing Apple Development identity via an ignored local
override. Strict signature verification passes. After refreshing only this
app's existing privacy grants, Diagnostics shows capture, injection and clipboard
access Available, one paired/connected PC and the unchanged public identity.
The live native tap is enabled at the active session location with filtering
options 0. Resume, Bring input here and Pause pass native IPC checks. The saved
Mac layout now places the PC primary adjacent to the Mac, followed by its
secondary monitor. Sharing is left paused for the two-updated-app trial.

[Hosted run 37895937871](https://github.com/Itskorrah/The-hopping-cursor/actions/runs/37895937871)
again finished all five jobs with zero executed steps because of the account
restriction. With explicit owner approval for Microsoft's build-tool licence,
a local Tauri/cargo-xwin MSVC/NSIS build produced the
[Windows 0.1.1 test release](https://github.com/Itskorrah/The-hopping-cursor/releases/tag/v0.1.1-test.1)
from source commit `56cdf1e`. Both executable version resources are 0.1.1.0;
the app is AMD64. The installer passes NSIS archive integrity checks. Extracted
application bytes match the build allowing only the documented `UNK`→`NSS`
bundle marker, and uploaded ZIP hashes are checked. The package includes license,
notices, hashes and build provenance. Missing CRT PDB warnings affect debugging
symbols; the compile/link/package completed. Windows native runtime checks and
physical input acceptance have not run for this cross-build. It is unsigned;
production distribution remains unfinished. See [build steps](BUILDING.md).

## 0.1.2 shared desk — 2026-10-09

Physical user feedback for **0.1.1**: Mac trackpad → Windows, typing, Enter and
text copy/paste were useful and fairly smooth. Windows physical mouse/keyboard
→ Mac failed, and the Windows pointer remained visible after returning to Mac.
The prior automated routing test did not exercise Windows' clipped native mouse
motion. No exact OS root cause was remotely reproduced on the PC.

**0.1.2** adds a Windows Raw Input sink, dedicated parking-window cursor ownership,
authenticated focus coordination, idle-keyboard routing, accepted layout sharing
and opt-in verified file clipboard batches. A new regression exposed that urgent
Leave invalidation discarded a queued focus announcement; reordering fixes it.
New-controller Enter no longer echoes Leave back to its sender. Disconnects clear
synthetic shared keys, and physical keys on the destination preserve its inbound
mouse lease. Clipboard checks cover consent, echo, batch supersession and newer
recipient clipboard preservation. UI adds honest controller/destination/visibility
state and native chrome conventions.

Local verification: **55 Rust tests (21 unit / 34 encrypted integration)**,
**14 Playwright browser tests**, TypeScript/Vite, strict core/CLI Clippy on
Apple Silicon, Intel Mac and Windows x64, Rustfmt and parser harness compile pass.
Dedicated multicast discovery remains an ignored separate check; no new parser
fuzz campaign or end-to-end latency benchmark was performed for this pass.
Windows native runtime is not exercised by cross-checks. The release checklist
in TEST_0.1.2.md is still pending. Native edge file dragging, folders/promises,
privileged desktops, signed production distribution and external review remain
unfinished. This branch is for controlled test delivery and is not merged into main.

Startup sampling traced the local 0.1.2 launch wait to SecItemCopyMatching in
macOS SecurityAgent. Engine startup now runs on a dedicated background thread;
IPC fails closed until ready, while the window/tray and Quit remain responsive.
An IPC-mocked browser check confirms no sample computers appear during this wait.
No Keychain approval, password bypass or new identity fallback was automated.
Verified received files survive OS clipboard publication failure, with a visible
diagnostic fallback; its encrypted transfer regression passes.

### 0.1.2 delivery checkpoint

The refreshed Apple Development-signed local Mac app verifies strictly and opens
with the unchanged public identity and one saved PC pairing. Diagnostics shows
capture, injection and clipboard Available; Bring input here succeeds via native
IPC. Sharing remains paused. An incoming old PC connection is explicitly rejected
for protocol mismatch; both computers must update. The protected startup wait
is covered by the IPC mock; no human password entry was automated.

The [Windows test release](https://github.com/Itskorrah/The-hopping-cursor/releases/tag/v0.1.2-test.1)
is built from `049d2ca556c90e222cc0d874c715c0d3257a510b`. Its AMD64 application
and NSIS installer report 0.1.2.0. Archive integrity and extracted application
bytes pass with only the documented NSIS marker mutation. The uploaded ZIP and
GitHub download round trip match SHA-256
`b51b22c1ddeb44f19434bf846caa4d90e8c3f527ca6a89cfe461c923c1fd037d`.
The package includes license, notices, build provenance and physical checklist.
The installer is unsigned; actual Windows installation/runtime remains pending.
Hosted run 37920694980 completed all five jobs with **zero executed steps**;
its annotation reports failed recent account payments or a spending limit.
Local builds used the already-authorized SDK route, without billing changes.
PR #2 remains draft and the branch is unmerged pending physical acceptance.

## 2026-10-10 alpha checkpoint and Windows handoff

The user installed the Windows 0.1.2 package and reports that physical mice and
keyboards now control the other computer in both directions. Mac-source input
feels smooth. Windows-source motion is clunky, E opens the Mac emoji picker,
and setup needed reconnect/Pause/Resume experimentation. These are tracked as
BUG-018–020; no new input fixes or benchmark are claimed. Cursor rendering, held
input, broad reliability and native file paste/drag acceptance remain pending.

The owner authorized consolidating 0.1.2 into main as a development alpha and
continuing on Windows. The README uses an unedited native window screenshot
with the real PC connected; the sidebar covers the visible window height.
WINDOWS_HANDOFF.md includes prerequisites, reproduction, evidence limits,
security constraints, build/test commands and a first Codex prompt. The installed
0.1.2 binaries are unchanged by this documentation pass. Historical draft/
unmerged notes above describe earlier checkpoints, not the current alpha merge.
