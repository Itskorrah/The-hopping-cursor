This is a conservative feature tracking score from the installed Replica skill,
updated 2026-10-09. It is not a percentage of development completed, performance
benchmark, security certification, or release approval. Native capabilities
remain partial until physical acceptance checks pass. Added privacy markers are
excluded from reference parity because equivalent reference support is unverified.

## Parity: 60.6 / 100

features 60.6  (27 counted, must-haves 6 of 13 done)

Not shippable yet: 7 must-have features are not done.

## By area, weakest first
- power                          0.0  (2 features)
- input                         46.2  (6 features)
- clipboard                     50.0  (2 features)
- files                         50.0  (3 features)
- desktop                       50.0  (1 features)
- layout                        60.0  (4 features)
- distribution                  60.0  (2 features)
- security                      86.4  (4 features)
- network                      100.0  (2 features)
- diagnostics                  100.0  (1 features)

## Missing, in build order
- [must] files: Native edge file drag, no  (App-window drag/drop works; Explorer/Finder cross-edge dragging not implemented)
- [must] clipboard: Clipboard text, partial  (Real encrypted exchange tested with memory adapter; OS pasteboard tests pending)
- [must] input: Edge cursor hopping, partial  (Native adapters and routing implemented; physical Windows/Mac tests pending)
- [must] input: Keyboard sharing and modifier mapping, partial  (HID mapping implemented; OS behavior and international layouts need hardware tests)
- [must] layout: Display lifecycle, partial  (Two-second hotplug refresh; Mac UUID and Windows monitor interface paths; physical dock testing pending)
- [must] layout: Multiple display arrangement, partial  (Offset stacked and hidden topology tested; mixed DPI OS behavior pending)
- [must] security: Protected identities, partial  (Keychain migration verified on Mac; DPAPI cross-compiled; Windows runtime validation pending)
- [should] distribution: Signed installers, no  (Unsigned native CI packages; certificates not configured)
- [should] power: Idle display dimming, no  (Hardware brightness adapter pending)
- [should] power: Lock wake session synchronization, no  (Protected-input and timing-gap safety recovery implemented; synchronized OS actions and full notifications pending)
- [should] clipboard: Clipboard images and formatting, partial  (PNG/HTML adapters compile; cross-app OS tests pending)
- [should] desktop: Single instance, partial  (Second launch focuses existing window; native verification pending)
- [should] files: Clipboard files, partial  (Explicit Finder URLs and Explorer CF_HDROP actions through consented transfer; Mac isolated round-trip tested; automatic sync and physical cross-app checks pending)
- [should] input: Cursor lock, partial  (Edge-blocking toggle; native usage needs testing)
- [should] input: Game mode, partial  (Edge-blocking only; raw relative gaming input pending)
- [should] input: Monitor hop shortcuts, partial  (Persistent physical-key chords and per-monitor desktop controls; protocol and UI tested; native hardware pending)
- [should] layout: Hide displays, partial  (Hidden displays excluded from routing; native held drags need validation)
- [could] input: Secure desktop control, no  (Windows service privilege work pending)

## Left out on purpose (not scored)
- Commercial lifetime licensing: User-owned local alpha; commercial licensing deferred

## Yours, not in the original (not scored)
- Consent by default
- Clipboard privacy markers

No new screenshot/layout or reference-app behavior comparison was performed in
this continuation; this is a native desktop reliability pass. Prior public
research remains in recon.md. Browser samples do not establish native parity.

Highest-value next work: physical input/permission testing; cross-app native
file clipboard validation and edge drag lifecycle; OS power/session adapters;
keyboard/raw-input compatibility; signing, updates, sustained fuzzing and external
security review. Follow docs/ROADMAP.md and docs/PLATFORM_TESTING.md.
