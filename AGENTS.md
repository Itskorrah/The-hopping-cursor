# Working on The Hopping Cursor

This is a native desktop input-sharing application, not a website clone.
Read `replica/recon.md`, `replica/architecture.md`, `docs/SECURITY.md`, and the
current README before changing protocol or native input code.

Never log keys, clipboard contents, file contents, or private identities.
Discovery never grants trust. No plaintext fallback. Keep control traffic
independent of file/clipboard traffic, input ownership explicit, and emergency
release available even if network queues are full.

Native API code lives in `crates/hop-core/src/platform/`. Keep injected-event
filtering and pressed-key cleanup intact. Do not claim Windows secure desktop,
real hardware validation, latency benchmarks, or native Finder/Explorer edge
drag-and-drop without evidence.

Use the installed Replica skills in `.agents/skills/` for recon/build/test/parity
work. Updated feature status belongs in `replica/features.csv` and README.
Browser preview data must remain visibly labelled and never appear in native
mode. Keep UI actions wired to the core through Tauri IPC.

After meaningful engine changes run `cargo test` and `cargo clippy -p hop-core
-p hopping-cursor --all-targets -- -D warnings`. After UI changes run `npm run
build` and the relevant Playwright tests. Run `cargo fmt --all` before committing.
Cross-check native adapters for Windows and both Mac architectures; GitHub
Actions builds installers. No agent delegation is required by this file.
