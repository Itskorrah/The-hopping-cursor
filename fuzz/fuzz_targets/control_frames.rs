#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|bytes: &[u8]| {
    hop_core::crypto::fuzz_plaintext(bytes);
});
