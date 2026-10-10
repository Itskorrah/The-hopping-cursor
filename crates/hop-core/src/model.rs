use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl Rect {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.width && y < self.y + self.height
    }
    pub fn valid(&self) -> bool {
        [self.x, self.y, self.width, self.height]
            .iter()
            .all(|x| x.is_finite())
            && self.width >= 100.0
            && self.height >= 100.0
            && self.width <= 32768.0
            && self.height <= 32768.0
            && self.x.abs() <= 100000.0
            && self.y.abs() <= 100000.0
    }
    pub fn clamp(&self, x: f64, y: f64) -> (f64, f64) {
        (
            x.clamp(self.x, self.x + self.width - 1.0),
            y.clamp(self.y, self.y + self.height - 1.0),
        )
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Display {
    pub id: String,
    pub name: String,
    pub bounds: Rect,
    pub scale: f64,
    pub primary: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub displays: Vec<Display>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LayoutDisplay {
    pub device_id: String,
    pub display_id: String,
    pub rect: Rect,
    pub hidden: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HopKey {
    pub device_id: String,
    pub display_id: String,
    pub keys: Vec<u16>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub device_name: String,
    pub port: u16,
    pub clipboard_text: bool,
    pub clipboard_images: bool,
    pub clipboard_html: bool,
    pub clipboard_files: bool,
    pub map_modifiers: bool,
    pub auto_accept_files: bool,
    pub receive_directory: String,
    pub max_file_bytes: u64,
    pub sensitivity: f64,
    pub edge_delay_ms: u64,
    pub paused: bool,
    pub cursor_locked: bool,
    pub game_mode: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            device_name: "My computer".into(),
            port: 24810,
            clipboard_text: true,
            clipboard_images: true,
            clipboard_html: true,
            clipboard_files: false,
            map_modifiers: true,
            auto_accept_files: false,
            receive_directory: String::new(),
            max_file_bytes: 10 * 1024 * 1024 * 1024,
            sensitivity: 1.0,
            edge_delay_ms: 80,
            paused: false,
            cursor_locked: false,
            game_mode: false,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            !self.device_name.trim().is_empty() && self.device_name.chars().count() <= 64,
            "Computer name must contain 1–64 characters"
        );
        anyhow::ensure!(self.port > 1024, "Choose a port above 1024");
        anyhow::ensure!(
            self.sensitivity.is_finite() && (0.1..=4.0).contains(&self.sensitivity),
            "Sensitivity must be between 0.1 and 4"
        );
        anyhow::ensure!(
            self.edge_delay_ms <= 2000,
            "Edge delay must be at most 2000 ms"
        );
        anyhow::ensure!(self.max_file_bytes > 0, "File limit must be positive");
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedPeer {
    pub public_key: String,
    pub name: String,
    pub address: Option<String>,
    pub allow_input: bool,
    pub allow_clipboard: bool,
    pub allow_files: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerStatus {
    pub device: Device,
    pub address: String,
    pub public_key: Option<String>,
    pub safety_code: Option<String>,
    pub trusted: bool,
    pub connected: bool,
    pub latency_ms: Option<f64>,
    pub last_error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferStatus {
    pub id: String,
    pub peer_id: String,
    pub name: String,
    pub size: u64,
    pub bytes: u64,
    pub direction: String,
    pub state: String,
    pub error: Option<String>,
    pub saved_path: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permissions {
    pub capture: bool,
    pub injection: bool,
    pub clipboard: bool,
    pub explanation: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutOffer {
    pub peer_id: String,
    pub layout: Vec<LayoutDisplay>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub local: Device,
    pub public_key: String,
    pub fingerprint: String,
    pub settings: Settings,
    pub peers: Vec<PeerStatus>,
    pub layout: Vec<LayoutDisplay>,
    pub layout_offers: Vec<LayoutOffer>,
    pub profiles: Vec<String>,
    pub hop_keys: Vec<HopKey>,
    pub peer_permissions: BTreeMap<String, PeerPermissions>,
    pub transfers: Vec<TransferStatus>,
    pub permissions: Permissions,
    pub capture_ready: bool,
    pub displays_ready: bool,
    pub active_device: String,
    pub controller_device: String,
    pub cursor_visible: bool,
    pub listening_port: u16,
    pub diagnostics: Vec<String>,
    pub version: String,
    pub session_available: bool,
    pub suppressed_events: Option<u64>,
    pub keyboard_diagnostics: KeyboardDiagnostics,
    pub scroll_diagnostics: ScrollDiagnostics,
    pub control_queues: ControlQueueDiagnostics,
}
/// Counts only, with no wheel positions, directions, apps or event trace.
/// These process-lifetime counters describe engine stages, not application
/// acceptance of an OS-injected event.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScrollDiagnostics {
    pub captured: u64,
    pub forwarded: u64,
    pub received: u64,
    pub applied: u64,
    pub rejected: u64,
    pub natural_scrolling: Option<bool>,
}
/// Normal control queues for current connections. Historical values reset
/// when a connection is replaced. Wait excludes socket/OS/application latency.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ControlQueueDiagnostics {
    pub pending: usize,
    pub peak_pending: usize,
    pub coalesced_moves: u64,
    pub full_rejections: u64,
    pub longest_wait_us: u64,
}
/// Aggregate counts only: never key identities, text, foreground app names or
/// a timestamped event trace. Reset when the engine process restarts.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KeyboardDiagnostics {
    pub captured: u64,
    pub forwarded: u64,
    pub received: u64,
    pub applied: u64,
    pub rejected: u64,
    pub native: Option<KeyboardCaptureStats>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardCaptureStats {
    pub physical_events: u64,
    pub injected_ignored: u64,
    pub unsupported_events: u64,
    pub longest_callback_us: u64,
    pub slow_callbacks: u64,
    pub last_event_had_focus: Option<bool>,
    pub last_event_had_caret: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerPermissions {
    pub input: bool,
    pub clipboard: bool,
    pub files: bool,
}

/// USB HID keyboard usages, independent of Windows virtual keys and Mac keycodes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InputEvent {
    Move { x: f64, y: f64 },
    Button { button: u8, down: bool },
    Scroll { x: f64, y: f64 },
    Key { code: u16, down: bool },
    ReleaseAll,
}
impl InputEvent {
    pub fn validate(&self) -> bool {
        match self {
            Self::Move { x, y } => {
                x.is_finite() && y.is_finite() && x.abs() < 100000.0 && y.abs() < 100000.0
            }
            Self::Scroll { x, y } => {
                x.is_finite() && y.is_finite() && x.abs() <= 1000.0 && y.abs() <= 1000.0
            }
            Self::Button { button, .. } => *button < 5,
            Self::Key { code, .. } => (4..=231).contains(code),
            Self::ReleaseAll => true,
        }
    }
}
#[derive(Debug, Clone)]
pub enum CapturedEvent {
    Move {
        x: f64,
        y: f64,
        dx: f64,
        dy: f64,
    },
    Input(InputEvent),
    /// A native key without a portable mapping must not leak to the source.
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardPart {
    pub mime: String,
    pub data: Vec<u8>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlobKind {
    File,
    FileClipboard {
        batch: String,
        index: u16,
        total: u16,
    },
    Clipboard,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    Hello {
        version: u16,
        device: Device,
        purpose: String,
        port: u16,
        approved: bool,
    },
    Ping(u64),
    Pong(u64),
    Enter {
        display_id: String,
        x: f64,
        y: f64,
    },
    Leave,
    /// The authenticated controller names the computer displaying the cursor.
    CursorFocus {
        device_id: String,
    },
    /// An idle peer keyboard follows the shared cursor without claiming its mouse.
    SharedKey {
        code: u16,
        down: bool,
    },
    /// Arrangement changes require an explicit recipient action.
    LayoutOffer {
        layout: Vec<LayoutDisplay>,
    },
    Input(InputEvent),
    BlobStart {
        id: String,
        kind: BlobKind,
        name: String,
        size: u64,
        hash: String,
    },
    BlobAccept {
        id: String,
        accepted: bool,
    },
    BlobChunk {
        id: String,
        offset: u64,
        data: Vec<u8>,
    },
    BlobEnd {
        id: String,
    },
    BlobResult {
        id: String,
        ok: bool,
        error: Option<String>,
    },
}
pub type PeerMap = BTreeMap<String, PeerStatus>;
