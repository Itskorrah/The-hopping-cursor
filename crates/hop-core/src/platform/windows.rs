use super::{keys, CaptureCallback, Platform};
use crate::model::*;
use anyhow::{Context, Result};
use std::{
    collections::BTreeSet,
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex, OnceLock,
    },
    thread::JoinHandle,
};
use windows_sys::Win32::{
    Foundation::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Input::{KeyboardAndMouse::*, *},
        WindowsAndMessaging::*,
    },
};
static CURSOR_WINDOW: AtomicUsize = AtomicUsize::new(0);
static CURSOR_HIDDEN: AtomicBool = AtomicBool::new(false);
// Store source position separately from the native pointer-processing anchor.
// Parking inside a monitor gives Windows room to apply its pointer gain in
// either direction, even after an edge handoff.
static PARKING: OnceLock<Mutex<Option<(POINT, POINT)>>> = OnceLock::new();
const CURSOR_UPDATE: u32 = WM_APP + 1;
static CALLBACK: OnceLock<Mutex<Option<CaptureCallback>>> = OnceLock::new();
static KEYBOARD_PHYSICAL: AtomicU64 = AtomicU64::new(0);
static KEYBOARD_INJECTED: AtomicU64 = AtomicU64::new(0);
static KEYBOARD_UNSUPPORTED: AtomicU64 = AtomicU64::new(0);
static KEYBOARD_LONGEST: AtomicU64 = AtomicU64::new(0);
static KEYBOARD_SLOW: AtomicU64 = AtomicU64::new(0);
static KEYBOARD_FOCUS: AtomicUsize = AtomicUsize::new(0);
static KEYBOARD_CARET: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
const FOCUS_PROBE_MARKER: usize = 0x484F5054;
#[cfg(test)]
const MOUSE_PRESSURE_PROBE: u32 = WM_APP + 19;
pub struct Windows {
    thread_id: AtomicU32,
    thread: Mutex<Option<JoinHandle<()>>>,
    keyboard_thread_id: AtomicU32,
    keyboard_thread: Mutex<Option<JoinHandle<()>>>,
    held: Mutex<BTreeSet<u16>>,
    buttons: Mutex<BTreeSet<u8>>,
    suppressed: Arc<AtomicU64>,
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
            keyboard_thread_id: AtomicU32::new(0),
            keyboard_thread: Mutex::new(None),
            held: Mutex::new(BTreeSet::new()),
            buttons: Mutex::new(BTreeSet::new()),
            suppressed: Arc::new(AtomicU64::new(0)),
        }
    }
    fn stop_keyboard(&self) {
        let id = self.keyboard_thread_id.swap(0, Ordering::SeqCst);
        if id != 0 {
            unsafe {
                PostThreadMessageW(id, WM_QUIT, 0, 0);
            }
            if let Some(thread) = self.keyboard_thread.lock().unwrap().take() {
                let _ = thread.join();
            }
        }
    }
}
// Keyboard hook delivery must not queue behind mouse/window/raw-input work.
// Each hook owns a dedicated live message pump; callbacks still share the
// service's ordered ownership state and production injection filtering.
fn start_keyboard() -> Result<(u32, JoinHandle<()>)> {
    let (ready, result) = std::sync::mpsc::channel();
    let thread = std::thread::spawn(move || unsafe {
        let mut msg: MSG = std::mem::zeroed();
        PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_NOREMOVE);
        let hook = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(keyboard_hook),
            GetModuleHandleW(std::ptr::null()),
            0,
        );
        if hook.is_null() {
            let _ = ready.send(Err("Windows keyboard hook installation failed"));
            return;
        }
        let id = windows_sys::Win32::System::Threading::GetCurrentThreadId();
        if ready.send(Ok(id)).is_ok() {
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        UnhookWindowsHookEx(hook);
    });
    match result.recv() {
        Ok(Ok(id)) => Ok((id, thread)),
        error => {
            let _ = thread.join();
            anyhow::bail!(
                "{}",
                match error {
                    Ok(Err(message)) => message,
                    _ => "Windows keyboard thread failed",
                }
            );
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
        // The ignored native probe uses a tagged SendInput packet to exercise
        // OS hook delivery. Production always rejects injected keyboard input.
        #[cfg(test)]
        let probe = k.dwExtraInfo == FOCUS_PROBE_MARKER;
        #[cfg(not(test))]
        let probe = false;
        if k.flags & LLKHF_INJECTED == 0 || probe {
            KEYBOARD_PHYSICAL.fetch_add(1, Ordering::Relaxed);
            let started = std::time::Instant::now();
            let mut info: GUITHREADINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
            let available = GetGUIThreadInfo(0, &mut info) != 0;
            KEYBOARD_FOCUS.store(
                if available {
                    1 + usize::from(!info.hwndFocus.is_null())
                } else {
                    0
                },
                Ordering::Relaxed,
            );
            KEYBOARD_CARET.store(
                if available {
                    1 + usize::from(!info.hwndCaret.is_null())
                } else {
                    0
                },
                Ordering::Relaxed,
            );
            let down = matches!(w as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
            let event = if let Some(hid) =
                keys::windows_to_hid(k.vkCode, k.scanCode, k.flags & LLKHF_EXTENDED != 0)
            {
                CapturedEvent::Input(InputEvent::Key { code: hid, down })
            } else {
                KEYBOARD_UNSUPPORTED.fetch_add(1, Ordering::Relaxed);
                CapturedEvent::Unsupported
            };
            let consumed = callback(event);
            let micros = started.elapsed().as_micros().min(u64::MAX as u128) as u64;
            KEYBOARD_LONGEST.fetch_max(micros, Ordering::Relaxed);
            if micros >= 100_000 {
                KEYBOARD_SLOW.fetch_add(1, Ordering::Relaxed);
            }
            if consumed {
                return 1;
            }
        } else {
            KEYBOARD_INJECTED.fetch_add(1, Ordering::Relaxed);
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, w, l)
}
unsafe extern "system" fn mouse_hook(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
    if code >= 0 {
        let m = &*(l as *const MSLLHOOKSTRUCT);
        if m.flags & LLMHF_INJECTED == 0 {
            let event = match w as u32 {
                // Use Windows' processed screen displacement while remote.
                // Raw counts bypass pointer speed/acceleration and feel unlike
                // the user's local desktop. WM_INPUT remains the local edge
                // detector because clipping can erase outward motion there.
                WM_MOUSEMOVE => {
                    if CURSOR_HIDDEN.load(Ordering::SeqCst) {
                        let parking = *PARKING.get_or_init(|| Mutex::new(None)).lock().unwrap();
                        if let Some((anchor, origin)) = parking {
                            let dx = m.pt.x as f64 - anchor.x as f64;
                            let dy = m.pt.y as f64 - anchor.y as f64;
                            if dx != 0. || dy != 0. {
                                let consumed = callback(CapturedEvent::Move {
                                    // Active routing uses deltas; an idle peer
                                    // reclaiming locally needs its saved source
                                    // position, never the hidden parking anchor.
                                    x: origin.x as f64 + dx,
                                    y: origin.y as f64 + dy,
                                    dx,
                                    dy,
                                });
                                if !consumed && !CURSOR_HIDDEN.load(Ordering::SeqCst) {
                                    SetCursorPos(origin.x + dx as i32, origin.y + dy as i32);
                                }
                            }
                        }
                        update_cursor_window();
                        // Consume the handoff-return packet too: the service
                        // has already injected its exact landing position.
                        return 1;
                    }
                    None
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
            let consumed = event.is_some_and(callback);
            // A physical click reclaiming this computer must remove the parking
            // window before Windows chooses the recipient of that click.
            if !CURSOR_HIDDEN.load(Ordering::SeqCst) {
                update_cursor_window();
            }
            if consumed {
                return 1;
            }
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, w, l)
}
// A tiny non-activating parking window owns its cursor. This avoids changing
// global cursor themes or relying on another application's WM_SETCURSOR state.
unsafe fn update_cursor_window() {
    let hwnd = CURSOR_WINDOW.load(Ordering::SeqCst) as HWND;
    if hwnd.is_null() {
        return;
    }
    if CURSOR_HIDDEN.load(Ordering::SeqCst) {
        if IsWindowVisible(hwnd) == 0 {
            let mut point = POINT { x: 0, y: 0 };
            if GetCursorPos(&mut point) != 0 {
                SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    point.x - 1,
                    point.y - 1,
                    3,
                    3,
                    SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
                // Parking/restoration happens in set_forwarding. A late
                // visibility update must never warp the pointer back after
                // ownership has already returned locally.
                SetCursor(std::ptr::null_mut());
            }
        }
    } else {
        ShowWindow(hwnd, SW_HIDE);
        SetCursor(LoadCursorW(std::ptr::null_mut(), IDC_ARROW));
    }
}
unsafe extern "system" fn input_window(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match message {
        CURSOR_UPDATE => {
            update_cursor_window();
            return 0;
        }
        WM_MOUSEACTIVATE => return MA_NOACTIVATE as LRESULT,
        WM_SETCURSOR if CURSOR_HIDDEN.load(Ordering::SeqCst) => {
            SetCursor(std::ptr::null_mut());
            return 1;
        }
        WM_INPUT => {
            // This packet was already routed by the mouse hook when remote.
            if CURSOR_HIDDEN.load(Ordering::SeqCst) {
                return DefWindowProcW(hwnd, message, w, l);
            }
            let mut raw: RAWINPUT = std::mem::zeroed();
            let mut size = std::mem::size_of::<RAWINPUT>() as u32;
            let n = GetRawInputData(
                l as HRAWINPUT,
                RID_INPUT,
                &mut raw as *mut _ as _,
                &mut size,
                std::mem::size_of::<RAWINPUTHEADER>() as u32,
            );
            if n != u32::MAX
                && n >= std::mem::size_of::<RAWINPUTHEADER>() as u32
                && raw.header.dwType == RIM_TYPEMOUSE
                && !raw.header.hDevice.is_null()
            {
                let mouse = raw.data.mouse;
                if mouse.usFlags & MOUSE_MOVE_ABSOLUTE == 0
                    && (mouse.lLastX != 0 || mouse.lLastY != 0)
                {
                    let mut point = POINT { x: 0, y: 0 };
                    if GetCursorPos(&mut point) != 0 {
                        callback(CapturedEvent::Move {
                            x: point.x as f64,
                            y: point.y as f64,
                            dx: mouse.lLastX as f64,
                            dy: mouse.lLastY as f64,
                        });
                        update_cursor_window();
                    }
                }
            }
        }
        _ => {}
    }
    DefWindowProcW(hwnd, message, w, l)
}
impl Platform for Windows {
    fn name(&self) -> &str {
        "windows"
    }
    fn displays(&self) -> Result<Vec<Display>> {
        super::display_list()
    }
    fn display_id_aliases(&self) -> Result<std::collections::BTreeMap<String, String>> {
        Ok(display_info::DisplayInfo::all()?
            .iter()
            .map(|d| (format!("display-{}", d.id), super::windows_display_id(d)))
            .collect())
    }
    fn session_available(&self) -> bool {
        use windows_sys::Win32::System::StationsAndDesktops::*;
        unsafe {
            let desktop = OpenInputDesktop(0, 0, DESKTOP_READOBJECTS);
            if desktop.is_null() {
                return false;
            }
            let mut name = [0u16; 256];
            let mut needed = 0;
            let ok = GetUserObjectInformationW(
                desktop,
                UOI_NAME,
                name.as_mut_ptr() as _,
                (name.len() * 2) as u32,
                &mut needed,
            );
            CloseDesktop(desktop);
            if ok == 0 {
                return false;
            }
            let length = name.iter().position(|c| *c == 0).unwrap_or(name.len());
            String::from_utf16_lossy(&name[..length]).eq_ignore_ascii_case("default")
        }
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
        let suppressed = self.suppressed.clone();
        *guard = Some(Arc::new(move |event| {
            let consumed = cb(event);
            if consumed {
                suppressed.fetch_add(1, Ordering::Relaxed);
            }
            consumed
        }));
        drop(guard);
        let (keyboard_id, keyboard_thread) = match start_keyboard() {
            Ok(started) => started,
            Err(error) => {
                *slot.lock().unwrap() = None;
                return Err(error);
            }
        };
        self.keyboard_thread_id.store(keyboard_id, Ordering::SeqCst);
        *self.keyboard_thread.lock().unwrap() = Some(keyboard_thread);
        let (tx, rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || unsafe {
            let module = GetModuleHandleW(std::ptr::null());
            let class: Vec<u16> = "HoppingCursorInputSink\0".encode_utf16().collect();
            let mut wc: WNDCLASSW = std::mem::zeroed();
            wc.lpfnWndProc = Some(input_window);
            wc.hInstance = module;
            wc.lpszClassName = class.as_ptr();
            RegisterClassW(&wc);
            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                class.as_ptr(),
                class.as_ptr(),
                WS_POPUP,
                0,
                0,
                3,
                3,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                module,
                std::ptr::null(),
            );
            let raw = RAWINPUTDEVICE {
                usUsagePage: 1,
                usUsage: 2,
                dwFlags: RIDEV_INPUTSINK,
                hwndTarget: hwnd,
            };
            let raw_ok = !hwnd.is_null()
                && RegisterRawInputDevices(&raw, 1, std::mem::size_of::<RAWINPUTDEVICE>() as u32)
                    != 0;
            if !hwnd.is_null() {
                SetLayeredWindowAttributes(hwnd, 0, 1, LWA_ALPHA);
                CURSOR_WINDOW.store(hwnd as usize, Ordering::SeqCst);
            }
            let mouse = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook), module, 0);
            if mouse.is_null() || !raw_ok {
                let _ = tx.send(Err("Windows input hook installation failed".to_string()));
            } else {
                let mut msg: MSG = std::mem::zeroed();
                PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_NOREMOVE);
                let id = windows_sys::Win32::System::Threading::GetCurrentThreadId();
                let _ = tx.send(Ok(id));
                while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                    #[cfg(test)]
                    if msg.message == MOUSE_PRESSURE_PROBE {
                        callback(CapturedEvent::Move {
                            x: 0.,
                            y: 0.,
                            dx: 1.,
                            dy: 0.,
                        });
                        continue;
                    }
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            CURSOR_HIDDEN.store(false, Ordering::SeqCst);
            update_cursor_window();
            CURSOR_WINDOW.store(0, Ordering::SeqCst);
            if raw_ok {
                let remove = RAWINPUTDEVICE {
                    usUsagePage: 1,
                    usUsage: 2,
                    dwFlags: RIDEV_REMOVE,
                    hwndTarget: std::ptr::null_mut(),
                };
                RegisterRawInputDevices(&remove, 1, std::mem::size_of::<RAWINPUTDEVICE>() as u32);
            }
            if !hwnd.is_null() {
                DestroyWindow(hwnd);
            }
            if !mouse.is_null() {
                UnhookWindowsHookEx(mouse);
            }
        });
        match rx
            .recv()
            .unwrap_or_else(|_| Err("Windows mouse thread failed".into()))
        {
            Ok(id) => {
                self.thread_id.store(id, Ordering::SeqCst);
                *self.thread.lock().unwrap() = Some(handle);
                Ok(())
            }
            Err(e) => {
                let _ = handle.join();
                self.stop_keyboard();
                *slot.lock().unwrap() = None;
                anyhow::bail!(e)
            }
        }
    }
    fn stop_capture(&self) {
        let _ = self.set_forwarding(false);
        let id = self.thread_id.swap(0, Ordering::SeqCst);
        if id != 0 {
            unsafe {
                PostThreadMessageW(id, WM_QUIT, 0, 0);
            }
            if let Some(h) = self.thread.lock().unwrap().take() {
                let _ = h.join();
            }
            self.stop_keyboard();
            if let Some(c) = CALLBACK.get() {
                *c.lock().unwrap() = None;
            }
        }
    }
    fn set_forwarding(&self, forwarding: bool) -> Result<()> {
        use windows_sys::Win32::Graphics::Gdi::*;
        let was_forwarding = CURSOR_HIDDEN.swap(forwarding, Ordering::SeqCst);
        if forwarding != was_forwarding {
            unsafe {
                if forwarding {
                    let mut origin: POINT = std::mem::zeroed();
                    let mut monitor: MONITORINFO = std::mem::zeroed();
                    monitor.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
                    if GetCursorPos(&mut origin) == 0
                        || GetMonitorInfoW(
                            MonitorFromPoint(origin, MONITOR_DEFAULTTONEAREST),
                            &mut monitor,
                        ) == 0
                    {
                        CURSOR_HIDDEN.store(false, Ordering::SeqCst);
                        anyhow::bail!("Cannot park the source pointer");
                    }
                    let r = monitor.rcMonitor;
                    let anchor = POINT {
                        x: r.left + (r.right - r.left) / 2,
                        y: r.top + (r.bottom - r.top) / 2,
                    };
                    *PARKING.get_or_init(|| Mutex::new(None)).lock().unwrap() =
                        Some((anchor, origin));
                    if SetCursorPos(anchor.x, anchor.y) == 0 {
                        *PARKING.get_or_init(|| Mutex::new(None)).lock().unwrap() = None;
                        CURSOR_HIDDEN.store(false, Ordering::SeqCst);
                        anyhow::bail!("Cannot park the source pointer");
                    }
                } else {
                    let parking = PARKING
                        .get_or_init(|| Mutex::new(None))
                        .lock()
                        .unwrap()
                        .take();
                    if let Some((_, origin)) = parking {
                        SetCursorPos(origin.x, origin.y);
                    }
                }
            }
        }
        let hwnd = CURSOR_WINDOW.load(Ordering::SeqCst) as HWND;
        if !hwnd.is_null() {
            unsafe {
                anyhow::ensure!(
                    PostMessageW(hwnd, CURSOR_UPDATE, 0, 0) != 0,
                    "Cannot update cursor visibility"
                );
            }
        }
        Ok(())
    }
    fn suppressed_events(&self) -> Option<u64> {
        Some(self.suppressed.load(Ordering::Relaxed))
    }
    fn keyboard_capture_stats(&self) -> Option<KeyboardCaptureStats> {
        let focus = KEYBOARD_FOCUS.load(Ordering::Relaxed);
        let caret = KEYBOARD_CARET.load(Ordering::Relaxed);
        Some(KeyboardCaptureStats {
            physical_events: KEYBOARD_PHYSICAL.load(Ordering::Relaxed),
            injected_ignored: KEYBOARD_INJECTED.load(Ordering::Relaxed),
            unsupported_events: KEYBOARD_UNSUPPORTED.load(Ordering::Relaxed),
            longest_callback_us: KEYBOARD_LONGEST.load(Ordering::Relaxed),
            slow_callbacks: KEYBOARD_SLOW.load(Ordering::Relaxed),
            last_event_had_focus: (focus != 0).then_some(focus == 2),
            last_event_had_caret: (caret != 0).then_some(caret == 2),
        })
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
    fn read_file_clipboard(&self) -> Result<Vec<std::path::PathBuf>> {
        super::file_clipboard::read()
    }
    fn write_file_clipboard(&self, paths: &[std::path::PathBuf]) -> Result<()> {
        super::file_clipboard::write(paths)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "Interactive isolated hook-pressure probe; quit sharing engines and run serially"]
    fn keyboard_delivery_does_not_wait_for_mouse_thread_work() {
        let native = Windows::new();
        let entered = Arc::new(AtomicBool::new(false));
        let seen = Arc::new(AtomicUsize::new(0));
        let busy = entered.clone();
        let keys = seen.clone();
        let (release, blocked) = std::sync::mpsc::channel();
        let blocked = Mutex::new(blocked);
        native
            .start_capture(Arc::new(move |event| {
                match event {
                    CapturedEvent::Move { .. } => {
                        busy.store(true, Ordering::SeqCst);
                        let _ = blocked
                            .lock()
                            .unwrap()
                            .recv_timeout(std::time::Duration::from_secs(1));
                    }
                    CapturedEvent::Input(InputEvent::Key { .. }) => {
                        keys.fetch_add(1, Ordering::SeqCst);
                    }
                    _ => {}
                }
                true
            }))
            .unwrap();
        unsafe {
            let class: Vec<_> = "STATIC\0".encode_utf16().collect();
            let title: Vec<_> = "Hopping Cursor isolated hook pressure probe\0"
                .encode_utf16()
                .collect();
            let fixture = CreateWindowExW(
                WS_EX_TOOLWINDOW,
                class.as_ptr(),
                title.as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                100,
                100,
                420,
                180,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                GetModuleHandleW(std::ptr::null()),
                std::ptr::null(),
            );
            assert!(!fixture.is_null());
            SetForegroundWindow(fixture);
            assert_eq!(
                GetForegroundWindow(),
                fixture,
                "Isolated pressure fixture must own foreground input"
            );
            PostThreadMessageW(
                native.thread_id.load(Ordering::SeqCst),
                MOUSE_PRESSURE_PROBE,
                0,
                0,
            );
            let started = std::time::Instant::now();
            while !entered.load(Ordering::SeqCst)
                && started.elapsed() < std::time::Duration::from_secs(2)
            {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            assert!(entered.load(Ordering::SeqCst));
            let start = std::time::Instant::now();
            for down in [true, false] {
                let input = INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: 0,
                            wScan: 0x12,
                            dwFlags: KEYEVENTF_SCANCODE | if down { 0 } else { KEYEVENTF_KEYUP },
                            time: 0,
                            dwExtraInfo: FOCUS_PROBE_MARKER,
                        },
                    },
                };
                assert_eq!(SendInput(1, &input, std::mem::size_of::<INPUT>() as i32), 1);
            }
            while seen.load(Ordering::SeqCst) < 2
                && start.elapsed() < std::time::Duration::from_millis(250)
            {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            let delivered_while_mouse_busy = seen.load(Ordering::SeqCst) == 2;
            let elapsed = start.elapsed();
            let _ = release.send(());
            native.stop_capture();
            DestroyWindow(fixture);
            assert!(
                delivered_while_mouse_busy && elapsed < std::time::Duration::from_millis(250),
                "Keyboard hook waited for busy mouse thread: {elapsed:?}"
            );
        }
    }

    #[test]
    #[ignore = "Interactive test-owned focus window; quit sharing engines and run serially"]
    fn native_keyboard_delivery_without_a_focused_text_field() {
        let native = Windows::new();
        let observed = Arc::new(Mutex::new(Vec::new()));
        let captured = observed.clone();
        native
            .start_capture(Arc::new(move |event| {
                captured.lock().unwrap().push(event);
                true
            }))
            .unwrap();
        unsafe {
            let class: Vec<_> = "STATIC\0".encode_utf16().collect();
            let title: Vec<_> = "Hopping Cursor isolated keyboard focus probe\0"
                .encode_utf16()
                .collect();
            let fixture = CreateWindowExW(
                WS_EX_TOOLWINDOW,
                class.as_ptr(),
                title.as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                100,
                100,
                420,
                180,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                GetModuleHandleW(std::ptr::null()),
                std::ptr::null(),
            );
            assert!(!fixture.is_null());
            let edit_class: Vec<_> = "EDIT\0".encode_utf16().collect();
            let edit = CreateWindowExW(
                0,
                edit_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | WS_BORDER,
                20,
                40,
                300,
                40,
                fixture,
                std::ptr::null_mut(),
                GetModuleHandleW(std::ptr::null()),
                std::ptr::null(),
            );
            assert!(!edit.is_null());
            SetForegroundWindow(fixture);
            assert_eq!(
                GetForegroundWindow(),
                fixture,
                "Isolated focus fixture must own foreground input"
            );
            let mut results = Vec::new();
            for focused in [std::ptr::null_mut(), fixture, edit, std::ptr::null_mut()] {
                SetFocus(focused);
                assert_eq!(GetFocus(), focused);
                observed.lock().unwrap().clear();
                for _ in 0..8 {
                    for down in [true, false] {
                        let input = INPUT {
                            r#type: INPUT_KEYBOARD,
                            Anonymous: INPUT_0 {
                                ki: KEYBDINPUT {
                                    wVk: 0,
                                    wScan: 0x12,
                                    dwFlags: KEYEVENTF_SCANCODE
                                        | if down { 0 } else { KEYEVENTF_KEYUP },
                                    time: 0,
                                    dwExtraInfo: FOCUS_PROBE_MARKER,
                                },
                            },
                        };
                        assert_eq!(SendInput(1, &input, std::mem::size_of::<INPUT>() as i32), 1);
                    }
                }
                // Hook delivery executes on the native capture thread. Pump
                // this fixture's messages so its foreground queue stays live.
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
                while observed.lock().unwrap().len() < 16 && std::time::Instant::now() < deadline {
                    let mut msg: MSG = std::mem::zeroed();
                    while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                results.push(observed.lock().unwrap().clone());
            }
            // Untagged synthetic input follows the production rejection path.
            // The test-owned STATIC window has no editable document.
            SetFocus(fixture);
            observed.lock().unwrap().clear();
            let ignored = KEYBOARD_INJECTED.load(Ordering::Relaxed);
            for down in [true, false] {
                let input = INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: 0,
                            wScan: 0x12,
                            dwFlags: KEYEVENTF_SCANCODE | if down { 0 } else { KEYEVENTF_KEYUP },
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                assert_eq!(SendInput(1, &input, std::mem::size_of::<INPUT>() as i32), 1);
            }
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            while KEYBOARD_INJECTED.load(Ordering::Relaxed) < ignored + 2
                && std::time::Instant::now() < deadline
            {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            let injection_rejected = observed.lock().unwrap().is_empty()
                && KEYBOARD_INJECTED.load(Ordering::Relaxed) >= ignored + 2;
            DestroyWindow(fixture);
            native.stop_capture();
            assert!(
                injection_rejected,
                "Production injected-event rejection must remain intact"
            );
            for (index, events) in results.iter().enumerate() {
                assert_eq!(
                    events.len(),
                    16,
                    "Focus phase {index}: missing native keyboard deliveries"
                );
                for (i, event) in events.iter().enumerate() {
                    assert!(
                        matches!(event, CapturedEvent::Input(InputEvent::Key {code:8, down}) if *down == (i % 2 == 0))
                    );
                }
            }
        }
    }

    #[test]
    #[ignore = "Interactive desktop probe; quit other sharing engines and run serially"]
    fn parked_motion_reclaim_uses_source_position_and_keeps_first_displacement() {
        let native = Arc::new(Windows::new());
        let observed = Arc::new(Mutex::new(Vec::new()));
        let events = observed.clone();
        let controller = Arc::downgrade(&native);
        native
            .start_capture(Arc::new(move |event| {
                events.lock().unwrap().push(event);
                controller.upgrade().unwrap().set_forwarding(false).unwrap();
                false
            }))
            .unwrap();
        unsafe {
            let mut original: POINT = std::mem::zeroed();
            assert_ne!(GetCursorPos(&mut original), 0);
            native.set_forwarding(true).unwrap();
            let (anchor, origin) = PARKING.get().unwrap().lock().unwrap().unwrap();
            // Feed a synthetic, preprocessed native-hook packet directly. This
            // checks callback/OS plumbing without claiming real device gain.
            let packet = MSLLHOOKSTRUCT {
                pt: POINT {
                    x: anchor.x + 6,
                    y: anchor.y - 3,
                },
                mouseData: 0,
                flags: 0,
                time: 0,
                dwExtraInfo: 0,
            };
            let suppressed = mouse_hook(0, WM_MOUSEMOVE as WPARAM, &packet as *const _ as LPARAM);
            let mut after: POINT = std::mem::zeroed();
            GetCursorPos(&mut after);
            // Restore before assertions so a failed probe leaves the desk usable.
            native.stop_capture();
            SetCursorPos(original.x, original.y);
            assert_eq!(suppressed, 1);
            let captured = observed.lock().unwrap();
            assert!(
                matches!(captured.as_slice(), [CapturedEvent::Move { x, y, dx: 6., dy: -3. }] if *x == origin.x as f64 + 6. && *y == origin.y as f64 - 3.)
            );
            assert!(!CURSOR_HIDDEN.load(Ordering::SeqCst));
            // Desktop clipping is allowed if the saved source was at an edge.
            assert!(
                (after.x - origin.x).abs() <= 6 && (after.y - origin.y).abs() <= 3,
                "Reclaim displacement: {}, {}",
                after.x - origin.x,
                after.y - origin.y
            );
        }
    }
}
