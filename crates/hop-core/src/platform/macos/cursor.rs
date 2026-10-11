//! Cursor control from an ordinary background application.
//!
//! Public Quartz hide/show calls normally require the foreground process.
//! This scoped WindowServer SPI enables only our own connection. It is loaded
//! dynamically, verified by read-back, and never used to control other apps.
//! Missing/changed SPI fails the handoff instead of claiming a hidden cursor.
use super::{
    CFBooleanGetTypeID, CFBooleanGetValue, CFGetTypeID, CFRelease, CFStringCreateWithCString, Ref,
};
use anyhow::{Context, Result};
use std::sync::OnceLock;

type Connection = unsafe extern "C" fn() -> i32;
type CopyProperty = unsafe extern "C" fn(i32, i32, Ref, *mut Ref) -> i32;
type SetProperty = unsafe extern "C" fn(i32, i32, Ref, Ref) -> i32;

#[derive(Clone, Copy)]
struct Api {
    connection: Connection,
    copy: CopyProperty,
    set: SetProperty,
}
extern "C" {
    fn dlsym(handle: Ref, symbol: *const i8) -> Ref;
}
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kCFBooleanTrue: Ref;
    static kCFBooleanFalse: Ref;
}

impl Api {
    fn load() -> Result<Self> {
        static API: OnceLock<Option<Api>> = OnceLock::new();
        API.get_or_init(|| unsafe {
            // Darwin's RTLD_DEFAULT searches already-loaded system frameworks.
            let handle = -2_isize as Ref;
            let connection = dlsym(handle, c"CGSMainConnectionID".as_ptr());
            let copy = dlsym(handle, c"CGSCopyConnectionProperty".as_ptr());
            let set = dlsym(handle, c"CGSSetConnectionProperty".as_ptr());
            if connection.is_null() || copy.is_null() || set.is_null() {
                return None;
            }
            Some(Api {
                connection: std::mem::transmute::<Ref, Connection>(connection),
                copy: std::mem::transmute::<Ref, CopyProperty>(copy),
                set: std::mem::transmute::<Ref, SetProperty>(set),
            })
        })
        .context("Background cursor control is unavailable on this macOS version")
    }
    unsafe fn enabled(self, connection: i32, key: Ref) -> Result<bool> {
        let mut value = std::ptr::null_mut();
        let error = (self.copy)(connection, connection, key, &mut value);
        let enabled = if value.is_null() {
            Ok(false)
        } else {
            let result = if CFGetTypeID(value) == CFBooleanGetTypeID() {
                Ok(CFBooleanGetValue(value))
            } else {
                Err(anyhow::anyhow!("Unexpected background cursor capability"))
            };
            CFRelease(value);
            result
        };
        anyhow::ensure!(
            error == 0,
            "Cannot read background cursor capability ({error})"
        );
        enabled
    }
}

pub(super) struct BackgroundCursor {
    api: Api,
    connection: i32,
    key: usize,
    previous: bool,
    changed: bool,
}
impl BackgroundCursor {
    pub(super) fn enable() -> Result<Self> {
        let api = Api::load()?;
        unsafe {
            let key = CFStringCreateWithCString(
                std::ptr::null_mut(),
                c"SetsCursorInBackground".as_ptr(),
                0x08000100,
            );
            anyhow::ensure!(!key.is_null(), "Cannot create background cursor capability");
            let mut guard = Self {
                api,
                connection: (api.connection)(),
                key: key as usize,
                previous: false,
                changed: false,
            };
            guard.previous = api.enabled(guard.connection, key)?;
            if !guard.previous {
                let error = (api.set)(guard.connection, guard.connection, key, kCFBooleanTrue);
                anyhow::ensure!(
                    error == 0,
                    "Cannot enable background cursor control ({error})"
                );
                guard.changed = true;
            }
            anyhow::ensure!(
                api.enabled(guard.connection, key)?,
                "macOS did not enable background cursor control"
            );
            Ok(guard)
        }
    }
    pub(super) fn restore(&mut self) -> Result<()> {
        if self.changed {
            unsafe {
                let error = (self.api.set)(
                    self.connection,
                    self.connection,
                    self.key as Ref,
                    if self.previous {
                        kCFBooleanTrue
                    } else {
                        kCFBooleanFalse
                    },
                );
                anyhow::ensure!(
                    error == 0,
                    "Cannot restore background cursor capability ({error})"
                );
                self.changed = false;
            }
        }
        Ok(())
    }
}
impl Drop for BackgroundCursor {
    fn drop(&mut self) {
        let _ = self.restore();
        unsafe {
            CFRelease(self.key as Ref);
        }
    }
}
