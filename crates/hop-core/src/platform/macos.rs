use super::{keys, scroll, CaptureCallback, Platform};
use crate::model::*;
use anyhow::{Context, Result};
use std::{
    collections::BTreeSet,
    ffi::c_void,
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
};
type Ref = *mut c_void;
#[repr(C)]
#[derive(Copy, Clone)]
struct Point {
    x: f64,
    y: f64,
}
#[repr(C)]
#[derive(Copy, Clone)]
struct Size {
    width: f64,
    height: f64,
}
#[repr(C)]
#[derive(Copy, Clone)]
struct CGRect {
    origin: Point,
    size: Size,
}
#[repr(C)]
struct UuidBytes {
    bytes: [u8; 16],
}
fn display_id(id: u32) -> String {
    unsafe {
        let uuid = CGDisplayCreateUUIDFromDisplayID(id);
        if uuid.is_null() {
            return format!("display-{id}");
        }
        let bytes = CFUUIDGetUUIDBytes(uuid);
        CFRelease(uuid);
        format!("mac-{}", hex::encode(bytes.bytes))
    }
}
const MARKER: i64 = 0x484f505043555253;

fn read_natural_scroll() -> bool {
    unsafe {
        // Read the user's global policy without changing OS preferences.
        CFPreferencesSynchronize(
            kCFPreferencesAnyApplication,
            kCFPreferencesCurrentUser,
            kCFPreferencesAnyHost,
        );
        let key = CFStringCreateWithCString(
            std::ptr::null_mut(),
            c"com.apple.swipescrolldirection".as_ptr(),
            0x08000100,
        );
        if key.is_null() {
            return true;
        }
        let value = CFPreferencesCopyValue(
            key,
            kCFPreferencesAnyApplication,
            kCFPreferencesCurrentUser,
            kCFPreferencesAnyHost,
        );
        CFRelease(key);
        if value.is_null() {
            return true;
        } // macOS's natural-scrolling default
        let natural = if CFGetTypeID(value) == CFBooleanGetTypeID() {
            CFBooleanGetValue(value)
        } else {
            true
        };
        CFRelease(value);
        natural
    }
}

