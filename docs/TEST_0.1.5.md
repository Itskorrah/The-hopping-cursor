# Scroll and reliability pass — 0.1.5

The owner reports that 0.1.4 feels good during a short test, but a Windows mouse
does not scroll on the Mac. Long physical acceptance is still outstanding.
The wire format remains protocol 2. Update both apps to 0.1.5 for scroll acceptance;
an older Mac receiver does not contain the native scroll fix.

## Changes and evidence

- The Mac FFI declared every wheel delta as variadic. Apple's declaration has a
  **fixed `int32_t wheel1` argument**, followed by variadic additional wheels.
  That distinction matters on Apple Silicon's calling convention. A typed ABI
  regression fails to compile with the old declaration and compiles with the
  corrected declaration. Mac-native constructor tests check both axes and
  fractional event fields without posting events. They are compile-checked here,
  **not executed on this Windows host**. The physical failure's complete cause
  cannot be confirmed until the rebuilt Mac receiver is tested.
- Wire scroll amounts retain fractional wheel steps: positive Y is wheel-forward
  (traditional up), positive X is Windows wheel-right. Mac source events are
  normalized, and the receiving Mac applies its own natural/traditional setting.
  OS preferences are read, never changed. The global macOS preference is cached
  during the existing two-second background refresh, outside the input callback.
  Third-party reversers and device-specific overrides need separate acceptance.
  Continuous Mac source events use a point-to-line conversion; native trackpad
  speed and inertia equivalence are not established.
- Mac fixed-point fields preserve fractional input. Integer line/point fields
  carry fractional remainder instead of truncating each packet. Windows likewise
  accumulates fractions smaller than one native wheel unit and skips zero-unit
  injection. Both adapters clear remainder on release; changing Mac direction
  also clears it. Regressions fail with previous truncation and pass with
  accumulation, including opposing movement and floating-point roundoff.
- Scroll, key and button events remain ordering barriers. Only consecutive
  absolute pointer positions in the same ownership epoch may coalesce.
  Encrypted tests send a wheel immediately after entry in both directions,
  followed by mixed keys/buttons/wheels, and verify exact order and no source
  scroll injection. Priority recovery still invalidates stale input.
- Diagnostics show aggregate scroll capture/queue/receive/injection/rejection
  counts, local scroll policy and control queue pressure. No wheel position,
  direction, typed content, key identity or event timeline is exported.
  Forwarded means queued; applied means the OS injection call succeeded.
  Neither establishes that a destination application scrolled.

