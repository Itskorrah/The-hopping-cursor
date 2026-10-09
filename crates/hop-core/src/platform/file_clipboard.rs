//! Existing-file clipboard adapters. Remote paths are never injected into the OS.
use anyhow::Result;
use std::path::PathBuf;

#[cfg(target_os = "macos")]
mod native {
    use super::*;
    use std::ffi::{c_void, CStr, CString};
    type Id = *mut c_void;
    #[link(name = "AppKit", kind = "framework")]
    extern "C" {
        static NSPasteboardURLReadingFileURLsOnlyKey: Id;
    }
    #[link(name = "objc")]
    extern "C" {
        fn objc_getClass(name: *const i8) -> Id;
        fn sel_registerName(name: *const i8) -> Id;
        fn objc_msgSend();
        fn objc_autoreleasePoolPush() -> Id;
        fn objc_autoreleasePoolPop(pool: Id);
    }
    struct Pool(Id);
    impl Pool {
        fn new() -> Self {
            Self(unsafe { objc_autoreleasePoolPush() })
        }
    }
    impl Drop for Pool {
        fn drop(&mut self) {
            unsafe { objc_autoreleasePoolPop(self.0) }
        }
    }
    unsafe fn send(object: Id, selector: &CStr) -> Id {
        let call: unsafe extern "C" fn(Id, Id) -> Id =
            std::mem::transmute(objc_msgSend as *const ());
        call(object, sel_registerName(selector.as_ptr()))
    }
    unsafe fn with(object: Id, selector: &CStr, value: Id) -> Id {
        let call: unsafe extern "C" fn(Id, Id, Id) -> Id =
            std::mem::transmute(objc_msgSend as *const ());
        call(object, sel_registerName(selector.as_ptr()), value)
    }
    pub fn read() -> Result<Vec<PathBuf>> {
        let _pool = Pool::new();
        let board = unsafe {
            send(
                objc_getClass(c"NSPasteboard".as_ptr()),
                c"generalPasteboard",
            )
        };
        read_board(board)
    }
    fn read_board(board: Id) -> Result<Vec<PathBuf>> {
        anyhow::ensure!(!board.is_null(), "Native pasteboard is unavailable");
        unsafe {
            let classes = with(
                objc_getClass(c"NSArray".as_ptr()),
                c"arrayWithObject:",
                objc_getClass(c"NSURL".as_ptr()),
            );
            let read: unsafe extern "C" fn(Id, Id, Id, Id) -> Id =
                std::mem::transmute(objc_msgSend as *const ());
            let number: unsafe extern "C" fn(Id, Id, bool) -> Id =
                std::mem::transmute(objc_msgSend as *const ());
            let only_files = number(
                objc_getClass(c"NSNumber".as_ptr()),
                sel_registerName(c"numberWithBool:".as_ptr()),
                true,
            );
            let options = read(
                objc_getClass(c"NSDictionary".as_ptr()),
                sel_registerName(c"dictionaryWithObject:forKey:".as_ptr()),
                only_files,
                NSPasteboardURLReadingFileURLsOnlyKey,
            );
            let urls = read(
                board,
                sel_registerName(c"readObjectsForClasses:options:".as_ptr()),
                classes,
                options,
            );
            let count: unsafe extern "C" fn(Id, Id) -> usize =
                std::mem::transmute(objc_msgSend as *const ());
            let length = count(urls, sel_registerName(c"count".as_ptr()));
            anyhow::ensure!(length <= 100, "Copy at most 100 files at once");
            let item: unsafe extern "C" fn(Id, Id, usize) -> Id =
                std::mem::transmute(objc_msgSend as *const ());
            let boolean: unsafe extern "C" fn(Id, Id) -> bool =
                std::mem::transmute(objc_msgSend as *const ());
            let mut paths = Vec::new();
            for index in 0..length {
                let url = item(urls, sel_registerName(c"objectAtIndex:".as_ptr()), index);
                if !boolean(url, sel_registerName(c"isFileURL".as_ptr())) {
                    continue;
                }
                let path = send(send(url, c"path"), c"fileSystemRepresentation") as *const i8;
                if !path.is_null() {
                    use std::os::unix::ffi::OsStrExt;
                    paths.push(PathBuf::from(std::ffi::OsStr::from_bytes(
                        CStr::from_ptr(path).to_bytes(),
                    )));
                }
            }
            Ok(paths)
        }
    }
    pub fn write(paths: &[PathBuf]) -> Result<()> {
        let _pool = Pool::new();
        let board = unsafe {
            send(
                objc_getClass(c"NSPasteboard".as_ptr()),
                c"generalPasteboard",
            )
        };
        write_board(board, paths)
    }
    fn write_board(board: Id, paths: &[PathBuf]) -> Result<()> {
        anyhow::ensure!(!board.is_null(), "Native pasteboard is unavailable");
        unsafe {
            let array = send(objc_getClass(c"NSMutableArray".as_ptr()), c"array");
            let string: unsafe extern "C" fn(Id, Id, *const i8) -> Id =
                std::mem::transmute(objc_msgSend as *const ());
            for path in paths {
                // Transfer filenames already require Unicode. Reject unsupported paths.
                let path = CString::new(
                    path.to_str()
                        .ok_or_else(|| anyhow::anyhow!("File path must be Unicode"))?,
                )?;
                let path = string(
                    objc_getClass(c"NSString".as_ptr()),
                    sel_registerName(c"stringWithUTF8String:".as_ptr()),
                    path.as_ptr(),
                );
                let url = with(objc_getClass(c"NSURL".as_ptr()), c"fileURLWithPath:", path);
                anyhow::ensure!(!url.is_null(), "Could not create file URL");
                let add: unsafe extern "C" fn(Id, Id, Id) =
                    std::mem::transmute(objc_msgSend as *const ());
                add(array, sel_registerName(c"addObject:".as_ptr()), url);
            }
            send(board, c"clearContents");
            let write: unsafe extern "C" fn(Id, Id, Id) -> bool =
                std::mem::transmute(objc_msgSend as *const ());
            anyhow::ensure!(
                write(board, sel_registerName(c"writeObjects:".as_ptr()), array),
                "Could not copy received file to pasteboard"
            );
            Ok(())
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn mac_file_urls_round_trip_on_isolated_pasteboard() {
            let _pool = Pool::new();
            unsafe {
                let board = send(
                    objc_getClass(c"NSPasteboard".as_ptr()),
                    c"pasteboardWithUniqueName",
                );
                assert!(read_board(board).unwrap().is_empty());
                let directory = tempfile::tempdir().unwrap();
                let paths = ["hop test-é.txt", "第二.txt"]
                    .iter()
                    .map(|name| {
                        let path = directory.path().join(name);
                        std::fs::write(&path, b"synthetic").unwrap();
                        // macOS returns filesystem-normalized Unicode paths.
                        std::fs::canonicalize(path).unwrap()
                    })
                    .collect::<Vec<_>>();
                write_board(board, &paths).unwrap();
                let received = read_board(board).unwrap();
                let count: unsafe extern "C" fn(Id, Id) -> usize =
                    std::mem::transmute(objc_msgSend as *const ());
                assert_eq!(
                    received.len(),
                    paths.len(),
                    "Native pasteboard contains {} items",
                    count(
                        send(board, c"pasteboardItems"),
                        sel_registerName(c"count".as_ptr())
                    )
                );
                // NSURL decomposes Unicode even on normalization-preserving APFS.
                // Verify that both paths identify the same actual file.
                use std::os::unix::fs::MetadataExt;
                for (actual, expected) in received.iter().zip(&paths) {
                    let actual = std::fs::metadata(actual).unwrap();
                    let expected = std::fs::metadata(expected).unwrap();
                    assert_eq!(
                        (actual.dev(), actual.ino()),
                        (expected.dev(), expected.ino())
                    );
                }
                let release: unsafe extern "C" fn(Id, Id) =
                    std::mem::transmute(objc_msgSend as *const ());
                release(board, sel_registerName(c"releaseGlobally".as_ptr()));
            }
        }
    }
}
#[cfg(target_os = "windows")]
mod native {
    use super::*;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use windows_sys::Win32::{
        Foundation::GlobalFree,
        System::{DataExchange::*, Memory::*},
        UI::{
            Shell::{DragQueryFileW, DROPFILES},
            WindowsAndMessaging::{CreateWindowExW, DestroyWindow, HWND_MESSAGE},
        },
    };
    const FILES: u32 = 15; // CF_HDROP
    struct Clipboard(windows_sys::Win32::Foundation::HWND);
    impl Clipboard {
        fn open() -> Result<Self> {
            anyhow::ensure!(
                unsafe { OpenClipboard(std::ptr::null_mut()) } != 0,
                "Clipboard is busy; retry"
            );
            Ok(Self(std::ptr::null_mut()))
        }
    }
    impl Drop for Clipboard {
        fn drop(&mut self) {
            unsafe {
                CloseClipboard();
                if !self.0.is_null() {
                    DestroyWindow(self.0);
                }
            }
        }
    }
    fn open_writer() -> Result<Clipboard> {
        let class = [83u16, 84, 65, 84, 73, 67, 0]; // STATIC
        let window = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                std::ptr::null(),
                0,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        };
        anyhow::ensure!(!window.is_null(), "Could not create clipboard owner");
        if unsafe { OpenClipboard(window) } == 0 {
            unsafe {
                DestroyWindow(window);
            }
            anyhow::bail!("Clipboard is busy; retry");
        }
        Ok(Clipboard(window))
    }
    pub fn read() -> Result<Vec<PathBuf>> {
        let _clipboard = Clipboard::open()?;
        unsafe {
            let handle = GetClipboardData(FILES);
            if handle.is_null() {
                return Ok(vec![]);
            }
            let count = DragQueryFileW(handle, u32::MAX, std::ptr::null_mut(), 0);
            anyhow::ensure!(count <= 100, "Copy at most 100 files at once");
            let mut paths = Vec::new();
            for index in 0..count {
                let length = DragQueryFileW(handle, index, std::ptr::null_mut(), 0);
                anyhow::ensure!((1..32768).contains(&length), "Invalid copied file path");
                let mut path = vec![0u16; length as usize + 1];
                anyhow::ensure!(
                    DragQueryFileW(handle, index, path.as_mut_ptr(), path.len() as u32) == length,
                    "Copied file list changed"
                );
                paths.push(PathBuf::from(std::ffi::OsString::from_wide(
                    &path[..length as usize],
                )));
            }
            Ok(paths)
        }
    }
    pub fn write(paths: &[PathBuf]) -> Result<()> {
        let mut names = Vec::new();
        for path in paths {
            names.extend(path.as_os_str().encode_wide());
            names.push(0);
        }
        names.push(0);
        let size = std::mem::size_of::<DROPFILES>() + names.len() * 2;
        unsafe {
            let memory = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, size);
            anyhow::ensure!(!memory.is_null(), "Could not allocate file clipboard");
            let pointer = GlobalLock(memory) as *mut u8;
            if pointer.is_null() {
                GlobalFree(memory);
                anyhow::bail!("Could not lock file clipboard");
            }
            let header = pointer as *mut DROPFILES;
            (*header).pFiles = std::mem::size_of::<DROPFILES>() as u32;
            (*header).fWide = 1;
            std::ptr::copy_nonoverlapping(
                names.as_ptr() as *const u8,
                pointer.add(std::mem::size_of::<DROPFILES>()),
                names.len() * 2,
            );
            GlobalUnlock(memory);
            let clipboard = match open_writer() {
                Ok(c) => c,
                Err(e) => {
                    GlobalFree(memory);
                    return Err(e);
                }
            };
            if EmptyClipboard() == 0 || SetClipboardData(FILES, memory).is_null() {
                GlobalFree(memory);
                anyhow::bail!("Could not copy received file to clipboard");
            }
            drop(clipboard); // Windows now owns the allocated memory.
            Ok(())
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        #[ignore = "Replaces the real clipboard; run explicitly on a disposable Windows CI runner"]
        fn windows_file_clipboard_round_trip() {
            let directory = tempfile::tempdir().unwrap();
            let paths = ["hop test-é.txt", "第二.txt"]
                .iter()
                .map(|name| {
                    let path = directory.path().join(name);
                    std::fs::write(&path, b"synthetic").unwrap();
                    path
                })
                .collect::<Vec<_>>();
            write(&paths).unwrap();
            assert_eq!(read().unwrap(), paths);
        }
    }
}
pub fn read() -> Result<Vec<PathBuf>> {
    let before = super::clipboard_privacy::stamp()?;
    if before.1 {
        anyhow::bail!("Source application marked this clipboard private or temporary");
    }
    let paths = native::read()?;
    anyhow::ensure!(
        super::clipboard_privacy::stamp()? == before,
        "Clipboard changed; copy files again"
    );
    Ok(paths)
}
pub fn write(paths: &[PathBuf]) -> Result<()> {
    anyhow::ensure!(
        !paths.is_empty() && paths.len() <= 100 && paths.iter().all(|p| p.is_absolute()),
        "Select 1–100 local files"
    );
    native::write(paths)
}
