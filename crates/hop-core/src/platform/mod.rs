use crate::model::*;
use anyhow::Result;
use std::sync::Arc;

#[cfg(any(target_os = "windows", target_os = "macos"))]
mod clipboard;
#[cfg(any(target_os = "windows", target_os = "macos"))]
mod clipboard_privacy;
#[cfg(any(target_os = "windows", target_os = "macos"))]
mod file_clipboard;
mod keys;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
pub type CaptureCallback = Arc<dyn Fn(CapturedEvent) -> bool + Send + Sync>;
pub trait Platform: Send + Sync {
    fn name(&self) -> &str;
    fn displays(&self) -> Result<Vec<Display>>;
    fn display_id_aliases(&self) -> Result<std::collections::BTreeMap<String, String>> {
        Ok(Default::default())
    }
    fn permissions(&self) -> Permissions;
    fn session_available(&self) -> bool {
        true
    }
    fn input_available(&self) -> bool {
        true
    }
    fn request_permissions(&self) -> Permissions {
        self.permissions()
    }
    fn start_capture(&self, callback: CaptureCallback) -> Result<()>;
    fn stop_capture(&self);
    /// Balance source-cursor suppression on all ownership/recovery paths.
    fn set_forwarding(&self, _forwarding: bool) -> Result<()> {
        Ok(())
    }
    fn suppressed_events(&self) -> Option<u64> {
        None
    }
    fn inject(&self, event: &InputEvent) -> Result<()>;
    fn read_clipboard(&self) -> Result<Vec<ClipboardPart>>;
    fn clipboard_revision(&self) -> Option<u64> {
        None
    }
    fn write_clipboard(&self, parts: &[ClipboardPart]) -> Result<()>;
    fn read_file_clipboard(&self) -> Result<Vec<std::path::PathBuf>> {
        anyhow::bail!("File clipboard is unavailable on this platform")
    }
    fn write_file_clipboard(&self, _paths: &[std::path::PathBuf]) -> Result<()> {
        anyhow::bail!("File clipboard is unavailable on this platform")
    }
}
pub fn native() -> Arc<dyn Platform> {
    #[cfg(target_os = "windows")]
    {
        Arc::new(windows::Windows::new())
    }
    #[cfg(target_os = "macos")]
    {
        Arc::new(macos::Mac::new())
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Arc::new(Headless::default())
    }
}

/// Real network/transfer development on Linux. This deliberately does not claim
/// Linux input support. An in-memory clipboard enables deterministic integration tests.
#[derive(Default)]
pub struct Headless {
    pub forwarding: std::sync::atomic::AtomicBool,
    pub session_blocked: std::sync::atomic::AtomicBool,
    pub events: std::sync::Mutex<Vec<InputEvent>>,
    pub clipboard: std::sync::Mutex<Vec<ClipboardPart>>,
    pub file_clipboard: std::sync::Mutex<Vec<std::path::PathBuf>>,
    pub display_override: std::sync::Mutex<Option<Vec<Display>>>,
}
impl Platform for Headless {
    fn set_forwarding(&self, forwarding: bool) -> Result<()> {
        self.forwarding
            .store(forwarding, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
    fn session_available(&self) -> bool {
        !self
            .session_blocked
            .load(std::sync::atomic::Ordering::SeqCst)
    }
    fn name(&self) -> &str {
        "headless"
    }
    fn displays(&self) -> Result<Vec<Display>> {
        if let Some(displays) = self.display_override.lock().unwrap().clone() {
            return Ok(displays);
        }
        Ok(vec![Display {
            id: "headless-main".into(),
            name: "Headless test display".into(),
            bounds: Rect {
                x: 0.,
                y: 0.,
                width: 1920.,
                height: 1080.,
            },
            scale: 1.,
            primary: true,
        }])
    }
    fn permissions(&self) -> Permissions {
        Permissions{capture:false,injection:false,clipboard:false,explanation:"Linux is a headless protocol/transfer development target. Global input is available in Windows/macOS builds.".into()}
    }
    fn start_capture(&self, _: CaptureCallback) -> Result<()> {
        Ok(())
    }
    fn stop_capture(&self) {}
    fn inject(&self, event: &InputEvent) -> Result<()> {
        self.events.lock().unwrap().push(event.clone());
        Ok(())
    }
    fn read_clipboard(&self) -> Result<Vec<ClipboardPart>> {
        Ok(self.clipboard.lock().unwrap().clone())
    }
    fn read_file_clipboard(&self) -> Result<Vec<std::path::PathBuf>> {
        Ok(self.file_clipboard.lock().unwrap().clone())
    }
    fn write_file_clipboard(&self, paths: &[std::path::PathBuf]) -> Result<()> {
        *self.file_clipboard.lock().unwrap() = paths.to_vec();
        Ok(())
    }
    fn write_clipboard(&self, parts: &[ClipboardPart]) -> Result<()> {
        *self.clipboard.lock().unwrap() = parts.to_vec();
        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn display_list() -> Result<Vec<Display>> {
    Ok(display_info::DisplayInfo::all()?
        .into_iter()
        .map(|d| Display {
            id: windows_display_id(&d),
            name: d.friendly_name.clone(),
            bounds: Rect {
                x: d.x as f64,
                y: d.y as f64,
                width: d.width as f64,
                height: d.height as f64,
            },
            scale: d.scale_factor as f64,
            primary: d.is_primary,
        })
        .collect())
}

#[cfg(target_os = "windows")]
fn windows_display_id(display: &display_info::DisplayInfo) -> String {
    use sha2::{Digest, Sha256};
    use windows_sys::Win32::Graphics::Gdi::{EnumDisplayDevicesW, DISPLAY_DEVICEW};
    let name = display
        .name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut device: DISPLAY_DEVICEW = unsafe { std::mem::zeroed() };
    device.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
    if unsafe { EnumDisplayDevicesW(name.as_ptr(), 0, &mut device, 1) } != 0 {
        let length = device
            .DeviceID
            .iter()
            .position(|c| *c == 0)
            .unwrap_or(device.DeviceID.len());
        if length > 0 {
            let path = String::from_utf16_lossy(&device.DeviceID[..length]).to_lowercase();
            return format!("win-{}", hex::encode(Sha256::digest(path.as_bytes())));
        }
    }
    // Virtual/driver-only displays may not expose a monitor interface path.
    format!("display-{}", display.id)
}
