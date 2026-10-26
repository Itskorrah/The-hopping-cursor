# PR #3 local preparation — 2026-10-26

Version **0.1.7-pr3.1**, tested on Windows x64 with Node 24.1.0, Rust/Cargo
1.99.0 and the existing session-local portable LLVM/MSVC SDK setup. No physical
Windows/Mac acceptance or Mac execution is claimed.

The delivery's BUILD_PROVENANCE.json pins the tested source commit and package
hashes. See [handover](PR3_HANDOVER.md) for the next-machine procedure.

## Fresh local checks

| Command/check | Observed result |
| --- | --- |
| `npm ci` | Passed; audited 143 packages, zero reported vulnerabilities. |
| `node scripts/verify-local.mjs` | All seven stages passed, including five verifier smoke tests. |
| `cargo test --locked` (inside verifier) | 38 unit/core and 39 encrypted integration tests passed. Opt-in probes, real discovery and timed soak stayed ignored. |
| Formatting and strict core/CLI Clippy | Passed inside verifier. |
| TypeScript, production frontend build, Playwright | Passed; 19 browser tests, including shortcut preferences/accessibility. Preview and mocked IPC are not native evidence. |
| `cargo clippy --locked -p hopping-cursor-desktop --all-targets -- -D warnings` | Passed natively on Windows. |
| Strict core/CLI Clippy, `--all-targets --target aarch64-apple-darwin` | Passed compilation only. |
| Strict core/CLI Clippy, `--all-targets --target x86_64-apple-darwin` | Passed compilation only. |
| `cargo test --locked -p hop-core --test native_windows -- --ignored --test-threads=1` | One native capture/parking/restore probe passed. |
| Serial opt-in `platform::windows::tests::` | Two passed: parked-motion reclaim and synthetic wheel-hook/injection filtering. Two failed their foreground-fixture precondition; see below. |
| `python scripts/generate-notices.py` with toolchain activated | Generated inventory matches the tracked dependency/font notices. |

Logs are retained under the isolated checkout's ignored `.local/pr3-handover/`.
The extra regression exercises active shortcut capture, saturation of the normal
queue, emergency priority Leave, ownership clearing and epoch invalidation.
It passed in the full suite; it uses a headless OS adapter.

## Foreground-dependent Windows probes — not passed

`keyboard_delivery_does_not_wait_for_mouse_thread_work` and
`native_keyboard_delivery_without_a_focused_text_field` stopped at the
`GetForegroundWindow() == fixture` assertion, immediately after
`SetForegroundWindow(fixture)`. Windows retained another foreground window.
The pressure/delivery assertions were not reached. No native adapter file was
changed by this preparation, and this result does not establish a shortcut defect.

Do not bypass Windows foreground restrictions or change production capture to
make these fixtures pass. Rerun from a normal interactive Windows terminal, with
other sharing engines quit and the test fixture allowed foreground input:

```powershell
# First activate this PC's existing portable toolchain as in BUILDING.md.
cargo test --locked -p hop-core --lib keyboard_delivery_does_not_wait_for_mouse_thread_work -- --ignored --test-threads=1
cargo test --locked -p hop-core --lib native_keyboard_delivery_without_a_focused_text_field -- --ignored --test-threads=1
```

These checks remain open alongside physical keyboard acceptance. The clipboard-
replacing Windows probe was deliberately not run on the user's live clipboard.

## Windows package

Native `npm run tauri -w apps/desktop -- build --target x86_64-pc-windows-msvc
--bundles nsis -- --locked` passed. The NSIS installer is 3,585,703 bytes.
7-Zip archive integrity and extraction passed. The packaged app has x64 PE
architecture, ProductVersion/FileVersion **0.1.7-pr3.1** and numeric version
0.1.7.0. Authenticode reports NotSigned, as expected for this development package.
The extracted application matches the compiled executable except Tauri's exact
three-byte UNK → NSS bundle marker; hashes are in delivery provenance.

The existing portable build emitted missing-CRT-PDB debug-symbol warnings and
a Cargo lib/bin PDB filename-collision warning. Compilation, linking and NSIS
packaging completed successfully; no debug-symbol completeness is claimed.

Installer SHA-256:
`10baa4ae6fc952ca5d43adce3de2cde73b911f9bfb128640474b36264bba11f2`.
No installation over the user's live configuration or application-level physical
acceptance is claimed. The final delivery pins the committed source and hashes.

Physical acceptance is pending: Windows shell interception, Mac Command+Tab,
repeat/reverse cleanup, toggles, AltGr/IME, recovery and the 15-minute trial.
Keep the PR in draft. Local checks cannot replace these observations.

Hosted Actions are unavailable; local checks are the execution route. Skipped
CI is not a pass. Linux sanitizer fuzzing and real Intel/ARM Mac runtime results
are not supplied by this Windows session. Protocol/codec code is unchanged.
