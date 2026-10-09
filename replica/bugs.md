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
- Status: fixed in 1091ac5 (native CI checks in 8c8de86); parallel suite passes.

### BUG-002: Strict Clippy fails in native Mac clipboard conversion

- Severity: S3
- Steps: strict Clippy on macOS with Rust 1.99.
- Expected: clean native adapter checks.
- Actual: chunks_exact_to_as_chunks lint fails two PNG conversion paths.
- Fix: typed slice chunks; strict native Clippy also added to CI.
- Status: fixed in 1091ac5 (native CI checks in 8c8de86).

### BUG-003: Connected count includes the local computer

- Severity: S3
- Steps: open native Workspace with no paired peers.
- Expected: zero paired and connected computers.
- Actual: summary claims one paired computer.
- Fix: show the actual connected peer count.
- Status: fixed in 1091ac5 (native CI checks in 8c8de86).

### BUG-004: Missing displays leave stale routing eligible

- Severity: S2
- Steps: route to a peer; make the source display backend return no monitors;
  refresh displays and poll permissions.
- Expected: release input and suspend routing while retaining the saved layout.
- Actual: refresh returned an error before releasing ownership.
- Fix: gate input on usable displays independently of permission polling;
  restore routing only after valid displays return. Regression passes.
- Status: fixed in the session/clipboard hardening batch.

### BUG-005: Shutdown retains connected status and permits new connections

- Severity: S2
- Steps: pair two test services, shut one down, inspect its snapshot and attempt
  a manual connection.
- Expected: disconnected status, no new control sessions or live reader tasks.
- Actual: the new regression fails on retained connected status; connection
  admission had no stop check and early task exit bypassed reader cleanup.
- Fix: stop admission, clear peer state and channels, and run task cleanup even
  if shutdown happened before the control task was scheduled.
- Status: fixed in the session/clipboard hardening batch; regression passes.

### BUG-006: Reused transfer ID disrupts pending consent

- Severity: S2
- Steps: pair a peer, send an offer, then send another authenticated data offer
  with the same ID and a different filename while the first awaits approval.
- Expected: reject the duplicate and keep the original offer/decision intact.
- Actual: the regression fails; duplicate state/decision replacement disrupts
  the first transfer and returns a consent response to the duplicate session.
- Fix: reserve incoming IDs atomically for the full operation and reject IDs
  still present in transfer history. Cleanup releases reservations on every exit.
- Status: fixed in the session/clipboard hardening batch; original transfer
  completes byte-for-byte after duplicate rejection in the regression.

### BUG-007: Mac file clipboard round-trip returns no files in CI

- Severity: S2
- Flow: explicit native file clipboard bridge
- Build: 6dcbb2a; both macOS 14 ARM runner jobs (one builds the Intel package).
- Steps: run the parallel native Rust suite; write two existing Unicode files
  to a unique pasteboard and read them using the native adapter.
- Expected: two paths referring to the original file identities.
- Actual: successful write followed by zero read-back paths.
- Evidence: run 37882551696 failed the same assertion in both Mac jobs. The
  earlier CI run and 50 isolated / 12 complete local repetitions had passed.
- Change: explicitly link AppKit's file-URL-reading constant, provide the
  documented file-URL-only option, reject unavailable pasteboards, and retain
  item-count diagnostics. The exact reason for the previous empty read has not
  been independently isolated.
- Status: regression now passes after 4b016e8; both Mac jobs in run 37883109120
  pass the full suite and twenty repeated isolated checks each. Physical Finder
  and cross-application validation remain required.

### BUG-008: Accessibility switch enabled but the debug app remains untrusted

- Severity: S2
- Flow: Mac native permission setup / packaging
- Steps: grant Accessibility to the local app, remove/re-add its entry, and
  restart the bundle. Input Monitoring becomes available, but injection remains
  unavailable despite the Accessibility switch being enabled.
- Evidence: the app-specific macOS TCC log reports a code-requirement mismatch.
  Strict bundle validation reports missing sealed resources. The earlier ARM
  installer has the same signature defect; the Intel installer is unsigned.
- Cause: no Mac signing identity was configured; the local `--no-sign` build
  also bypassed bundle signing. A bare linker signature does not seal the bundle.
- Change: explicitly configure Tauri's ad hoc identity `-`. The corrected local
  bundle passes strict recursive verification, binds Info.plist/resources and
  uses `com.itskorrah.hoppingcursor` as its signing identifier. This does not
  establish a verified publisher, notarization, or stable grants across rebuilds.
- Status: corrected local bundle verifies strictly. After refreshing both privacy
  entries and completing Keychain access, native diagnostics confirm capture,
  injection and clipboard access Available. Public identity is unchanged;
  Pause/Resume and Bring input here pass native IPC smoke checks. CI now verifies
  the actual app inside each finished DMG; its guard rejects the earlier invalid
  package and detaches on failure. Physical peer control remains untested.
