# Product status — Windows → Mac shortcut mapping

**Branch:** `fix/win-mac-alt-tab-bridge-20261019` based on development-alpha
`main` at `5e215e1` (2026-10-11). **Not released.**

## Implemented (in this review branch)

- Remote-target-aware Left Alt+Tab → Mac Command+Tab routing, held Alt/Tab
  cycling, Shift reverse, safe modifier restoration when interrupted by another
  key, and Right Alt/AltGr and Control exclusions.
- Global shortcut preference, per-connected-Mac opt-out, persisted backwards-
  compatible defaults and peer-override cleanup on unpair.
- Reset of in-memory shortcut state on input recovery, ownership return,
  release, settings change and session/display interruption; existing Leave
  and ReleaseAll clean up destination synthetic keys.
- Unit regression cases and a browser-preview preferences persistence case.
  These additions do **not** prove native shell suppression or CGEvent behaviour.
- Clear acceptance and privacy documentation; no new protocol version, paid
  infrastructure, privileged helper, remote telemetry or native permissions.

## Tested/accepted/released

- **Code inspection:** Windows/macOS input adapters, HID conversion, routing,
  crypto/control recovery, Tauri UI and workflows inspected.
- **Automated checks:** the fresh PR workflow [run 39131429829](https://github.com/Itskorrah/The-hopping-cursor/actions/runs/39131429829)
  failed before execution on 2026-10-19 (Sydney time): **all five jobs had
  zero steps and no assigned runner**. This is an Actions allocation/account
  blocker, not a Rust, TypeScript or native test result. Local Rust/Cargo
  toolchains and physical paired machines are unavailable in this session.
- **Physical Windows+Mac:** unavailable to this session, so **unverified**.
- **Remaining routing edge:** an *idle Windows keyboard* forwarded through the
  existing `SharedKey` path while a different computer owns the pointer still
  uses the legacy modifier map; the new chord bridge applies to the active
  Windows-source → Mac ownership lease only. Test and extend that path before
  describing shortcut translation as universal.
- **AltGr caveat:** some Windows layouts synthesise Left Ctrl when Right
  Alt is pressed. The new Alt+Tab chord excludes Right Alt, but the existing
  Ctrl-to-Command modifier feature may still alter international AltGr typing.
  This needs physical language-layout tests and a separate, carefully scoped
  fix; it is **not proven compatible**.
- **Release:** none made; existing 0.1.5/0.1.6 alpha artefacts remain separate.

## Blockers and ranked next work

1. **P0** Physically verify Windows shell *does not* consume Alt+Tab while Mac
   owns input; hold Alt across Tab cycles and prove complete key-up restoration.
2. **P0** Re-enable functional GitHub hosted runner allocation/CI without an
   unapproved billing change. Earlier runs completed without step execution;
   obtain fresh format, Clippy, Rust, UI and native Windows/macOS build results.
3. **P1** Test both modifier layouts and AltGr/IME/dead keys, UAC focus,
   sleep/wake, timeout, reconnect, emergency release and Mac CGEvent injection.
4. **P1** Retest duplicate Mac cursors and isolate/replace the undocumented
   WindowServer cursor facility with documented, version-gated recovery.
5. **P1** Run a 15+ minute mixed-display, scroll, clipboard consent and
   encrypted file transfer/collision/cancellation soak.
6. **P2** Improve native workspace polish, user-facing recovery diagnostics,
   installers, signing/notarisation and documented releases.

**Reproduction:** see [shortcut acceptance](docs/SHORTCUT_ACCEPTANCE.md),
[setup/architecture](README.md), [threat model](docs/SECURITY.md),
[building](docs/BUILDING.md) and [feature matrix](replica/features.csv).
