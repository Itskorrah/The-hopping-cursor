# Building test installers

Installers are development packages. Keep the current app's configuration and
protected identity when updating. Fully quit the engine before running an
installer; closing its window leaves the tray engine running.

## Local verification without GitHub Actions

Use the same checkout/commit you intend to ship. Install Node.js 22+ and
stable Rust (including rustfmt and Clippy), then run `npm ci` from the repository
root. Install Playwright Chromium separately if it is not already available,
for example `npx playwright install chromium`, and install the normal operating
system prerequisites described below. The verifier never installs anything,
changes credentials, enables billing or alters application configuration.

```sh
node scripts/verify-local.mjs --dry-run
node scripts/verify-local.mjs
# On Windows or macOS only; include a native installer package build:
node scripts/verify-local.mjs --native
```

The verifier runs its own Node smoke tests, `cargo fmt --check`, strict
Clippy, Cargo unit/integration tests, TypeScript type checking, frontend build
and Playwright tests. It continues after an individual failed check, prints a
failure summary and exits nonzero if **any** check failed or could not start.
`--dry-run` lists commands without validating their execution.

This is a **developer build/automation gate**, not evidence of real
Windows keyboard hook interception, macOS Command+Tab, pairing security or
two-machine cursor/file acceptance. Those require the physical checklist in
[platform testing](PLATFORM_TESTING.md). The Windows-to-Mac Alt+Tab physical
acceptance plan is tracked in [PR #3](https://github.com/Itskorrah/The-hopping-cursor/pull/3).

## Windows x64 on Windows

On this Windows development PC, a machine-wide Build Tools install was rejected
by automatic approval review. A user-local portable setup was then built and
verified: Rust 1.99.0 MSVC, LLVM 22.1.8, xwin 0.10.0, CRT 14.44.17.14 and SDK
10.0.26100. Its ignored files are in `.local/setup`; no registry or machine PATH
changes are required. Activate it in each new PowerShell session:

```powershell
. ./scripts/use-windows-portable-toolchain.ps1
```

For reproduction, download LLVM's official `LLVM-22.1.8-win64.exe` and extract
it with 7-Zip into `.local/setup/llvm` (without running its installer). LLVM's
release SHA-256 is `16e5709785fef73c854646241c4a92c5cd574318d1b33c63330dd7721903e55c`.
Download xwin's official Windows 0.10.0 archive, verify its released checksum,
and extract it under `.local/setup`. Microsoft's CRT/SDK license applies; review
it before running xwin with `--accept-license`:

```powershell
& .local/setup/xwin-0.10.0-x86_64-pc-windows-msvc/xwin.exe --accept-license --cache-dir .local/setup/xwin-cache splat --output .local/setup/winsysroot --use-winsysroot-style --preserve-ms-arch-notation --disable-symlinks
```

Sources: [LLVM release](https://github.com/llvm/llvm-project/releases/tag/llvmorg-22.1.8),
[xwin release and checksum](https://github.com/Jake-Shadle/xwin/releases/tag/0.10.0),
[xwin's Windows clang-cl layout guidance](https://github.com/Jake-Shadle/xwin#splat-options).
The activation script selects the available CRT/SDK versions. Other hosts should
use Tauri's normal Visual Studio prerequisites where available.

Install Node.js 22+, Git, stable Rust with the MSVC target, and Microsoft's C++
Build Tools with **Desktop development with C++**. WebView2 is also required.
Follow [Tauri's Windows prerequisites](https://v2.tauri.app/start/prerequisites/#windows).

Clone into a new directory, or download/extract the source ZIP for the intended
commit while signed into the private GitHub repository. Do not overwrite an
existing checkout. From the repository root in PowerShell:

```powershell
npm.cmd ci
cargo test --locked -p hop-core
cargo clippy --locked -p hop-core -p hopping-cursor --all-targets -- -D warnings
npm.cmd run tauri -w apps/desktop -- build --target x86_64-pc-windows-msvc --bundles nsis -- --locked
```

Stop if a command fails. The installer is under
`target/x86_64-pc-windows-msvc/release/bundle/nsis/`. For 0.1.2, its filename is
`The Hopping Cursor_0.1.2_x64-setup.exe`. Run that installer after quitting the
old app, reopen, and check the version shown in the sidebar. Existing pairings
should remain; never delete the configuration to solve an update problem.

## Windows x64 from this Mac

Native Windows builds remain preferred. If hosted runners are unavailable,
[Tauri documents an MSVC/NSIS cross-build](https://v2.tauri.app/distribute/windows-installer/#build-windows-apps-on-linux-and-macos).
The `cargo-xwin` runner uses Microsoft SDK/build libraries and automatically
accepts their [licence](https://visualstudio.microsoft.com/license-terms/mlt031519/)
when used. Obtain the owner's approval before that step.

```sh
brew install llvm lld nsis
cargo install --locked cargo-xwin --version 0.23.1
rustup target add x86_64-pc-windows-msvc
export PATH="/opt/homebrew/opt/llvm/bin:/opt/homebrew/opt/lld/bin:$PATH"
export XWIN_ARCH=x86_64
export XWIN_CACHE_DIR="$HOME/.cache/hopping-cursor-xwin"
npm run tauri -w apps/desktop -- build --runner cargo-xwin --target x86_64-pc-windows-msvc --bundles nsis -- --locked
```

Use an SDK cache path without spaces: this setup rejected a cache inside the
project's spaced path before compilation. The project path itself can stay put.
Check the resulting installer's archive integrity, extracted application hash,
PE architecture and version resources before uploading. Tauri changes its bundle
marker from `UNK` to `NSS` inside the packaged executable, then restores the
original build; allow only that exact mutation when comparing both binaries.
Include `LICENSE` and
the dependency notices with a test download. Cross-building cannot execute the
Windows native tests or prove that installation/input hooks work on Windows.
Use the [physical acceptance checklist](PLATFORM_TESTING.md) on the actual PC.

## Local Mac debug app

The default configuration uses ad hoc signing. For stable local permissions,
use an existing Apple Development signing identity via an ignored override,
keeping that identity out of the repository. From the repository root:

```sh
npm run tauri -w apps/desktop -- build --debug --bundles app --config "$PWD/.local/macos-dev-signing.json"
codesign --verify --deep --strict "target/debug/bundle/macos/The Hopping Cursor.app"
```

The override contains `bundle.macOS.signingIdentity` set to the local certificate.
Install the app in `~/Applications` after quitting the old copy, verify its
signature again, and check capture/injection in Diagnostics. Refresh only this
app's Accessibility/Input Monitoring entries if its signing identity changes.
Apple Development signing is for this local setup; production Developer ID
signing, notarization and verified updates remain unfinished.