- Guidance: [Tauri ad hoc signing](https://v2.tauri.app/distribute/sign/macos/#ad-hoc-signing)
  and [Apple on TCC signing identity](https://developer.apple.com/forums/thread/730043).

## To check on physical hardware

OS permissions, hotplug/dock identities, native input, pasteboards, secure desktop,
long-duration reliability and latency. These are verification gaps, not invented
bug reproductions. See docs/PLATFORM_TESTING.md.

### BUG-009: First remote monitor follows enumeration order instead of physical geometry

- Severity: S2; flow F02; reported on the physical three-screen Mac/Windows setup.
- Expected: the Mac's right edge enters the Windows primary display in the middle.
- Actual: it enters the far-right display; both PC tiles had identical labels.
- Reproduction: reversed native monitor enumeration puts the primary second;
  the new topology regression fails against 0.1.0.
- Change: preserve native bounds for new monitor groups, retain existing user
  placements, label display order and primary status, and provide an explicit
  OS-order repair action for existing layouts.
- Status: automated regressions pass; exact physical arrangement requires retest.

### BUG-010: Mac input remains visible/active while controlling Windows

- Severity: S1; flow F02; user reports movement and clicks affecting both computers.
- Findings: the native tap is active, but source cursor state was not managed.
  Unmapped keys also bypassed local suppression. The exact cause of the reported
  mapped-click duplication has not been isolated in a new physical trial.
- Change: use an active Mac session filter, private marked injection source with
  local-event suppression disabled, balanced cursor hiding and event-free parking;
  suppress unsupported source keys while a remote lease is active. Restore cursor
  state on release, leave, pause, session failure, disconnect and capture shutdown.
  Windows restores its previous shared cursor shape without changing cursor themes.
- Evidence: native direct-callback tests discard right clicks and unmapped keys
  without posting test events into the session; injected events bypass capture.
  Encrypted integration checks verify callback consumption and recovery state.
- Status: physical duplicate-click/cursor verification remains open. Sharing
  is paused until the updated apps and arrangements are ready for a controlled retest.

### BUG-011: Windows left edge wraps to its other local monitor

- Severity: S2; flow F02; user reports failure to enter the Mac from Windows.
- Reproduction: swapping local monitor placements makes the engine consume the
  primary display's native left edge and warp to the local secondary display.
- Change: leave physical local-to-local motion to the native OS. Route only
  cross-computer edges when local input owns the cursor. Both computers need
  matching saved arrangements for bidirectional routing.
- Status: regression and encrypted Windows-primary-to-Mac keyboard/button
  routing tests pass. The updated cross-built 0.1.1 Windows test installer is
  available; actual installation and physical Windows retest remain pending.

### BUG-012: Crossing two remote monitors drops a held drag/key lease

- Severity: S2; flow F02; found by regression testing the two-monitor PC.
- Reproduction: a held button is released/re-pressed when switching displays
  belonging to the same remote computer.
- Cause: sending Leave and Enter on the same peer plus unconditional ReleaseAll.
- Change: keep the same peer lease and held state across monitor transitions.
- Status: failing-before/passing-after encrypted regression retained; hardware retest pending.

### BUG-013: Idle destination retains a cursor after return

- Severity: S2
- Evidence: user physical report on 0.1.1, Mac→Windows trial.
- Actual: Windows pointer stays at its edge after the cursor returns to Mac.
- Change: protocol 2 communicates cursor focus to peers; idle recipients hide
  through the native adapter. Windows uses a non-activating parking window,
  without overwriting global cursor themes. Recovery restores it.
- Status: automated ownership regression passes; physical Windows rendering and
  ordinary application cursor compatibility still need 0.1.2 acceptance.

### BUG-014: Windows physical control cannot enter Mac

- Severity: originally S1; basic handoff user-verified on 0.1.2, broader acceptance open.
- Evidence: user reports failure in the 0.1.1 three-screen trial.
- Finding: Windows capture derives motion from clipped pointer coordinates;
  layouts were independently saved. Neither is remotely established as the sole
  cause on this PC.
- Change: Raw Input deltas continue at the edge; explicit arrangement sharing
  lets the recipient apply the same geometry. Both app versions must be 0.1.2.
- Status: both-direction encrypted engine regressions and Windows strict compile
  pass. On 2026-10-10 the user confirms basic physical handoff now works in both
  directions. Remaining typing/motion/setup defects are BUG-018–020. This permits
  the owner-authorized alpha checkpoint merge; exhaustive native QA is pending.

### BUG-015: Physical takeover loses focus announcement

- Severity: S2
- Reproduction: encrypted integration test starts control, then moves the
  destination's physical mouse to reclaim it.
- Actual: urgent Leave advances the queue epoch and invalidates the earlier
  normal-queue CursorFocus, leaving the previous source visible.
- Fix: send focus after priority release; expose controller/destination state.
  Do not echo Leave to a new controller during Enter.
- Status: failing-before/passing-after regression; native counterpart pending.

### BUG-016: Pending copied-file offers can replace newer clipboard choices

- Severity: S2
- Scope: newly added opt-in automatic file clipboard.
- Tests: edit recipient clipboard during consent; answer old/new file offers
  out of order. Publish only a complete verified latest batch, and preserve
  newer recipient revisions. Hash verified local paths to prevent echo.
- Status: encrypted integration tests pass; physical Finder/Explorer checks pending.

### BUG-017: Keychain approval makes desktop startup appear frozen

- Severity: S2.
- Evidence: native process sample waits in SecItemCopyMatching while Tauri setup
  synchronously blocks its main thread. SecurityAgent is running.
- Change: background startup with readiness/error guards for every engine command.
  Window and Quit are available before protected storage completes; no identity
  regeneration or security bypass.
- Status: strict native desktop Clippy and IPC-mocked startup regression pass.
  The refreshed signed native Mac app opens with the existing identity and
  all three permissions Available. Actual pending OS-dialog behavior remains
  represented by a mock; no password/security bypass was performed.

### BUG-018: Windows E opens macOS emoji picker

- Severity: S2; keyboard correctness, priority first.
- Evidence: user physical 0.1.2 report on 2026-10-10; not independently reproduced
  by this Mac agent. Windows control → blank Mac editor → press unmodified E.
- Expected: letter E/e. Actual: emoji picker. Other typing and bidirectional
  handoff work. Mac injection preserves function flags; Fn/Globe+E is an OS
  shortcut. Unwanted Fn state is a hypothesis, not a confirmed root cause.
- 0.1.3: inherited Fn/Globe and receiver modifiers are stripped; synthetic flag
  regression passes and core checks for both Mac targets. Status: mitigated,
  pending rebuilt Mac receiver and physical E/shortcut acceptance. Do not log keys.
- Subsequent owner testing on 0.1.3: E/emoji now appears fixed. Preserve the fix;
  broad modifier/shortcut acceptance is still separate from this observation.

### BUG-022: Windows-to-Mac typing appears dependent on source text focus

- Severity: S3; user-reported with a text-field workaround.
- Owner reproduction on 0.1.3: keep the Mac field focused, start from Windows
  desktop/non-text context, cross and type; initial typing fails. Clicking
  Notepad/Chrome's address bar on Windows before crossing makes it succeed.
- Expected: remote typing depends on the destination field; source text focus
  is irrelevant. E/emoji now appears resolved in the same report.
- Local evidence: native tagged-input focus fixture passes without text focus
  on 0.1.3. A separate mouse-thread stall blocks its shared keyboard pump for
  953.7 ms, and the remote lease is not queued before native handoff publishes
  the owner. These weaknesses are confirmed; causation of the physical
  focus-dependent failure remains unconfirmed.
- 0.1.4: dedicated Windows keyboard pump; atomic Enter/held-state publication;
  aggregate capture/forward/receive/apply diagnostics. Failing-before/passing-after
  regressions, four native probes and repeated encrypted handoffs pass.
- Status: confirmed local weaknesses fixed; **physical BUG-022 acceptance pending**.
  See docs/TEST_0.1.4.md. No forced source focus or injection-filter relaxation.

### BUG-019: Windows mouse movement on Mac feels clunky

- Severity: S2; user-experience defect.
- Evidence: user's 0.1.2 directional comparison; Mac→Windows smooth, Windows→Mac
  clunky. No measured latency or independent native reproduction yet.
- Expected: consistent precise movement. Actual: uneven feel on Windows source.
- To check: raw-count DPI/gain, logical/physical scaling, cadence, queues and
  injection. Do not treat sensitivity or smoothing as an established fix.
- 0.1.3: remote Windows motion uses native processed screen displacement from
  a monitor-center parking anchor. Raw Input retains local edge detection.
  Consecutive pending positions coalesce without crossing discrete barriers.
  The 8,000-event encrypted burst fails on the original queue and passes here;
  real Windows hook/parking/restoration probe passes. Status: changed and
  regression-tested; directional feel, source visibility and drags still require
  physical acceptance. Mac source motion is unchanged.

### BUG-020: Sharing setup feels clunky and readiness is unclear

- Severity: S3; user clarified that this was a general clunky impression.
- Evidence: exact action sequence/connection fault is not established. Windows
  native 0.1.2 was observed saying Workspace ready with zero connected computers.
- Expected: approved, correctly arranged peers become ready predictably and show
  the exact blocker if sharing is paused, not paired or lacks permissions.
- Actual: readiness was unclear and needed experimentation.
- 0.1.3: explicit capture-start state cannot be resurrected by refresh after a
  failed hook installation. Capture retry, setup blocker status, serialized
  polling/stale-response protection and in-window Quit were added. Failed-start
  recovery and browser blocker tests pass. Status: concrete readiness issues
  fixed; complete physical startup/reconnect acceptance pending. Deliberate pause
  and pairing/layout consent remain explicit.

### BUG-021: README preview leaves white space below the sidebar

- Severity: S4; screenshot presentation.
- Evidence: user's old browser preview screenshot extends below the sidebar.
  CSS fixes the sidebar to the viewport; the current native window capture fills
  the visible height and shows actual connected devices.
- Fix: replace docs/images/workspace.jpg with an unedited native Mac 0.1.2 window
  screenshot, and label its source/date in README. No CSS or input code changed.
- Status: visually verified on 2026-10-10; documentation fix in this handoff commit.
