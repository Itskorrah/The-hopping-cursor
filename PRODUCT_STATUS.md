# Product status — PR #3 Windows → Mac shortcuts

Test version: **0.1.7-pr3.1**. Development alpha; not merged or physically accepted.

## Implemented

- Active Windows source → Mac Left Alt+Tab → Command+Tab, repeat cycling,
  Shift reverse, modifier restoration and Right Alt/Control exclusions.
- Global preference and per-Mac opt-out; persisted defaults and unpair cleanup.
  The new preference defaults to enabled when configuration migrates.
- Ownership-locked transition ordering; priority emergency Leave, stale-queue
  invalidation, injected-event filtering and pressed-key cleanup retained.
- The October pilot's saturated-queue/emergency-capture regression and formatting.
- Current main integrated, retaining its README and Mac 0.1.6 cursor work.

## Verification and handover

See [fresh local evidence](docs/TEST_PR3.md) and [handover](docs/PR3_HANDOVER.md).
Actions minutes are unavailable. Local Windows/Mac checks and packages are the
execution route; skipped hosted checks are never reported as passing.
The standard verifier does not cover every native/installer or fuzz check;
extra checks and omissions are recorded in the report.

Physical shell interception, Mac Command+Tab delivery, modifier cleanup,
AltGr/IME and the 15-minute paired trial are **pending**. Keep PR #3 in draft
until observations are recorded and defects resolved.

## Explicit limitations

- The bridge applies when Windows actively owns remote Mac pointer/input.
  An idle Windows keyboard on the SharedKey route retains the legacy map;
  universal idle-keyboard shortcut translation is outside this PR's acceptance.
- Some AltGr layouts synthesize Ctrl; the older Control/Command map can affect
  typing. Physical international-layout compatibility remains unproven.
- Mac cursor hiding uses an undocumented WindowServer capability with fail-safe
  release; hardware/macOS-version acceptance is separate.
- Secure desktop input, native cross-edge file dragging, production signing,
  notarization and updates remain separate product work.

Next: install matching builds, run [physical acceptance](docs/SHORTCUT_ACCEPTANCE.md),
record results, then request merge approval. No Actions purchase is required.
