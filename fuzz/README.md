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
cargo +nightly fuzz run control_frames crates/hop-core/tests/fixtures/malformed-display-count.bin -- -rss_limit_mb=512 -print_final_stats=1
cargo +nightly fuzz run control_frames -- -max_total_time=60 -rss_limit_mb=2048 -max_len=65519 -print_final_stats=1
```

Keep generated corpus/artifacts out of Git. Minimize a discovered failure, add a
synthetic regression, and preserve its diagnostic privately if it exposes a
security issue. Never seed with real keys, clipboard data, or network captures.

The campaign uses libFuzzer's bounded default whole-process RSS budget of
2048 MiB. A Linux CI campaign initially hit a 512 MiB cap after 4,968,635 cases;
the diagnostic reported about 25.5 MB of live heap and 120.9 MB quarantined.
Its saved 160-byte synthetic input replays successfully under AddressSanitizer
at the original 512 MiB cap and is retained as a normal decoder regression and
corpus seed. This evidence points to cumulative sanitizer/fuzzer allocator
overhead rather than that input alone allocating the process limit. CI replays
the case before each campaign. AddressSanitizer and its quarantine remain
enabled; no finding is suppressed. See [libFuzzer's resource flags](https://llvm.org/docs/LibFuzzer.html#options)
and [AddressSanitizer's quarantine settings](https://github.com/google/sanitizers/wiki/AddressSanitizerFlags).

On Apple Silicon macOS, a 2026-10-09 run executed 5,299,006 cases in 181 seconds
under AddressSanitizer, with no crash/assertion and peak RSS 549 MiB. An earlier
61-second run passed 1,770,886 cases at 353 MiB. Linux CI also passed the retained
single-input replay at the original 512 MiB cap and a 61-second campaign with
5,930,511 cases at 588 MiB. These are short parser smoke campaigns, not exhaustive protocol, native-adapter, or
cryptographic assurance. Continue longer campaigns and independent review before
production distribution.


## Portable Windows mutation fallback (0.1.5)

The normal MSVC toolchain on this PC does not run the Mac/Linux ASan harness.
A deterministic bounded smoke campaign is available without libFuzzer:

```powershell
. ./scripts/use-windows-portable-toolchain.ps1
$env:HOP_MUTATION_CASES = '1000000'
cargo test --locked --release -p hop-core --features fuzzing --lib bounded_plaintext_mutation_campaign -- --ignored --nocapture
```

It mutates synthetic introductions/input/blob frames and the retained malformed
fixture, truncates, appends, flips bits and exercises malformed counts/random
bounded buffers. Seed is fixed; input length is capped at 65,519 bytes. The
existing decode/validate/canonical-roundtrip invariants are exercised. One million
cases pass on Windows in this pass. This is not coverage-guided, uses no sanitizer,
and must not be described as equivalent to the ASan campaigns above.
