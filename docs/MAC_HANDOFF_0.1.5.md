# Mac receiver handoff — 0.1.5

The Windows owner reports that basic sharing feels good, but scrolling with the
Windows mouse on Mac failed. Windows-local tests cannot execute Quartz. The Mac
must be rebuilt from updated main for the receiver fix to take effect.

Give Mac Codex this prompt:

> Pull GitHub main for The Hopping Cursor and read AGENTS.md, README.md,
> docs/TEST_0.1.5.md and docs/WINDOWS_HANDOFF.md. Build and install Mac 0.1.5 using
> the existing stable local signing configuration. Preserve the protected identity,
> PC pairing, settings and permission setup. Run native Mac scroll constructor,
> ABI and fractional-field tests, core checks and isolated pasteboard tests.
> Verify the signed installed bundle and native permissions. Then compare Windows
> mouse scrolling on the same Mac document with Natural scrolling on and off,
> tiny movements and the first wheel after crossing. Use aggregate diagnostics
> for any remaining failure. Do not infer physical acceptance from Windows compile
> checks, alter OS preferences automatically, relax injection filtering or reset
> pairing to solve a scroll defect.

Checks after pulling and preparing the existing Mac toolchain:

```sh
cargo test --locked -p hop-core
cargo test --locked -p hop-core platform::macos::tests -- --nocapture
cargo clippy --locked -p hop-core -p hopping-cursor --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
npm run build
npm run test:ui
```

The Mac constructors create/inspect events without posting them. Use the existing
isolated pasteboard procedure; do not indiscriminately run ignored clipboard or
discovery tests. If a constructor assertion fails, investigate native field/unit
behavior before changing the adapter.

The first wheel argument is fixed; additional deltas remain variadic. Mac policy
is read through CoreFoundation and cached by background display refresh; callbacks
read an atomic value. Incoming events retain fractional fixed/point values and
use the destination cursor position. Application delivery, immediate-post timing,
hardware wheel/trackpad behavior and third-party reversers need this Mac.

No Mac access or installation was performed from the Windows host.
