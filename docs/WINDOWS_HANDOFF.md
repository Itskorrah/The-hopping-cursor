# Windows development handoff — 2026-10-10


## Latest scroll/reliability checkpoint — 0.1.5

Start with [TEST_0.1.5.md](TEST_0.1.5.md) and [Mac receiver handoff](MAC_HANDOFF_0.1.5.md).
The owner briefly tested 0.1.4 and reports good behavior/feel, but Windows mouse
scrolling on Mac failed. Mac's first wheel argument was incorrectly declared
variadic; it is now fixed. Fractional input survives both native adapters, and
Mac source normalization/receiver direction follow its local scroll setting.
Read-only preference refresh stays off the input callback. Mac constructor tests
are compile-checked on Windows and need actual Mac execution.

Scroll stage/queue diagnostics contain counts only. Exact mixed-event ordering,
first-wheel entry, denied input, queue pressure and existing security/recovery
regressions pass. The repeatable encrypted soak and bounded parser campaign
use synthetic data; their results are in the evidence report. They do not prove
physical latency or multi-hour hardware reliability. Preserve the keyboard fixes,
pairing, protected identities, injected-event filtering and emergency priority.

Windows has a native 0.1.5 test package. Rebuild Mac using its stable existing
signing setup before scroll acceptance. Protocol 2 is unchanged; older peers can
decode messages but an older Mac cannot benefit from the updated receiver.
Find Skills led to reviewed Systematic Debugging, TDD and Verification guidance,
installed locally with MIT licenses. Keep specialist guidance subordinate to
native API evidence. Continue independent work around hosted account blocks.


## Previous keyboard checkpoint — 0.1.4

Read [TEST_0.1.4.md](TEST_0.1.4.md) first. Owner testing says the 0.1.3 E/emoji fix
appears successful, but initial Windows-to-Mac typing depends on first focusing
a Windows text field. BUG-022 records this as observed behavior, not a diagnosis.

The Windows keyboard hook now has a separate pump from mouse/raw/window work.
Enter and pre-handoff held state queue under ownership lock before publishing
the new active owner. Two previously failing regressions pass; 62 core tests,
four native probes and 17 browser tests pass, as do strict and Mac compile checks.
Aggregate keyboard diagnostics expose the capture/delivery stage without keys
or typed text. Physical Windows desktop/Chrome/Notepad versus the same Mac field
still needs acceptance. Update Mac with its existing signing setup for receiver
counts; 0.1.3 is still protocol-compatible. No pairing/settings reset is needed.

Find Skills was used to search Windows keyboard guidance; no sufficiently
established matching specialist was installed. Native API documentation and the
existing Tauri/Replica guidance were used. No third-party KVM implementation
code was copied. Continue independent work around hosted billing blocks.

## Latest Windows checkpoint — 0.1.3

The Windows pass is documented in [TEST_0.1.3.md](TEST_0.1.3.md). The original
0.1.2 handoff below remains historical context. Find Skills is now installed
with provenance and license in `.agents/skills/find-skills`.

Native Windows builds work on this PC with a portable LLVM/MSVC SDK toolchain;
dot-source `scripts/use-windows-portable-toolchain.ps1` in a new PowerShell session.
The native pointer-parking probe and isolated encrypted integration tests pass.
Core code checks for both Mac targets; this does not validate Mac execution.

BUG-018's inherited Fn path is mitigated, BUG-019's Windows raw-count path and
motion backlog are changed, and BUG-020 now has actual capture readiness plus
clearer setup status. Keep all three awaiting physical acceptance. The user
clarified that BUG-020 was a general clunky impression; no exact repeated-action
sequence or underlying connection fault was established.

The Mac needs these source changes rebuilt using its existing stable local
signing override, then the physical keyboard/mouse checklist. Preserve pairing,
identities, settings and explicit arrangement acceptance. Protocol remains 2;
0.1.2 peers remain compatible, but the Mac key fix requires the updated receiver.

Continue the existing native desktop KVM. This is the **0.1.2 development alpha**
checkpoint being consolidated into `main`, not a production release. Do not
rebuild it as a website. Inspect the checkout and remote before editing; newer
work may exist. Preserve all unrelated local work.

