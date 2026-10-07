use crate::model::*;
use anyhow::Result;
use std::sync::Arc;

#[cfg(any(target_os = "windows", target_os = "macos"))]
mod clipboard;
mod keys;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
pub type CaptureCallback = Arc<dyn Fn(CapturedEvent) -> bool + Send + Sync>;
pub trait Platform: Send + Sync {
    fn name(&self) -> &str;
    fn displays(&self) -> Result<Vec<Display>>;
    fn permissions(&self) -> Permissions;
    fn start_capture(&self, callback: CaptureCallback) -> Result<()>;
    fn stop_capture(&self);
    fn inject(&self, event: &InputEvent) -> Result<()>;
    fn read_clipboard(&self) -> Result<Vec<ClipboardPart>>;
    fn write_clipboard(&self, parts: &[ClipboardPart]) -> Result<()>;
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
    pub events: std::sync::Mutex<Vec<InputEvent>>,
    pub clipboard: std::sync::Mutex<Vec<ClipboardPart>>,
}
impl Platform for Headless {
    fn name(&self) -> &str {
        "headless"
    }
    fn displays(&self) -> Result<Vec<Display>> {
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
            id: format!("display-{}", d.id),
            name: format!("Display {}", d.id),
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
