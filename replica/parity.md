## Parity: 56.0 / 100

features 56.0  (24 counted, must-haves 5 of 11 done)

Not shippable yet: 6 must-have features are not done.

## By area, weakest first
- power                          0.0  (2 features)
- distribution                  30.0  (2 features)
- files                         37.5  (3 features)
- input                         38.5  (6 features)
- clipboard                     50.0  (2 features)
- layout                        64.3  (3 features)
- security                     100.0  (3 features)
- network                      100.0  (2 features)
- diagnostics                  100.0  (1 features)

## Missing, in build order
- [must] files: Native edge file drag, no  (App-window drag/drop works; Explorer/Finder cross-edge dragging not implemented)
- [must] clipboard: Clipboard text, partial  (Real encrypted exchange tested with memory adapter; OS pasteboard tests pending)
- [must] distribution: Windows and macOS builds, partial  (Engine target checks passed; full native installer build workflow pending)
- [must] input: Edge cursor hopping, partial  (Native adapters and routing implemented; physical Windows/Mac tests pending)
- [must] input: Keyboard sharing and modifier mapping, partial  (HID mapping implemented; OS behavior and international layouts need hardware tests)
- [must] layout: Multiple display arrangement, partial  (Offset stacked and hidden topology tested; mixed DPI OS behavior pending)
- [should] distribution: Signed installers, no  (Unsigned native CI packages; certificates not configured)
- [should] files: Clipboard files, no  (Native file pasteboard bridge pending)
- [should] input: Monitor hop shortcuts, no  (Planned configurable per-display bindings)
- [should] power: Idle display dimming, no  (Hardware brightness adapter pending)
- [should] power: Lock wake session synchronization, no  (OS notifications and privileged integrations pending)
- [should] clipboard: Clipboard images and formatting, partial  (PNG/HTML adapters compile; cross-app OS tests pending)
- [should] input: Cursor lock, partial  (Edge-blocking toggle; native usage needs testing)
- [should] input: Game mode, partial  (Edge-blocking only; raw relative gaming input pending)
- [should] layout: Hide displays, partial  (Hidden displays excluded from routing; native held drags need validation)
- [could] input: Secure desktop control, no  (Windows service privilege work pending)

## Left out on purpose (not scored)
- Commercial lifetime licensing: User-owned local alpha; commercial licensing deferred

## Yours, not in the original (not scored)
- Linux input
- Consent by default
