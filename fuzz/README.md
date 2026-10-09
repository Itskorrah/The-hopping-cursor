# Bounded control-frame fuzzing

This harness exercises the Postcard decoder for decrypted control/bulk messages,
input validation, display bounds, and canonical encode/decode round trips. All
seeds are synthetic. It never connects to a computer or reads an OS clipboard.
The `fuzzing` feature is absent from normal desktop builds.

From the repository root:

```sh
rustup toolchain install nightly --profile minimal
cargo install cargo-fuzz --version 0.13.2 --locked
cargo run --locked --manifest-path fuzz/Cargo.toml --example seed
cargo +nightly fuzz run control_frames -- -max_total_time=60 -rss_limit_mb=512 -max_len=65519 -print_final_stats=1
```

Keep generated corpus/artifacts out of Git. Minimize a discovered failure, add a
synthetic regression, and preserve its diagnostic privately if it exposes a
security issue. Never seed with real keys, clipboard data, or network captures.

On Apple Silicon macOS, the 2026-10-09 run executed 1,770,886 cases in 61 seconds
under AddressSanitizer, with no crash/assertion and peak RSS 353 MiB. This is a
short parser smoke campaign, not exhaustive protocol, native-adapter, or
cryptographic assurance. Continue longer campaigns and independent review before
production distribution.
