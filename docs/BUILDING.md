# Building test installers

Installers are development packages. Keep the current app's configuration and
protected identity when updating. Fully quit the engine before running an
installer; closing its window leaves the tray engine running.

## Windows x64 on Windows

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
`target/x86_64-pc-windows-msvc/release/bundle/nsis/`. For 0.1.1, its filename is
`The Hopping Cursor_0.1.1_x64-setup.exe`. Run that installer after quitting the
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
