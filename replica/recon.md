# Research and functional specification

Researched 2026-10-07 using public, rendered pages. No target source code,
bundles, binaries, private endpoints, accounts, or assets were used.

## Sources and evidence

| Source | Findings | Limits |
| --- | --- | --- |
| https://cursorhop.com/help/ | mDNS discovery; no video transport; LAN-only data; Accessibility requirements; account-based membership | Vendor documentation, not a benchmark |
| https://cursorhop.com/download/ | v0.12.0; Windows 10+, macOS 12+, Intel and ARM; unsigned; 340 MB and bundled .NET 6 on Windows; ports 24800–24802 | Rust core does not imply entire product is Rust |
| https://cursorhop.com/changelog/ | Sept releases: remote cursor hiding; about 1.5-second recovery; physical-display hop keys; modifier recorder; layout profiles; permission diagnostics; held-button and mixed-DPI fixes | Detailed implementation is unknown |
| https://cursorhop.com/features/mouse-sharing/ | edges, vertical and offset arrangements, up to ten computers | No independent latency measurements |
| https://cursorhop.com/features/keyboard-sharing/ | Command/Control and Option/Alt translation; target keyboard layout applies | Media/vendor keys are separate work |
| https://cursorhop.com/features/clipboard-sharing/ | text, formatting, images; clipboard files gated by plan | Platform pasteboard format conversions unspecified |
| https://cursorhop.com/features/file-transfer/ | native cross-screen file dragging; cancel/progress; folders described as zipped | This is harder than dropping into our own window |
| https://cursorhop.com/features/aes-encryption/ | Despite URL name, page specifies Noise_XX, X25519, ChaCha20-Poly1305, SHA-256 | Pairing authentication UX not explained publicly |
| https://cursorhop.com/features/auto-discovery/ | mDNS and remembered devices | Enterprise multicast isolation remains a constraint |
| https://cursorhop.com/features/screen-dimming/ | idle screens dim; Max-only | OS brightness hardware varies |
| https://www.trustpilot.com/review/cursorhop.com | Three reviews visible: Don Simmonds, Zam, Fadi Alkazzaz. Hidden displays and support praised | Very small sample, not representative; no invented complaint frequencies |
| https://deskflow.org/ | Active open-source alternative, encrypted sharing, Windows/macOS/Linux | Studied public product description, no code copied |
| https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput | SendInput blocked by higher integrity levels/UIPI | Secure desktop needs signed privileged service engineering |
| https://developer.apple.com/documentation/coregraphics/cgevent | Native event taps and synthetic input | Requires permissions and real macOS validation |

## Scope

Desktop software input switch, not remote video streaming or a website clone.
Working name: The Hopping Cursor. No account or subscription needed for local
pairing. Native Rust control/data core, independent interface and assets.
Platform target: Windows and macOS; Linux supports protocol development and
headless transfer tests, not global input capture in this version.

## Screens (inferred from public descriptions; native UI not directly inspected)

| ID | Screen | Our purpose/states |
| --- | --- | --- |
| S01 | Workspace | Connected, alone, paused, permission warning; active computer |
| S02 | Computers and pairing | Discovered, fingerprint pending, trusted, offline, rejected |
| S03 | Display arrangement | Drag, snap, hidden displays, invalid overlap, save |
| S04 | Transfers | Offer, accept/decline, streaming, complete, cancelled, failed |
| S05 | Preferences | Sharing switches, receiving folder, sensitivity, network |
| S06 | Diagnostics | Permissions, actual RTT, connection failures, support export |

## Flows

F01 Install/open → discover → establish encrypted handshake → compare safety
code on both machines → explicitly trust on both → connect. Edge: key changed,
wrong address, multicast blocked, port busy, one side not approved.

F02 Arrange physical displays → save → cross an adjacent visible edge → input
and keyboard follow → return. Edges: offsets, mixed scale, hidden display,
offline peer, held modifiers, disconnect, emergency release.

F03 Copy supported clipboard type → encrypted bulk connection → apply on peers
without bounce loops. Edges: sharing disabled, size limit, sensitive clipboard.

F04 Choose/drop file in app → offer → recipient consent → encrypted chunks →
SHA-256 verification → atomic publication without overwriting existing files.
Edges: path traversal, size mismatch, disk error, rejection, cancellation.
Native Explorer/Finder drag across screen edges is a distinct advanced feature.

F05 Pause/lock cursor → keep input local; emergency chord always recovers.
F06 Inspect diagnostics/export → contains no keystrokes or clipboard/file content.

## Inferred entities

Identity (persistent X25519 key), Peer (identity, trust, last address,
permissions), PhysicalDisplay (owner, stable ID, bounds/scale), LayoutDisplay
(workspace rectangle, visibility), Preferences, Transfer (offer, bytes, hash,
status). These are our architecture choices, not claims about the target's schema.

## Size and difficult parts

L: a production system needs multiple OS testing cycles. Native global input,
secure desktop privilege boundaries, and native cross-app drag-and-drop are
the largest risks. Commercial billing, target's accounts, proprietary
licensing/updates, Linux input, and promises of measured superiority are out of
scope. Every implemented feature must have its validation limits recorded.
