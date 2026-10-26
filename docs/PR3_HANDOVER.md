# PR #3 handover — 0.1.7-pr3.1

Development alpha. PR stays draft until physical acceptance passes. Preserve
existing identity, pairing, settings, layout, profiles and permissions.

## 1. Windows installation

Use `The-Hopping-Cursor-0.1.7-pr3.1-windows-x64-test.zip`. Its accompanying
`BUILD_PROVENANCE.json` pins the source commit; `SHA256SUMS.txt` identifies the
ZIP, installer and packaged app. This is an unsigned local test package.

1. Fully quit the old app from its tray; closing the window leaves it running.
2. Extract the ZIP. Run `The Hopping Cursor_0.1.7-pr3.1_x64-setup.exe`, keeping
   normal Windows security protections and existing configuration.
3. Reopen; confirm **0.1.7-pr3.1**. Do not uninstall/reset pairing or identities.
4. Leave sharing paused while preparing Mac; do not run duplicate engines.

Two optional foreground-dependent Windows keyboard probes need an interactive
rerun; their exact commands and observed setup failure are in
[TEST_PR3.md](TEST_PR3.md). They were not counted as passing.

This Windows session builds and inspects the installer. Installation over the
live configuration and physical input remain user steps.

## 2. Copy this prompt to Mac Codex

Supply `BUILD_PROVENANCE.json` from the Windows ZIP alongside the prompt.

> Prepare The Hopping Cursor PR #3 on my Mac for a Windows–Mac physical test.
> Read AGENTS.md, README.md, PRODUCT_STATUS.md, docs/TEST_PR3.md,
> docs/SHORTCUT_ACCEPTANCE.md and docs/BUILDING.md. Windows uses 0.1.7-pr3.1.
> Fetch origin fix/win-mac-alt-tab-bridge-20261019 and use an isolated checkout
> at source_commit from the supplied BUILD_PROVENANCE.json. Preserve all existing
> local work. Verify the SHA and version before building; do not choose main or
> an arbitrary revision. If the provenance is unavailable, ask for source_commit.
> Use the existing Mac toolchain and stable local signing override/certificate.
> The override may be in the original checkout's ignored .local directory; use
> its absolute path from the isolated checkout. Do not change authentication,
> signing identity, OS security policy, protected identity, pairing or settings.
> Never print keys or configuration contents. Run npm ci and
> node scripts/verify-local.mjs, then native desktop Clippy:
> cargo clippy --locked -p hopping-cursor-desktop --all-targets -- -D warnings.
> Run normal native Mac tests, including isolated pasteboards. Cross-check both
> Mac architectures where the installed SDK/targets allow it; compilation does
> not establish runtime acceptance. Never replace my live clipboard for a test.
> With the sharing app fully quit, run the documented serial opt-in cursor probes
> from docs/TEST_0.1.6.md if supported. They briefly hide/warp the pointer and
> restore it; do not describe them as physical crossing evidence.
> Build the signed Mac debug app using the existing signing override and the
> Tauri command in docs/BUILDING.md. Verify with codesign --verify --deep --strict.
> Preserve a rollback copy, fully quit the old app and install in the same app
> location. Verify installed signature/version, unchanged public identity and
> configuration fingerprints, and capture/injection permissions without exporting
> private state. Keep existing pairing, layout, profiles and normal protections.
> Do not use GitHub Actions, buy minutes, merge or release. Report actual results
> and blocked checks, then leave the app ready for my physical acceptance test.

## 3. Physical test — you operate both machines

Use a private LAN, harmless editors, the existing accepted layout and a local
recovery mouse/keyboard. Both apps should show **0.1.7-pr3.1**; resume both.

| Test | Required observation |
| --- | --- |
| Windows controls Mac; Left Alt+Tab | Mac switcher changes apps; Windows switcher stays closed. |
| Hold Alt; Tab five times; Shift+Tab | Forward/reverse cycling; no stuck Command/Option/Shift after release. |
| Return to Windows | Windows Alt+Tab works normally. |
| Disable global toggle, then per-Mac toggle separately | Translation follows each setting and resumes when enabled. |
| Normal typing; applicable AltGr/dead-key/IME layout | No new typing/modifier regression; record layout and modifier-map setting. |
| Hold chord; emergency Ctrl+Alt+Escape, Pause, edge return, Quit | Each action restores local control and releases injected keys. |
| Network disconnect during chord; reconnect | Local control recovers; no stuck keys; pairing remains. |
| At least 15 minutes of typing, switching, scrolling and screen hops | No duplicate input, stranded cursor, stuck keys or recovery failures. |

Also follow sleep/wake, permissions and display cases in
[SHORTCUT_ACCEPTANCE.md](SHORTCUT_ACCEPTANCE.md). Secure desktops must release
control safely; UAC/login injection is unsupported. Unavailable Mac hardware
remains explicitly untested. Idle Windows keyboard `SharedKey` translation is a
documented limitation outside this active-source shortcut acceptance.

## 4. Report back

```text
Windows/Mac app versions:
Source commit from BUILD_PROVENANCE.json:
Mac architecture/macOS version; keyboard layout; modifier mapping on/off:
Pass/fail: Alt+Tab / repeat+reverse / local Windows / toggles / typing
Pass/fail: emergency / Pause / Quit / edge return / disconnect+reconnect
15-minute trial duration/result:
Any extra cursor, stuck key, failure and recovery action:
```

Do not include typed content, key logs, configuration files or unreviewed
diagnostic exports. The next development session fixes observed failures or
records acceptance and requests merge approval. This handover does not merge.
