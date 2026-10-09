#![cfg(target_os = "windows")]
use hop_core::platform;
use std::sync::Arc;
use windows_sys::Win32::{Foundation::POINT, UI::WindowsAndMessaging::GetCursorPos};

// Explicit desktop probe: installs real hooks, parks/restores the real pointer.
// Close other input-sharing engines and run serially; never part of CI defaults.
#[test]
#[ignore = "Requires an interactive Windows desktop with other sharing engines quit"]
fn native_capture_parking_and_restore() {
    let native = platform::native();
    native.start_capture(Arc::new(|_| false)).unwrap();
    let mut origin: POINT = unsafe { std::mem::zeroed() };
    assert_ne!(unsafe { GetCursorPos(&mut origin) }, 0);
    native.set_forwarding(true).unwrap();
    let mut parked: POINT = unsafe { std::mem::zeroed() };
    assert_ne!(unsafe { GetCursorPos(&mut parked) }, 0);
    native.set_forwarding(false).unwrap();
    let mut restored: POINT = unsafe { std::mem::zeroed() };
    assert_ne!(unsafe { GetCursorPos(&mut restored) }, 0);
    native.stop_capture();
    assert_eq!((restored.x, restored.y), (origin.x, origin.y));
    let displays = native.displays().unwrap();
    assert!(displays.iter().any(
        |d| (parked.x as f64 - (d.bounds.x + d.bounds.width / 2.)).abs() <= 1.
            && (parked.y as f64 - (d.bounds.y + d.bounds.height / 2.)).abs() <= 1.
    ));
}
