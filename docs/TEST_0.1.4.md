# Keyboard handoff pass — 0.1.4

Development alpha based on 0.1.3 commit `55bbdf1`. Protocol remains 2. Pairing,
protected identities, settings and the Mac modifier flag fix are preserved.

## User feedback and investigation

The owner reports that E no longer opens the Mac emoji picker on 0.1.3.
Windows-to-Mac typing initially fails from the Windows desktop/non-text context;
first clicking a Windows text field makes it work. This workaround is evidence,
not a confirmed diagnosis. The Mac destination's focused field should determine
where remote typing goes; Windows does not need a text field focused.

A test-owned native Windows fixture receives all 64 tagged synthetic key events
with null focus, a non-text window, an edit field, then null focus again, even on
the original capture design. This does **not** reproduce the physical report.
The real Windows desktop/Chrome/Notepad versus Mac-field comparison remains open.

Two local defects were demonstrated with failing-before/passing-after regressions:

- Windows keyboard and mouse capture shared one message pump. In an isolated
  mouse-thread stall probe, keyboard delivery waited 953.7 ms before the change.
  Keyboard capture now has its own dedicated thread/message pump. The pressure
  probe passes its 250 ms bound, including ten repeats. That bound is a test
  threshold, not measured physical typing latency. Shared service-state contention
  and other applications' hooks are still possible; this is not proof of the
  physical report's root cause.
- A new active remote owner was visible before its Enter message was queued.
  Enter and the pre-handoff held-key/button state now queue under the ownership
  lock before exposing that owner, for both shortcuts and edge handoffs. This
  prevents a concurrent first keyboard packet preceding its lease. Taking the
  held snapshot in the same barrier also avoids duplicating a newly captured key.
  Failed partial entry invalidates the new peer's queue with priority Leave.

The event source stays focus-independent. Production rejects injected keyboard
events; only ignored test binaries admit a tagged synthetic packet into the
isolated fixture. No source foreground/text-field workaround is added.

## Diagnostics

Both platforms show/export cumulative keyboard counts: captured, forwarded,
received, applied and rejected. Windows additionally reports physical hook
events, ignored injection, unsupported keys, maximum callback duration, callbacks
over 100 ms, and focus/caret presence at the last captured physical event.

These contain no key identities, text, application names or event timeline.
Counts reset on process restart; held-key transfers/shared keyboards contribute
to forwarding totals, so totals need not be equal across every stage. A caret
observation is only a Win32 hint, not an authoritative text-field classification.
Forwarded counts mean accepted by the outbound queue, not confirmed delivery.
Applied counts mean successful injection requests, not proof that an application
accepted text. Callback duration excludes OS delivery wait and is not end-to-end latency.

## Verified on Windows

- Core/CLI suite: **62 passed** (24 unit/security, 38 encrypted integration).
  Six default ignores: real clipboard replacement, LAN discovery and four
  interactive Windows probes. The four native pointer/focus/pressure probes
  were run separately and passed, including production injection rejection; keyboard pressure was repeated ten times.
- New encrypted regression: 32 immediate first-key handoffs plus 32 shared-key
  handoffs after simulated receiver physical takeover; 128 key packets applied
  exactly once to the in-memory receiver. Held Shift and a first new key retain correct order and releases.
- Strict Clippy core/CLI and desktop, formatting, frontend production build:
  passed. **17 Playwright tests passed**, including diagnostics with absent
  source focus/caret and non-Windows stats, runtime checks and accessibility.
- Core compile checks for ARM and Intel Mac pass. These are not Mac runtime tests.
- Windows x64 NSIS build, package integrity/version/source comparison and
  installation evidence are included with the test package's provenance.

Native UI automation could read the existing app but clicking Bring input here
failed with `coordinate input geometry is unavailable`. The old Windows process
was stopped before isolated native tests; disconnect cleanup remains covered by
the encrypted tests. Native visual/click acceptance is not claimed.

## Physical acceptance: same Mac field, different Windows contexts

1. Update Windows to 0.1.4. For receiver diagnostics, update Mac from main using
   its existing stable signing setup. Version 0.1.3 peers remain protocol-compatible.
   Preserve existing identity/settings; compare full safety codes if pairing is needed.
2. Keep one blank Mac editor field focused. Test starting from Windows desktop,
   a non-text window, Notepad, then Chrome's address bar. Cross using the Windows
   mouse and type immediately, without clicking any source text field first.
3. Repeat after Mac physical mouse takeover (shared-keyboard path), in both
   directions, with E, ordinary letters, Shift, Ctrl/Command shortcuts and key releases.
4. Verify Windows receives no duplicate typing while Mac owns the cursor. Test
   held modifiers during crossing, return crossing, Pause/Resume, reconnect and
   Ctrl+Alt+Escape. Avoid admin/UAC/secure desktop during this ordinary-desktop check.
5. If either context still fails, export Diagnostics from **both** apps after
   the failing attempt, then after the successful workaround in the same process.
   Captured versus forwarded versus received/applied counts identify the stage;
   injected-ignore counts help distinguish physical input from software injection.

Hosted runners remain subject to the account's existing billing/spending-limit
block. No billing changes, physical Mac success, full bug closure or seamlessness
benchmark is claimed. Current source and asset hashes are in release provenance.