Repository: https://github.com/Itskorrah/The-hopping-cursor (private).
The Windows test package is [v0.1.2-test.1](https://github.com/Itskorrah/The-hopping-cursor/releases/tag/v0.1.2-test.1),
built from `049d2ca556c90e222cc0d874c715c0d3257a510b`. Later checkpoint commits
update documentation and the README screenshot, without changing that binary.
PR #2 contains the implementation and handoff merge. Start future fixes from
current `main` on a new `codex/` branch.

## Read first

- `AGENTS.md`, `README.md`, `docs/ROADMAP.md`, `docs/SECURITY.md`.
- `replica/recon.md`, `replica/architecture.md`, `replica/features.csv`,
  `replica/bugs.md`, `replica/test-plan.md`, `replica/skills.md`.
- `docs/DEVELOPMENT_STATUS.md`, `docs/TEST_0.1.2.md`,
  `docs/PLATFORM_TESTING.md`, `docs/BUILDING.md`, `docs/NATIVE_DRAG_DESIGN.md`.

Use the checked-in Tauri/Replica/native-feel skills under `.agents/skills` when
relevant. Skill discovery has already been performed; review that evidence before
installing more skills. Never log physical keys, clipboard/file contents, private
identities or protected-store data. Keep discovery untrusted, explicit pairing,
pinned identities, encrypted-only transport and control/bulk separation intact.
Emergency release and held-key cleanup must survive failures and queue pressure.

## Physical evidence and open issues

The user's desk has **Mac left → Windows primary middle → Windows secondary right**.
The user's 0.1.2 test confirms that both physical mice/keyboards can now control
the other computer after setup. Mac→Windows movement and typing feel smooth.
This is user-reported acceptance of basic handoffs, not exhaustive hardware QA.
The screenshot in `docs/images/workspace.jpg` is an unedited capture of the real
Mac 0.1.2 window on 2026-10-10 with the PC connected; it is not sample browser data.

Prioritize these observed issues:

1. **BUG-018 — Windows keyboard E opens the Mac emoji picker.** Take control
   from Windows, focus a harmless blank Mac editor, and press E without modifiers.
   Expected: a letter. Actual, reported by the user: emoji picker. Reproduce with
   mapped modifiers enabled/disabled, after controller changes and held-key cleanup.
   Check ordinary letters, left/right modifiers, shortcuts and key-up recovery.
   Mac injection currently preserves event function/lock flags while rebuilding
   Shift/Control/Option/Command flags. Apple's Fn/Globe+E emoji shortcut makes
   unintended Fn state a concrete suspect, **not a proven cause**. Check Windows
   scan-code→HID conversion and the entire receiving path too. This may require
   a Mac adapter change; Windows-local access alone cannot validate the receiver.
2. **BUG-019 — Windows mouse→Mac feels clunky.** Mac trackpad→Windows feels
   smoother. Windows now captures relative Raw Input counts; service routing
   adds those deltas times sensitivity directly to layout coordinates. Investigate
   mouse DPI/polling, pointer gain, logical/physical scaling, event cadence,
   queue pressure/coalescing and Mac injection before choosing a fix. Compare
   slow precise motion, fast movement, diagonals, scrolling and held drags.
   Do not mask the problem with arbitrary smoothing or invent latency numbers.
3. **BUG-020 — setup/recovery requires repeated actions.** The user tried Connect,
   disconnect/reconnect and Pause/Resume before sharing worked. The exact state
   sequence is not independently reproduced. Observe both versions, pairing,
   input permission, paused state, accepted layout and ownership state separately.
   Test startup in either order, reconnect, sleep and app restart. Make the UI
   explain the actual blocker and recover automatically where already authorized.
   Preserve explicit pairing/layout consent and deliberately paused sharing.

BUG-014's complete Windows→Mac handoff failure is superseded by the user's basic
0.1.2 success; broader acceptance remains open. BUG-010 duplicate interaction,
BUG-013 actual Windows cursor visibility, held buttons/modifiers and high-DPI
transitions still need explicit checks. Do not infer their success from silence.
File transfer UI, native file paste, HTML/PNG interoperability and file privacy
behaviour still require the physical checklist. Native Finder/Explorer **edge
file dragging is unimplemented**; app-window drops are a separate feature.
Folders/file promises, cut/move, dimming, lock/wake sync, privileged desktops,
media/international/game compatibility, production signing/notarization/updates
and external security review remain unfinished.

## Primary code paths

- `crates/hop-core/src/platform/windows.rs`: hooks, Raw Input and parking window.
- `crates/hop-core/src/platform/macos.rs`: injection source, modifier flags and tap.
- `crates/hop-core/src/platform/keys.rs`: scan-code/HID/native key tables.
- `crates/hop-core/src/topology.rs`: coordinates and modifier translation.
- `crates/hop-core/src/service.rs`: capture, ownership, pairing, reconnect and queues.
- `apps/desktop/src/App.tsx`: connection, pause, permissions and arrangement UI.
- `apps/desktop/src-tauri/src/lib.rs`: startup readiness, IPC and tray lifecycle.
- `crates/hop-core/tests/network.rs`, `apps/desktop/e2e/`: existing regressions.

## Windows checkout and toolchain

Clone into a new folder from a chosen parent directory. Authenticate to the
private repository through GitHub's normal sign-in; do not put tokens in URLs.
If a checkout already exists, inspect `git status`, branches and remotes first;
do not reset/clean or pull over local work.

```powershell
git clone --branch main https://github.com/Itskorrah/The-hopping-cursor.git
cd The-hopping-cursor
git status --short
git log -1 --oneline
```

Open this folder as a **local Windows project in Codex**. Native execution needs
Windows access, not WSL/headless Linux. Check Git, Node.js 22+, npm, stable Rust
MSVC, Microsoft C++ Build Tools with **Desktop development with C++** plus Windows
SDK, and WebView2. Follow [Tauri's Windows prerequisites](https://v2.tauri.app/start/prerequisites/#windows)
for missing tools. The owner completes any OS administrator/license dialogs;
do not weaken security or change billing to avoid them.

Fully **Quit** the installed app using its tray menu before launching a development
engine. Closing its window leaves it running. Avoid duplicate instances on TCP
24810 or concurrent editing of the same config. Preserve the existing Windows
DPAPI-protected identity, pairing and layouts; do not import the Mac's config/key.
The local Mac app is `~/Applications/The Hopping Cursor.app`; its signing override,
certificate, build cache and `.local` files are intentionally not committed.

Run commands individually and stop on failure. The PowerShell `.cmd` form avoids
requiring a global execution-policy change for npm.

```powershell
node --version
npm.cmd --version
rustc --version
cargo --version
rustup show active-toolchain
npm.cmd ci
cargo test --locked
cargo clippy --locked -p hop-core -p hopping-cursor --all-targets -- -D warnings
cargo clippy --locked -p hopping-cursor-desktop --all-targets -- -D warnings
cargo fmt --all -- --check
npm.cmd run build
npx.cmd playwright install chromium
npm.cmd run test:ui
npm.cmd run desktop
```

`cargo test` defaults to core/CLI; include native desktop Clippy separately.
Windows tests differ from Mac-specific tests; record the actual Windows count.
Default tests do not opt into replacing your native file clipboard. Any disposable
clipboard experiment needs a controlled test fixture and deliberate opt-in.
Use the Mac as the second physical peer. Both installed apps must speak protocol 2;
update both packages if changing protocol. Allow only necessary private-LAN access
(TCP 24810 and mDNS UDP 5353). Preserve Mac's existing stable local signing setup.

## Verification and delivery

Last implementation checkpoint on Mac: **55 Rust tests (21 unit/native + 34
integration), 14 Playwright tests**, TypeScript/Vite, Rustfmt, native desktop Clippy,
and strict core/CLI Clippy for ARM Mac, Intel Mac and Windows x64. The native Mac
bundle strictly verifies, opens with the same protected identity, has all three
permissions Available and passes recovery IPC. The Windows 0.1.2 package's NSIS
archive, AMD64 app, 0.1.2.0 resources and extracted bytes were verified, as was the
GitHub download SHA-256. The user subsequently installed and tested it physically.
These are different evidence levels; do not claim a Windows-native automated
suite already ran for the cross-build or measured smoothness/latency.

Hosted Actions run 37920694980 failed before every job's first step because of
an existing account payment/spending-limit restriction. Do not change billing,
spending limits or pay for runners. Native Windows-local builds are now preferred.
The existing approved Mac SDK cross-build route is documented in BUILDING.md.

For each fix retain a failing-before/passing-after regression where feasible,
then run appropriate checks plus physical acceptance. Changes must preserve the
smooth Mac→Windows path. Deliver a clearly distinguishable new test version so
the user can verify the installed binaries. Native Windows NSIS build:

```powershell
npm.cmd run tauri -w apps/desktop -- build --target x86_64-pc-windows-msvc --bundles nsis -- --locked
```

Update README, feature status and bugs with evidence; commit/push the fixes and
provide installer provenance/hashes. Record exactly what is still pending. Keep
merging an alpha checkpoint distinct from declaring a finished production app.

## Suggested first Codex message on Windows

> Read AGENTS.md, README.md and docs/WINDOWS_HANDOFF.md. Inspect and preserve local
> work, verify the native Windows development setup, and continue from current
> main on a new codex/ branch. Reproduce and fix BUG-018 (E opens Mac emojis), then
> BUG-019 (Windows→Mac mouse feel) and BUG-020 (connection/setup recovery). Preserve
> the working Mac→Windows path, trust/encryption, existing identities and emergency
> release. Use relevant installed skills, add meaningful regressions, run native
> checks and guide physical tests with my Mac. Keep progress moving around blockers,
> commit/push changes and provide updated test installers. Do not change billing,
> weaken security or claim unfinished native file dragging works.