References: [Apple scroll constructor](https://developer.apple.com/documentation/coregraphics/cgeventcreatescrollwheelevent?language=objc),
[Apple fixed-point field](https://developer.apple.com/documentation/coregraphics/cgeventfield/scrollwheeleventfixedptdeltaaxis1),
[Microsoft mouse packet](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-msllhookstruct).

## Review and verification

The review covered encrypted/pinned pairing, input validity and ownership gates,
injected-event filtering, shared keyboards, held-input cleanup, generation/epoch
replacement, emergency priority, fragmented reads, bulk isolation, file consent,
quotas, temporary cleanup and clipboard bounce/privacy handling. Existing
regressions exercise these paths. This is an engineering review, not an independent
security audit. No sensitivity smoothing or latency improvement is claimed
without a representative measurement.

Results and aggregate timing/resource measurements are recorded below. Native
Windows probes use test-owned fixtures and synthetic packets;
they do not establish physical Mac delivery. Native accessibility reads work,
but clicking failed with `coordinate input geometry is unavailable`, so native
click/visual acceptance is not claimed. Sharing was stopped before native probes.

| Check | Observed result |
| --- | --- |
| Windows core suite | 71 passed: 32 unit/security and 39 encrypted integration tests. Eight conditional/interactive tests remained ignored in the default suite. |
| Separate native Windows probes | Five passed, serially: keyboard delivery under mouse-thread pressure, source-focus independence, parked reclaim, signed wheel capture/injection filtering, and pointer parking/restore. Synthetic fixtures; no physical Mac delivery claim. |
| Browser/IPC preview suite | 18 passed, including aggregate scroll/policy/queue diagnostics and accessibility. Preview tests do not run native OS hooks. |
| Parser mutations | 1,000,000 deterministic bounded cases passed in 0.970 s using the existing plaintext fuzz target. No sanitizer or coverage guidance. |
| Static checks | Rust format and strict Windows core/CLI/desktop Clippy passed with all targets/features. Both ARM and Intel Mac core/CLI strict checks passed; compilation only. |
| Build/install | Frontend production build and Windows x64 NSIS build passed. Archive integrity, PE architecture, installed 0.1.5 metadata and packaged/installed binary equality verified. |
| Preserved state/startup | Protected identity and blob, pairing/permissions, settings, layout, profiles and hop shortcuts matched before/after. Native accessibility confirmed 0.1.5, ready workspace, connected paired peer and no browser-preview label. |
| Dependency review | Fresh npm and Cargo audits reported zero vulnerabilities. Existing Cargo maintenance advisories remain documented in [SECURITY.md](SECURITY.md); they were not suppressed. |
| Mac runtime/physical matrix | Pending. The Mac needs its own rebuild/install; this Windows update cannot change the Mac receiver. |

The installed Windows app is open. This is an unsigned development build. Hosted
CI status is reported separately in the GitHub release; the local checks above
are the evidence for this package.

### Sustained encrypted loopback

The 30-minute release-mode run passed **27,900 alternating handoffs**, **139
timing-gap recovery/reconnect cycles** and **100 verified 1 MiB file transfers**.
It checked 2,064,600 capture calls and 111,600 scroll events, with 55,800 at every
capture/queue/receive/injection stage per side, zero scroll rejections and zero
full-queue rejections. Discrete keys/buttons/wheels retained exact ordering and
held state was released. Peak normal control queue occupancy was 13 messages;
receiver event storage was cleared between cycles.

That long run used the instrumented control engine before the version bump,
still labelled 0.1.4. Later changes added Windows native fractional conversion,
the parser-test harness and test/version/format/documentation edits, with no
control-engine behavior changes. Its headless adapters never execute native
scroll conversion. A separate **final 0.1.5** 60-second release run passed 800
handoffs, four recoveries, eight verified transfers and 3,200 scroll events with
matching stages, zero rejections and peak queue occupancy 11.

| Loopback run | Capture call mean / max | Scroll capture-to-in-memory receiver mean, A / B | Scroll p95 bucket upper bound, A / B | Maximum, A / B |
| --- | --- | --- | --- | --- |
| Baseline 0.1.4, 60 s | 1.185 / 304 µs | 341.762 / 340.174 µs | 1,000 / 1,000 µs | 1,292 / 1,358 µs |
| Instrumented control engine, 30 min | 0.912 / 360 µs | 269.928 / 270.069 µs | 1,000 / 1,000 µs | 18,684 / 14,367 µs |
| Final 0.1.5, 60 s | 0.123 / 39 µs | 121.319 / 120.648 µs | 250 / 250 µs | 340 / 357 µs |

These are synthetic loopback timings, excluding native callbacks/injection,
physical network, hardware and application rendering. Percentiles are histogram
**bucket upper bounds**; means use per-observation whole microseconds. Background
work and sample duration differed. These observations do not establish an
optimization improvement or cross-computer cursor latency.

### Process resource samples

Samples used five-second intervals and pinned the intended process. CPU is the
average percentage of **one logical processor** over first-to-last samples.

| Process/window | Samples / sampled span | CPU | Private bytes min–max | Working-set bytes min–max | Threads / handles min–max |
| --- | --- | --- | --- | --- | --- |
| Installed connected 0.1.4 engine | 120 / 596.97 s | 0.395% | 8,650,752–8,683,520 | 37,085,184–37,601,280 | 29–30 / 409–411 |
| Installed connected 0.1.5 engine, after startup | 60 / 295.95 s | 1.758% | 7,069,696–7,196,672 | 29,016,064–29,253,632 | 31–32 / 413–418 |
| Synthetic sustained test, final tail only | 29 / 140.53 s | 0.233% | 5,423,104–5,955,584 | 13,266,944–13,795,328 | 11 / 204–206 |

Engine samples exclude WebView child processes. Activity and background load were
not controlled, startup/window lengths differ, and the synthetic sample covers
only the final tail of the run. They establish observed ranges, not a whole-app
memory result, leak absence or a comparative CPU improvement. Full synthetic
histograms and resource aggregates are in
[reliability-0.1.5.json](evidence/reliability-0.1.5.json); no user input or identity
data is included.

## Repeat the automated checks

In PowerShell on this PC, activate the existing portable toolchain first:

```powershell
. ./scripts/use-windows-portable-toolchain.ps1
cargo test --locked
cargo clippy --locked -p hop-core -p hopping-cursor --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
npm run build
npm run test:ui

# Synthetic encrypted loopback; no OS hooks or user clipboard. 5..7200 seconds.
$env:HOP_SOAK_SECONDS = '1800'
$env:HOP_SOAK_BULK = '1'
cargo test --locked --release -p hop-core --test reliability -- --ignored --nocapture

# Deterministic parser mutations. No sanitizer or coverage guidance.
cargo test --locked --release -p hop-core --features fuzzing --lib bounded_plaintext_mutation_campaign -- --ignored --nocapture
```

Quit sharing engines before these native probes; run serially. Do not run every
ignored test indiscriminately: clipboard and LAN checks have their own conditions.

```powershell
cargo test --locked -p hop-core --lib platform::windows::tests -- --ignored --test-threads=1 --nocapture
cargo test --locked -p hop-core --test native_windows -- --ignored --test-threads=1
```

## Physical scroll acceptance

1. Update Windows and Mac to **0.1.5**, preserving identity, pairing, settings and
   the Mac's stable signing/permission setup. See [Mac handoff](MAC_HANDOFF_0.1.5.md).
   Confirm capture/injection are available and the paired computer is connected.
2. On Mac open a harmless long document or browser page. Move the Windows mouse
   onto it and scroll immediately. Test one notch, small/rapid movements, both
   vertical directions, horizontal wheel/tilt if supported, and scrolling under
   the pointer in a second window. Windows must not scroll its background window.
3. Repeat with Mac Natural scrolling on, then off, allowing the next two-second
   refresh. Each setting should behave like the Mac's own mouse under that setting.
   Restore the user's preferred setting afterwards.
4. Repeat after a click, held Shift, return crossing, receiver mouse takeover,
   Pause/Resume, disconnect/reconnect and emergency release. Test Mac mouse/trackpad
   → Windows too, including slow sub-line motion. Check for stuck modifiers/buttons,
   doubled scroll and residual motion after release.
5. If it fails, export Diagnostics from both apps before/after a short isolated
   attempt in the same process. Compare changes in the five scroll counters.
   Source capture includes local scrolling; Mac receive/apply distinguish transport
   from an application ignoring injection. Review names/addresses in diagnostic
   messages before sharing exports.

## Broader acceptance and performance

Follow [PLATFORM_TESTING.md](PLATFORM_TESTING.md) for keyboard layouts/shortcuts,
all displays and mixed DPI, clipboard types, file consent/cancellation, sleep/wake,
network interruption, tray behavior and permission recovery. Repeat the
[same-destination-field typing comparison](TEST_0.1.4.md) from Windows desktop,
non-text windows, Chrome and Notepad. A short positive report is not a controlled
completion of that matrix.

Use a multi-hour physical session in both directions with ordinary editing,
small adjustments, wheel reversals, held drags and simultaneous file transfers.
Record device/OS/display/network settings and exact failing sequences. Measure true
cursor latency with a high-speed camera or equivalent physical setup; heartbeat
RTT and loopback receiver timing are different metrics. Signing/notarization,
secure desktop, native cross-edge file dragging and broad hardware coverage
remain release work. The application remains a development alpha.
