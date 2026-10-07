use super::{keys, CaptureCallback, Platform};
use crate::model::*;
use anyhow::{Context, Result};
use std::{
    collections::BTreeSet,
    sync::{
        atomic::{AtomicU32, Ordering},
        Mutex, OnceLock,
    },
    thread::JoinHandle,
};
use windows_sys::Win32::{
    Foundation::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};
static CALLBACK: OnceLock<Mutex<Option<CaptureCallback>>> = OnceLock::new();
pub struct Windows {
    thread_id: AtomicU32,
    thread: Mutex<Option<JoinHandle<()>>>,
    held: Mutex<BTreeSet<u16>>,
    buttons: Mutex<BTreeSet<u8>>,
}
impl Windows {
    pub fn new() -> Self {
        #[link(name = "user32")]
        extern "system" {
            fn SetProcessDpiAwarenessContext(value: isize) -> i32;
        }
        unsafe {
            SetProcessDpiAwarenessContext(-4);
        }
        Self {
            thread_id: AtomicU32::new(0),
            thread: Mutex::new(None),
            held: Mutex::new(BTreeSet::new()),
            buttons: Mutex::new(BTreeSet::new()),
        }
    }
}
fn callback(event: CapturedEvent) -> bool {
    let cb = CALLBACK.get().and_then(|m| m.lock().ok()?.clone());
    cb.is_some_and(|cb| cb(event))
}
unsafe extern "system" fn keyboard_hook(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
    if code >= 0 {
        let k = &*(l as *const KBDLLHOOKSTRUCT);
        if k.flags & LLKHF_INJECTED == 0 {
            let down = matches!(w as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
            if let Some(hid) =
                keys::windows_to_hid(k.vkCode, k.scanCode, k.flags & LLKHF_EXTENDED != 0)
            {
                if callback(CapturedEvent::Input(InputEvent::Key { code: hid, down })) {
                    return 1;
                }
            }
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, w, l)
}
unsafe extern "system" fn mouse_hook(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
    if code >= 0 {
        let m = &*(l as *const MSLLHOOKSTRUCT);
        if m.flags & LLMHF_INJECTED == 0 {
            let event = match w as u32 {
                WM_MOUSEMOVE => {
                    let mut current = POINT { x: 0, y: 0 };
                    GetCursorPos(&mut current);
                    Some(CapturedEvent::Move {
                        x: m.pt.x as f64,
                        y: m.pt.y as f64,
                        dx: (m.pt.x - current.x) as f64,
                        dy: (m.pt.y - current.y) as f64,
                    })
                }
                WM_LBUTTONDOWN | WM_LBUTTONUP => Some(CapturedEvent::Input(InputEvent::Button {
                    button: 0,
                    down: w as u32 == WM_LBUTTONDOWN,
                })),
                WM_RBUTTONDOWN | WM_RBUTTONUP => Some(CapturedEvent::Input(InputEvent::Button {
                    button: 1,
                    down: w as u32 == WM_RBUTTONDOWN,
                })),
                WM_MBUTTONDOWN | WM_MBUTTONUP => Some(CapturedEvent::Input(InputEvent::Button {
                    button: 2,
                    down: w as u32 == WM_MBUTTONDOWN,
                })),
                WM_XBUTTONDOWN | WM_XBUTTONUP => Some(CapturedEvent::Input(InputEvent::Button {
                    button: if m.mouseData >> 16 == 1 { 3 } else { 4 },
                    down: w as u32 == WM_XBUTTONDOWN,
                })),
                WM_MOUSEWHEEL => Some(CapturedEvent::Input(InputEvent::Scroll {
                    x: 0.,
                    y: (m.mouseData >> 16) as i16 as f64 / 120.,
                })),
                WM_MOUSEHWHEEL => Some(CapturedEvent::Input(InputEvent::Scroll {
                    x: (m.mouseData >> 16) as i16 as f64 / 120.,
                    y: 0.,
                })),
                _ => None,
            };
            if event.is_some_and(callback) {
                return 1;
            }
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, w, l)
}
impl Platform for Windows {
    fn name(&self) -> &str {
        "windows"
    }
    fn displays(&self) -> Result<Vec<Display>> {
        super::display_list()
    }
    fn permissions(&self) -> Permissions {
        Permissions{capture:true,injection:true,clipboard:true,explanation:"Normal desktop input available. Elevated apps and Windows secure desktop require additional privileged-service work.".into()}
    }
    fn start_capture(&self, cb: CaptureCallback) -> Result<()> {
        let slot = CALLBACK.get_or_init(|| Mutex::new(None));
        let mut guard = slot.lock().unwrap();
        anyhow::ensure!(
            guard.is_none(),
            "Only one native input engine can run in this process"
        );
        *guard = Some(cb);
        drop(guard);
        let (tx, rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || unsafe {
            let module = GetModuleHandleW(std::ptr::null());
            let keyboard = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook), module, 0);
            let mouse = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook), module, 0);
            if keyboard.is_null() || mouse.is_null() {
                let _ = tx.send(Err("Windows input hook installation failed".to_string()));
            } else {
                let mut msg: MSG = std::mem::zeroed();
                PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_NOREMOVE);
                let id = windows_sys::Win32::System::Threading::GetCurrentThreadId();
                let _ = tx.send(Ok(id));
                while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            if !keyboard.is_null() {
                UnhookWindowsHookEx(keyboard);
            }
            if !mouse.is_null() {
                UnhookWindowsHookEx(mouse);
            }
        });
        match rx.recv().context("Input thread failed")? {
            Ok(id) => {
                self.thread_id.store(id, Ordering::SeqCst);
                *self.thread.lock().unwrap() = Some(handle);
                Ok(())
            }
            Err(e) => {
                let _ = handle.join();
                *slot.lock().unwrap() = None;
                anyhow::bail!(e)
            }
        }
    }
    fn stop_capture(&self) {
        let id = self.thread_id.swap(0, Ordering::SeqCst);
        if id != 0 {
            unsafe {
                PostThreadMessageW(id, WM_QUIT, 0, 0);
            }
            if let Some(h) = self.thread.lock().unwrap().take() {
                let _ = h.join();
            }
            if let Some(c) = CALLBACK.get() {
                *c.lock().unwrap() = None;
            }
        }
    }
    fn inject(&self, event: &InputEvent) -> Result<()> {
        if *event == InputEvent::ReleaseAll {
            let held: Vec<_> = self.held.lock().unwrap().iter().copied().collect();
            for code in held {
                self.inject(&InputEvent::Key { code, down: false })?;
            }
            let buttons: Vec<_> = self.buttons.lock().unwrap().iter().copied().collect();
            for button in buttons {
                self.inject(&InputEvent::Button {
                    button,
                    down: false,
                })?;
            }
            return Ok(());
        }
        let mut input: INPUT = unsafe { std::mem::zeroed() };
        match event {
            InputEvent::Key { code, down } => {
                let (scan, extended) =
                    keys::hid_to_windows_scan(*code).context("Unsupported keyboard key")?;
                input.r#type = INPUT_KEYBOARD;
                input.Anonymous.ki = KEYBDINPUT {
                    wVk: 0,
                    wScan: scan,
                    dwFlags: KEYEVENTF_SCANCODE
                        | if *down { 0 } else { KEYEVENTF_KEYUP }
                        | if extended { KEYEVENTF_EXTENDEDKEY } else { 0 },
                    time: 0,
                    dwExtraInfo: 0,
                };
            }
            InputEvent::Move { x, y } => unsafe {
                let left = GetSystemMetrics(SM_XVIRTUALSCREEN);
                let top = GetSystemMetrics(SM_YVIRTUALSCREEN);
                let width = GetSystemMetrics(SM_CXVIRTUALSCREEN).max(2);
                let height = GetSystemMetrics(SM_CYVIRTUALSCREEN).max(2);
                input.r#type = INPUT_MOUSE;
                input.Anonymous.mi = MOUSEINPUT {
                    dx: ((*x - left as f64) * 65535. / (width - 1) as f64) as i32,
                    dy: ((*y - top as f64) * 65535. / (height - 1) as f64) as i32,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                    time: 0,
                    dwExtraInfo: 0,
                };
            },
            InputEvent::Button { button, down } => {
                let flags = match (*button, *down) {
                    (0, true) => MOUSEEVENTF_LEFTDOWN,
                    (0, false) => MOUSEEVENTF_LEFTUP,
                    (1, true) => MOUSEEVENTF_RIGHTDOWN,
                    (1, false) => MOUSEEVENTF_RIGHTUP,
                    (2, true) => MOUSEEVENTF_MIDDLEDOWN,
                    (2, false) => MOUSEEVENTF_MIDDLEUP,
                    (_, true) => MOUSEEVENTF_XDOWN,
                    (_, false) => MOUSEEVENTF_XUP,
                };
                input.r#type = INPUT_MOUSE;
                input.Anonymous.mi = MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: if *button == 3 {
                        1
                    } else if *button == 4 {
                        2
                    } else {
                        0
                    },
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                };
            }
            InputEvent::Scroll { x, y } => {
                for (amount, flag) in [(*x, MOUSEEVENTF_HWHEEL), (*y, MOUSEEVENTF_WHEEL)] {
                    if amount != 0. {
                        let i = INPUT {
                            r#type: INPUT_MOUSE,
                            Anonymous: INPUT_0 {
                                mi: MOUSEINPUT {
                                    dx: 0,
                                    dy: 0,
                                    mouseData: (amount * 120.) as i32 as u32,
                                    dwFlags: flag,
                                    time: 0,
                                    dwExtraInfo: 0,
                                },
                            },
                        };
                        unsafe {
                            anyhow::ensure!(
                                SendInput(1, &i, std::mem::size_of::<INPUT>() as i32) == 1,
                                "Input injection denied"
                            );
                        }
                    }
                }
                return Ok(());
            }
            InputEvent::ReleaseAll => unreachable!(),
        }
        unsafe {
            anyhow::ensure!(
                SendInput(1, &input, std::mem::size_of::<INPUT>() as i32) == 1,
                "Windows refused input injection (check privilege level)"
            );
        }
        match event {
            InputEvent::Key { code, down } => {
                if *down {
                    self.held.lock().unwrap().insert(*code);
                } else {
                    self.held.lock().unwrap().remove(code);
                }
            }
            InputEvent::Button { button, down } => {
                if *down {
                    self.buttons.lock().unwrap().insert(*button);
                } else {
                    self.buttons.lock().unwrap().remove(button);
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn read_clipboard(&self) -> Result<Vec<ClipboardPart>> {
        super::clipboard::read()
    }
    fn clipboard_revision(&self) -> Option<u64> {
        Some(
            unsafe { windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber() }
                as u64,
        )
    }
    fn write_clipboard(&self, p: &[ClipboardPart]) -> Result<()> {
        super::clipboard::write(p)
    }
}
impl Drop for Windows {
    fn drop(&mut self) {
        self.stop_capture();
        let _ = self.inject(&InputEvent::ReleaseAll);
    }
}

pub fn read_html() -> Option<String> {
    unsafe {
        use windows_sys::Win32::System::{DataExchange::*, Memory::*};
        let name: Vec<u16> = "HTML Format\0".encode_utf16().collect();
        let format = RegisterClipboardFormatW(name.as_ptr());
        if format == 0 || OpenClipboard(std::ptr::null_mut()) == 0 {
            return None;
        }
        let handle = GetClipboardData(format);
        let result = if handle.is_null() {
            None
        } else {
            let size = GlobalSize(handle as _);
            let p = GlobalLock(handle as _);
            let result = if p.is_null() || size > 8 * 1024 * 1024 {
                None
            } else {
                let data = std::slice::from_raw_parts(p as *const u8, size);
                let text = String::from_utf8_lossy(data);
                let start = text.find("<!--StartFragment-->").map(|n| n + 20);
                let end = text.find("<!--EndFragment-->");
                match (start, end) {
                    (Some(a), Some(b)) if b >= a => Some(text[a..b].into()),
                    _ => None,
                }
            };
            if !p.is_null() {
                GlobalUnlock(handle as _);
            }
            result
        };
        CloseClipboard();
        result
    }
}