fn create_scroll_event(
    source: Ref,
    x: f64,
    y: f64,
    natural: bool,
    scroll: &mut scroll::MacScroll,
) -> Result<Ref> {
    unsafe {
        let current = CGEventCreate(std::ptr::null_mut());
        anyhow::ensure!(!current.is_null(), "Cannot locate destination cursor");
        let location = CGEventGetLocation(current);
        CFRelease(current);
        // Initialize one native line on each axis to obtain Quartz's own
        // line-to-point scale, then replace all delta fields consistently.
        let event = CGEventCreateScrollWheelEvent(source, 1, 2, 1_i32, 1_i32);
        anyhow::ensure!(!event.is_null(), "Cannot create macOS scroll event");
        let scale = CGEventGetIntegerValueField(event, 96).unsigned_abs() as f64;
        let delta = scroll.prepare(x, y, natural, if scale > 0. { scale } else { 8. });
        // Setting integer line fields can also alter fixed/point fields.
        // Set fractional and point values afterwards, preserving small wheels.
        CGEventSetIntegerValueField(event, 11, delta.lines[1] as i64);
        CGEventSetIntegerValueField(event, 12, delta.lines[0] as i64);
        CGEventSetDoubleValueField(event, 93, delta.fixed[1]);
        CGEventSetDoubleValueField(event, 94, delta.fixed[0]);
        CGEventSetIntegerValueField(event, 96, delta.points[1]);
        CGEventSetIntegerValueField(event, 97, delta.points[0]);
        CGEventSetLocation(event, location);
        Ok(event)
    }
}
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        mask: u64,
        callback: extern "C" fn(Ref, u32, Ref, Ref) -> Ref,
        user: Ref,
    ) -> Ref;
    fn CGEventTapEnable(tap: Ref, enable: bool);
    fn CGEventGetLocation(event: Ref) -> Point;
    fn CGEventGetIntegerValueField(event: Ref, field: u32) -> i64;
    fn CGEventSetIntegerValueField(event: Ref, field: u32, value: i64);
    fn CGEventGetDoubleValueField(event: Ref, field: u32) -> f64;
    fn CGEventSetDoubleValueField(event: Ref, field: u32, value: f64);
    fn CGEventSetLocation(event: Ref, point: Point);
    fn CGEventGetFlags(event: Ref) -> u64;
    fn CGEventSetFlags(event: Ref, flags: u64);
    fn CGEventCreate(source: Ref) -> Ref;
    fn CGEventCreateKeyboardEvent(source: Ref, key: u16, down: bool) -> Ref;
    fn CGEventCreateMouseEvent(source: Ref, kind: u32, point: Point, button: u32) -> Ref;
    // wheel1 is fixed, not variadic. On arm64 Darwin fixed arguments use
    // registers while the remaining variadic wheels are passed on the stack.
    fn CGEventCreateScrollWheelEvent(source: Ref, units: u32, count: u32, wheel1: i32, ...) -> Ref;
    fn CGEventPost(tap: u32, event: Ref);
    fn CGWarpMouseCursorPosition(point: Point) -> i32;
    fn CGDisplayHideCursor(display: u32) -> i32;
    fn CGDisplayShowCursor(display: u32) -> i32;
    fn CGEventSourceCreate(state: i32) -> Ref;
    fn CGEventSourceSetUserData(source: Ref, data: i64);
    fn CGEventSourceSetLocalEventsSuppressionInterval(source: Ref, seconds: f64);
    fn CGEventSourceSetLocalEventsFilterDuringSuppressionState(
        source: Ref,
        filter: u32,
        state: u32,
    );
    fn CGPreflightListenEventAccess() -> bool;
    fn CGRequestListenEventAccess() -> bool;
    fn CGGetActiveDisplayList(max: u32, ids: *mut u32, count: *mut u32) -> i32;
    fn CGDisplayBounds(id: u32) -> CGRect;
    fn CGMainDisplayID() -> u32;
    fn CGDisplayPixelsWide(id: u32) -> usize;
    fn CGDisplayCreateUUIDFromDisplayID(id: u32) -> Ref;
}
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: Ref) -> bool;
    static kAXTrustedCheckOptionPrompt: Ref;
}
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFMachPortCreateRunLoopSource(alloc: Ref, port: Ref, order: isize) -> Ref;
    fn CFRunLoopGetCurrent() -> Ref;
    fn CFRunLoopAddSource(runloop: Ref, source: Ref, mode: Ref);
    fn CFRunLoopRun();
    fn CFRunLoopStop(runloop: Ref);
    fn CFRelease(value: Ref);
    fn CFUUIDGetUUIDBytes(value: Ref) -> UuidBytes;
    fn CFStringCreateWithCString(allocator: Ref, value: *const i8, encoding: u32) -> Ref;
    fn CFPreferencesCopyValue(key: Ref, application: Ref, user: Ref, host: Ref) -> Ref;
    fn CFPreferencesSynchronize(application: Ref, user: Ref, host: Ref) -> bool;
    fn CFGetTypeID(value: Ref) -> usize;
    fn CFBooleanGetTypeID() -> usize;
    fn CFBooleanGetValue(value: Ref) -> bool;
    fn CFDictionaryCreate(
        allocator: Ref,
        keys: *const Ref,
        values: *const Ref,
        count: isize,
        key_callbacks: Ref,
        value_callbacks: Ref,
    ) -> Ref;
    static kCFBooleanTrue: Ref;
    static kCFRunLoopCommonModes: Ref;
    static kCFPreferencesAnyApplication: Ref;
    static kCFPreferencesCurrentUser: Ref;
    static kCFPreferencesAnyHost: Ref;
}
#[link(name = "Carbon", kind = "framework")]
extern "C" {
    fn IsSecureEventInputEnabled() -> bool;
}
struct TapContext {
    callback: CaptureCallback,
    tap: Ref,
    modifiers: BTreeSet<u16>,
    parked: Arc<Mutex<Option<Point>>>,
    suppressed: Arc<AtomicU64>,
    natural_scroll: Arc<AtomicBool>,
}
extern "C" fn event_callback(_: Ref, kind: u32, event: Ref, user: Ref) -> Ref {
    unsafe {
        let ctx = &mut *(user as *mut TapContext);
        if kind >= 0xfffffffe {
            ctx.modifiers.clear();
            (ctx.callback)(CapturedEvent::Input(InputEvent::ReleaseAll));
            CGEventTapEnable(ctx.tap, true);
            return event;
        }
        if CGEventGetIntegerValueField(event, 42) == MARKER {
            return event;
        }
        let captured = match kind {
            5 | 6 | 7 | 27 => {
                let p = CGEventGetLocation(event);
                Some(CapturedEvent::Move {
                    x: p.x,
                    y: p.y,
                    dx: CGEventGetIntegerValueField(event, 4) as f64,
                    dy: CGEventGetIntegerValueField(event, 5) as f64,
                })
            }
            1 | 2 | 3 | 4 | 25 | 26 => Some(CapturedEvent::Input(InputEvent::Button {
                button: match kind {
                    1 | 2 => 0,
                    3 | 4 => 1,
                    _ => CGEventGetIntegerValueField(event, 3).clamp(2, 4) as u8,
                },
                down: matches!(kind, 1 | 3 | 25),
            })),
            22 => {
                let (x, y) = if CGEventGetIntegerValueField(event, 88) != 0 {
                    // Continuous events carry points; retain sub-line motion.
                    (
                        CGEventGetIntegerValueField(event, 97) as f64 / 8.,
                        CGEventGetIntegerValueField(event, 96) as f64 / 8.,
                    )
                } else {
                    (
                        CGEventGetDoubleValueField(event, 94),
                        CGEventGetDoubleValueField(event, 93),
                    )
                };
                let [x, y] =
                    scroll::mac_direction(x, y, ctx.natural_scroll.load(Ordering::Relaxed));
                Some(CapturedEvent::Input(InputEvent::Scroll { x, y }))
            }
            10 | 11 => keys::mac_to_hid(CGEventGetIntegerValueField(event, 9) as u16).map(|code| {
                CapturedEvent::Input(InputEvent::Key {
                    code,
                    down: kind == 10,
                })
            }),
            12 => keys::mac_to_hid(CGEventGetIntegerValueField(event, 9) as u16).map(|code| {
                let flags = CGEventGetFlags(event);
                let mask = match code {
                    224 | 228 => 1 << 18,
                    225 | 229 => 1 << 17,
                    226 | 230 => 1 << 19,
                    227 | 231 => 1 << 20,
                    57 => 1 << 16,
                    _ => 0,
                };
                let down = flags & mask != 0 && !ctx.modifiers.contains(&code);
                if down {
                    ctx.modifiers.insert(code);
                } else {
                    ctx.modifiers.remove(&code);
                }
                CapturedEvent::Input(InputEvent::Key { code, down })
            }),
            _ => None,
        };
        let captured =
            captured.or_else(|| matches!(kind, 10..=12).then_some(CapturedEvent::Unsupported));
        if captured.is_some_and(|e| (ctx.callback)(e)) {
            ctx.suppressed.fetch_add(1, Ordering::Relaxed);
            // Quartz can update the hardware cursor before an active session
            // filter deletes the event. Park it without generating another event.
            if matches!(kind, 5 | 6 | 7 | 27) {
                if let Some(point) = *ctx.parked.lock().unwrap() {
                    CGWarpMouseCursorPosition(point);
                }
            }
            std::ptr::null_mut()
        } else {
            event
        }
    }
}
pub struct Mac {
    runloop: AtomicUsize,
    thread: Mutex<Option<JoinHandle<()>>>,
    held: Mutex<BTreeSet<u16>>,
    buttons: Mutex<BTreeSet<u8>>,
    parked: Arc<Mutex<Option<Point>>>,
    source: Mutex<usize>,
    suppressed: Arc<AtomicU64>,
    natural_scroll: Arc<AtomicBool>,
    scroll: Mutex<scroll::MacScroll>,
}
impl Mac {
    pub fn new() -> Self {
        Self {
            runloop: AtomicUsize::new(0),
            thread: Mutex::new(None),
            held: Mutex::new(BTreeSet::new()),
            buttons: Mutex::new(BTreeSet::new()),
            parked: Arc::new(Mutex::new(None)),
            source: Mutex::new(0),
            suppressed: Arc::new(AtomicU64::new(0)),
            natural_scroll: Arc::new(AtomicBool::new(read_natural_scroll())),
            scroll: Mutex::new(scroll::MacScroll::default()),
        }
    }
}
impl Platform for Mac {
    fn natural_scrolling(&self) -> Option<bool> {
        Some(self.natural_scroll.load(Ordering::Relaxed))
    }
    fn name(&self) -> &str {
        "macos"
    }
    fn displays(&self) -> Result<Vec<Display>> {
        // The display refresh runs off the input callback every two seconds.
        // Preference changes are read here; never touch defaults on the hot path.
        self.natural_scroll
            .store(read_natural_scroll(), Ordering::Relaxed);
        unsafe {
            let mut ids = [0; 32];
            let mut count = 0;
            anyhow::ensure!(
                CGGetActiveDisplayList(32, ids.as_mut_ptr(), &mut count) == 0,
                "Cannot enumerate displays"
            );
            Ok(ids[..count as usize]
                .iter()
                .map(|id| {
                    let b = CGDisplayBounds(*id);
                    Display {
                        id: display_id(*id),
                        name: format!("Display {id}"),
                        bounds: Rect {
                            x: b.origin.x,
                            y: b.origin.y,
                            width: b.size.width,
                            height: b.size.height,
                        },
                        scale: CGDisplayPixelsWide(*id) as f64 / b.size.width,
                        primary: *id == CGMainDisplayID(),
                    }
                })
                .collect())
        }
    }
    fn display_id_aliases(&self) -> Result<std::collections::BTreeMap<String, String>> {
        unsafe {
            let mut ids = [0; 32];
            let mut count = 0;
            anyhow::ensure!(
                CGGetActiveDisplayList(32, ids.as_mut_ptr(), &mut count) == 0,
                "Cannot enumerate displays"
            );
            Ok(ids[..count as usize]
                .iter()
                .map(|id| (format!("display-{id}"), display_id(*id)))
                .collect())
        }
    }
    fn session_available(&self) -> bool {
        unsafe { !IsSecureEventInputEnabled() }
    }
    fn input_available(&self) -> bool {
        let p = self.permissions();
        p.capture && p.injection
    }
    fn permissions(&self) -> Permissions {
        unsafe {
            let injection = AXIsProcessTrusted();
            let capture = CGPreflightListenEventAccess();
            Permissions{capture,injection,clipboard:true,explanation:if capture&&injection{"Accessibility and Input Monitoring granted."}else{"Grant Accessibility and Input Monitoring in System Settings → Privacy & Security, then restart the app."}.into()}
        }
    }
    fn request_permissions(&self) -> Permissions {
        unsafe {
            if !AXIsProcessTrusted() {
                let key = kAXTrustedCheckOptionPrompt;
                let value = kCFBooleanTrue;
                let options = CFDictionaryCreate(
                    std::ptr::null_mut(),
                    &key,
                    &value,
                    1,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                if !options.is_null() {
                    AXIsProcessTrustedWithOptions(options);
                    CFRelease(options);
                }
            }
            if !CGPreflightListenEventAccess() {
                CGRequestListenEventAccess();
            }
        }
        self.permissions()
    }
    fn start_capture(&self, cb: CaptureCallback) -> Result<()> {
        let p = self.permissions();
        anyhow::ensure!(p.capture && p.injection, "{}", p.explanation);
        let (tx, rx) = std::sync::mpsc::channel();
        let parked = self.parked.clone();
        let suppressed = self.suppressed.clone();
        let natural_scroll = self.natural_scroll.clone();
        let handle = std::thread::spawn(move || unsafe {
            let ctx = Box::new(TapContext {
                callback: cb,
                tap: std::ptr::null_mut(),
                modifiers: BTreeSet::new(),
                parked,
                suppressed,
                natural_scroll,
            });
            let ptr = Box::into_raw(ctx);
            let mask = [1, 2, 3, 4, 5, 6, 7, 10, 11, 12, 22, 25, 26, 27]
                .iter()
                .fold(0u64, |m, n| m | (1 << n));
            // An active session filter can discard input before application
            // delivery without relying on privileged HID-level interception.
            let tap = CGEventTapCreate(1, 0, 0, mask, event_callback, ptr as _);
            if tap.is_null() {
                let _ = tx.send(Err(
                    "Could not create macOS input tap. Check permissions.".to_string()
                ));
                drop(Box::from_raw(ptr));
                return;
            }
            (*ptr).tap = tap;
            let source = CFMachPortCreateRunLoopSource(std::ptr::null_mut(), tap, 0);
            let runloop = CFRunLoopGetCurrent();
            CFRunLoopAddSource(runloop, source, kCFRunLoopCommonModes);
            CGEventTapEnable(tap, true);
            let _ = tx.send(Ok(runloop as usize));
            CFRunLoopRun();
            CGEventTapEnable(tap, false);
            CFRelease(source);
            CFRelease(tap);
            drop(Box::from_raw(ptr));
        });
        match rx.recv().context("Input tap thread failed")? {
            Ok(r) => {
                self.runloop.store(r, Ordering::SeqCst);
                *self.thread.lock().unwrap() = Some(handle);
                Ok(())
            }
            Err(e) => {
                let _ = handle.join();
                anyhow::bail!(e)
            }
        }
    }
    fn stop_capture(&self) {
        let _ = self.set_forwarding(false);
        let r = self.runloop.swap(0, Ordering::SeqCst);
        if r != 0 {
            unsafe {
                CFRunLoopStop(r as Ref);
            }
            if let Some(h) = self.thread.lock().unwrap().take() {
                let _ = h.join();
            }
        }
    }
    fn set_forwarding(&self, forwarding: bool) -> Result<()> {
        let mut parked = self.parked.lock().unwrap();
        unsafe {
            if forwarding && parked.is_none() {
                let event = CGEventCreate(std::ptr::null_mut());
                anyhow::ensure!(!event.is_null(), "Cannot locate source cursor");
                let point = CGEventGetLocation(event);
                CFRelease(event);
                anyhow::ensure!(CGDisplayHideCursor(0) == 0, "Cannot hide source cursor");
                *parked = Some(point);
            } else if !forwarding && parked.is_some() {
                anyhow::ensure!(CGDisplayShowCursor(0) == 0, "Cannot restore source cursor");
                *parked = None;
            }
        }
        Ok(())
    }
    fn suppressed_events(&self) -> Option<u64> {
        Some(self.suppressed.load(Ordering::Relaxed))
    }
    fn inject(&self, event: &InputEvent) -> Result<()> {
        if *event == InputEvent::ReleaseAll {
            self.scroll.lock().unwrap().reset();
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
        unsafe {
            let mut source = self.source.lock().unwrap();
            if *source == 0 {
                let value = CGEventSourceCreate(-1); // private remote-input state
                anyhow::ensure!(!value.is_null(), "Cannot create macOS input source");
                CGEventSourceSetUserData(value, MARKER);
                CGEventSourceSetLocalEventsSuppressionInterval(value, 0.);
                CGEventSourceSetLocalEventsFilterDuringSuppressionState(value, 7, 0);
                CGEventSourceSetLocalEventsFilterDuringSuppressionState(value, 7, 1);
                *source = value as usize;
            }
            let source = *source as Ref;
            let e = match event {
                InputEvent::Key { code, down } => CGEventCreateKeyboardEvent(
                    source,
                    keys::hid_to_mac(*code).context("Unsupported keyboard key")?,
                    *down,
                ),
                InputEvent::Move { x, y } => {
                    let buttons = self.buttons.lock().unwrap();
                    let (kind, button) = match buttons.iter().next() {
                        Some(0) => (6, 0),
                        Some(1) => (7, 1),
                        Some(button) => (27, *button as u32),
                        None => (5, 0),
                    };
                    CGEventCreateMouseEvent(source, kind, Point { x: *x, y: *y }, button)
                }
                InputEvent::Button { button, down } => {
                    let current = CGEventCreate(std::ptr::null_mut());
                    anyhow::ensure!(!current.is_null(), "Cannot locate destination cursor");
                    let p = CGEventGetLocation(current);
                    CFRelease(current);
                    let kind = match (*button, *down) {
                        (0, true) => 1,
                        (0, false) => 2,
                        (1, true) => 3,
                        (1, false) => 4,
                        (_, true) => 25,
                        (_, false) => 26,
                    };
                    CGEventCreateMouseEvent(source, kind, p, *button as u32)
                }
                InputEvent::Scroll { x, y } => create_scroll_event(
                    source,
                    *x,
                    *y,
                    self.natural_scroll.load(Ordering::Relaxed),
                    &mut self.scroll.lock().unwrap(),
                )?,
                InputEvent::ReleaseAll => unreachable!(),
            };
            anyhow::ensure!(!e.is_null(), "macOS event creation failed");
            match event {
                InputEvent::Key { code, down } => {
                    let mut held = self.held.lock().unwrap();
                    if *down {
                        held.insert(*code);
                    } else {
                        held.remove(code);
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
            // Rebuild modifiers from authenticated remote state for every
            // event, including mouse/scroll. Inherited Fn can turn E into
            // the macOS emoji shortcut even with a private event source.
            let flags = keys::mac_remote_flags(
                CGEventGetFlags(e),
                self.held.lock().unwrap().iter().copied(),
            );
            CGEventSetFlags(e, flags);
            CGEventSetIntegerValueField(e, 42, MARKER);
            CGEventPost(0, e);
            CFRelease(e);
            Ok(())
        }
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
        unsafe {
            #[link(name = "objc")]
            extern "C" {
                fn objc_getClass(name: *const i8) -> Ref;
                fn sel_registerName(name: *const i8) -> Ref;
                fn objc_msgSend();
                fn objc_autoreleasePoolPush() -> Ref;
                fn objc_autoreleasePoolPop(pool: Ref);
            }
            let pool = objc_autoreleasePoolPush();
            let msg: unsafe extern "C" fn(Ref, Ref) -> Ref =
                std::mem::transmute(objc_msgSend as *const ());
            let count: unsafe extern "C" fn(Ref, Ref) -> isize =
                std::mem::transmute(objc_msgSend as *const ());
            let pb = msg(
                objc_getClass(c"NSPasteboard".as_ptr()),
                sel_registerName(c"generalPasteboard".as_ptr()),
            );
            let result = if pb.is_null() {
                None
            } else {
                Some(count(pb, sel_registerName(c"changeCount".as_ptr())) as u64)
            };
            objc_autoreleasePoolPop(pool);
            result
        }
    }
    fn write_clipboard(&self, p: &[ClipboardPart]) -> Result<()> {
        super::clipboard::write(p)
    }
}
impl Drop for Mac {
    fn drop(&mut self) {
        self.stop_capture();
        let _ = self.inject(&InputEvent::ReleaseAll);
        let source = *self.source.get_mut().unwrap();
        if source != 0 {
            unsafe {
                CFRelease(source as Ref);
            }
        }
    }
}

pub fn read_html() -> Option<String> {
    unsafe {
        #[link(name = "objc")]
        extern "C" {
            fn objc_getClass(name: *const i8) -> Ref;
            fn sel_registerName(name: *const i8) -> Ref;
            fn objc_msgSend();
            fn objc_autoreleasePoolPush() -> Ref;
            fn objc_autoreleasePoolPop(pool: Ref);
        }
        let pool = objc_autoreleasePoolPush();
        let msg: unsafe extern "C" fn(Ref, Ref) -> Ref =
            std::mem::transmute(objc_msgSend as *const ());
        let msg_string: unsafe extern "C" fn(Ref, Ref, *const i8) -> Ref =
            std::mem::transmute(objc_msgSend as *const ());
        let msg_type: unsafe extern "C" fn(Ref, Ref, Ref) -> Ref =
            std::mem::transmute(objc_msgSend as *const ());
        let pb = msg(
            objc_getClass(c"NSPasteboard".as_ptr()),
            sel_registerName(c"generalPasteboard".as_ptr()),
        );
        let ty = msg_string(
            objc_getClass(c"NSString".as_ptr()),
            sel_registerName(c"stringWithUTF8String:".as_ptr()),
            c"public.html".as_ptr(),
        );
        let val = msg_type(pb, sel_registerName(c"stringForType:".as_ptr()), ty);
        let result = if val.is_null() {
            None
        } else {
            let p = msg(val, sel_registerName(c"UTF8String".as_ptr())) as *const i8;
            if p.is_null() {
                None
            } else {
                Some(std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned())
            }
        };
        objc_autoreleasePoolPop(pool);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quartz_scroll_constructor_uses_fixed_primary_wheel_argument() {
        // Apple's C ABI has four fixed arguments; only wheels 2/3 are variadic.
        // Apple Silicon passes fixed and variadic arguments differently.
        let create: unsafe extern "C" fn(Ref, u32, u32, i32, ...) -> Ref =
            CGEventCreateScrollWheelEvent;
        unsafe {
            let event = create(std::ptr::null_mut(), 1, 2, 3_i32, -2_i32);
            assert!(!event.is_null());
            assert_eq!(CGEventGetIntegerValueField(event, 11), 3);
            assert_eq!(CGEventGetIntegerValueField(event, 12), -2);
            CFRelease(event);
        }
    }

    #[test]
    fn mac_scroll_event_preserves_fractional_axes_and_destination_direction_without_posting() {
        unsafe {
            let mut scroll = scroll::MacScroll::default();
            for (natural, x, y) in [(false, -0.125, 0.5), (true, 0.125, -0.5)] {
                let event =
                    create_scroll_event(std::ptr::null_mut(), 0.125, 0.5, natural, &mut scroll)
                        .unwrap();
                assert!((CGEventGetDoubleValueField(event, 94) - x).abs() < 1. / 65536.);
                assert!((CGEventGetDoubleValueField(event, 93) - y).abs() < 1. / 65536.);
                assert_eq!(CGEventGetIntegerValueField(event, 88), 0);
                CFRelease(event);
            }
        }
    }

    #[test]
    fn mac_active_filter_discards_clicks_and_unmapped_keys_without_posting_events() {
        // Construct isolated events and call the filter directly. Never post
        // test input into the user's session or request native permissions.
        unsafe {
            let source = CGEventSourceCreate(-1);
            assert!(!source.is_null());
            let click = CGEventCreateMouseEvent(source, 3, Point { x: 100., y: 100. }, 1);
            let unmapped = CGEventCreateKeyboardEvent(source, u16::MAX, true);
            let mut context = TapContext {
                callback: Arc::new(|_| true),
                tap: std::ptr::null_mut(),
                modifiers: BTreeSet::new(),
                parked: Arc::new(Mutex::new(None)),
                suppressed: Arc::new(AtomicU64::new(0)),
                natural_scroll: Arc::new(AtomicBool::new(false)),
            };
            for (kind, event) in [(3, click), (10, unmapped)] {
                assert!(!event.is_null());
                assert!(event_callback(
                    std::ptr::null_mut(),
                    kind,
                    event,
                    &mut context as *mut _ as Ref
                )
                .is_null());
            }
            context.callback = Arc::new(|_| false);
            assert_eq!(
                event_callback(
                    std::ptr::null_mut(),
                    3,
                    click,
                    &mut context as *mut _ as Ref
                ),
                click
            );
            context.callback = Arc::new(|_| panic!("Own injected input must bypass capture"));
            CGEventSetIntegerValueField(click, 42, MARKER);
            assert_eq!(
                event_callback(
                    std::ptr::null_mut(),
                    3,
                    click,
                    &mut context as *mut _ as Ref
                ),
                click
            );
            CFRelease(click);
            CFRelease(unmapped);
            CFRelease(source);
        }
    }
}
