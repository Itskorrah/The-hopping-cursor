use super::{keys, CaptureCallback, Platform};
use crate::model::*;
use anyhow::{Context, Result};
use std::{
    collections::BTreeSet,
    ffi::c_void,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
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
const MARKER: i64 = 0x484f505043555253;
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
    fn CGEventGetFlags(event: Ref) -> u64;
    fn CGEventSetFlags(event: Ref, flags: u64);
    fn CGEventCreate(source: Ref) -> Ref;
    fn CGEventCreateKeyboardEvent(source: Ref, key: u16, down: bool) -> Ref;
    fn CGEventCreateMouseEvent(source: Ref, kind: u32, point: Point, button: u32) -> Ref;
    fn CGEventCreateScrollWheelEvent(source: Ref, units: u32, count: u32, ...) -> Ref;
    fn CGEventPost(tap: u32, event: Ref);
    fn CGPreflightListenEventAccess() -> bool;
    fn CGGetActiveDisplayList(max: u32, ids: *mut u32, count: *mut u32) -> i32;
    fn CGDisplayBounds(id: u32) -> CGRect;
    fn CGMainDisplayID() -> u32;
    fn CGDisplayPixelsWide(id: u32) -> usize;
}
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
}
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFMachPortCreateRunLoopSource(alloc: Ref, port: Ref, order: isize) -> Ref;
    fn CFRunLoopGetCurrent() -> Ref;
    fn CFRunLoopAddSource(runloop: Ref, source: Ref, mode: Ref);
    fn CFRunLoopRun();
    fn CFRunLoopStop(runloop: Ref);
    fn CFRelease(value: Ref);
    static kCFRunLoopCommonModes: Ref;
}
struct TapContext {
    callback: CaptureCallback,
    tap: Ref,
    modifiers: BTreeSet<u16>,
}
extern "C" fn event_callback(_: Ref, kind: u32, event: Ref, user: Ref) -> Ref {
    unsafe {
        let ctx = &mut *(user as *mut TapContext);
        if kind >= 0xfffffffe {
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
            22 => Some(CapturedEvent::Input(InputEvent::Scroll {
                x: CGEventGetIntegerValueField(event, 12) as f64,
                y: CGEventGetIntegerValueField(event, 11) as f64,
            })),
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
        if captured.is_some_and(|e| (ctx.callback)(e)) {
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
}
impl Mac {
    pub fn new() -> Self {
        Self {
            runloop: AtomicUsize::new(0),
            thread: Mutex::new(None),
            held: Mutex::new(BTreeSet::new()),
            buttons: Mutex::new(BTreeSet::new()),
        }
    }
}
impl Platform for Mac {
    fn name(&self) -> &str {
        "macos"
    }
    fn displays(&self) -> Result<Vec<Display>> {
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
                        id: format!("display-{id}"),
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
    fn permissions(&self) -> Permissions {
        unsafe {
            let injection = AXIsProcessTrusted();
            let capture = CGPreflightListenEventAccess();
            Permissions{capture,injection,clipboard:true,explanation:if capture&&injection{"Accessibility and Input Monitoring granted."}else{"Grant Accessibility and Input Monitoring in System Settings → Privacy & Security, then restart the app."}.into()}
        }
    }
    fn start_capture(&self, cb: CaptureCallback) -> Result<()> {
        let p = self.permissions();
        anyhow::ensure!(p.capture && p.injection, "{}", p.explanation);
        let (tx, rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || unsafe {
            let ctx = Box::new(TapContext {
                callback: cb,
                tap: std::ptr::null_mut(),
                modifiers: BTreeSet::new(),
            });
            let ptr = Box::into_raw(ctx);
            let mask = [1, 2, 3, 4, 5, 6, 7, 10, 11, 12, 22, 25, 26, 27]
                .iter()
                .fold(0u64, |m, n| m | (1 << n));
            let tap = CGEventTapCreate(0, 0, 0, mask, event_callback, ptr as _);
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
        unsafe {
            let e = match event {
                InputEvent::Key { code, down } => CGEventCreateKeyboardEvent(
                    std::ptr::null_mut(),
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
                    CGEventCreateMouseEvent(
                        std::ptr::null_mut(),
                        kind,
                        Point { x: *x, y: *y },
                        button,
                    )
                }
                InputEvent::Button { button, down } => {
                    let current = CGEventCreate(std::ptr::null_mut());
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
                    CGEventCreateMouseEvent(std::ptr::null_mut(), kind, p, *button as u32)
                }
                InputEvent::Scroll { x, y } => {
                    CGEventCreateScrollWheelEvent(std::ptr::null_mut(), 1, 2, *y as i32, *x as i32)
                }
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
            // Mouse/scroll events must carry the mapped held modifiers too.
            // Preserve OS lock/function flags while replacing the four modifiers.
            let mut flags = CGEventGetFlags(e) & !((1 << 17) | (1 << 18) | (1 << 19) | (1 << 20));
            for h in self.held.lock().unwrap().iter() {
                flags |= match h {
                    224 | 228 => 1 << 18,
                    225 | 229 => 1 << 17,
                    226 | 230 => 1 << 19,
                    227 | 231 => 1 << 20,
                    _ => 0,
                };
            }
            CGEventSetFlags(e, flags);
            CGEventSetIntegerValueField(e, 42, MARKER);
            CGEventPost(0, e);
            CFRelease(e);
            Ok(())
        }
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
