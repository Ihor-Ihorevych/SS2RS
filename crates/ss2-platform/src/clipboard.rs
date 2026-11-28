//! The system clipboard (T10-035, RFC 0003 amendment 1): [`SystemClipboard`] reads the text a player copied, so that it can be pasted
//! into a text field (the folder path of the first-run wizard).
//!
//! On Windows it reads `CF_UNICODETEXT` through `windows-sys` (the only `unsafe` of this crate, in the small `win32` module below).
//! `winit` has no clipboard, and the clipboard crates either bring a licence the workspace does not allow (`arboard` needs
//! `clipboard-win`, BSL-1.0) or a much larger dependency tree. On other systems there is no clipboard yet: [`SystemClipboard::text`]
//! returns `None`, so pasting does nothing (X11 and Wayland need a connection to the display server, macOS the pasteboard; a task of
//! its own, `docs/roadmap/T10-ui.md`).

use ss2_platform_api::Clipboard;

/// The clipboard of the machine.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClipboard;

impl Clipboard for SystemClipboard {
    fn text(&mut self) -> Option<String> {
        read_text()
    }
}

#[cfg(windows)]
fn read_text() -> Option<String> {
    let text = win32::read_unicode_text();
    if text.is_none() {
        tracing::debug!("the clipboard holds no text, or another program has it open");
    }
    text
}

#[cfg(not(windows))]
fn read_text() -> Option<String> {
    None
}

/// The whole `unsafe` of the crate (`docs/CODING_STANDARDS.md`): open the clipboard, copy its UTF-16 text out, close it again.
#[cfg(windows)]
#[allow(unsafe_code)]
mod win32 {
    use std::ptr;

    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, OpenClipboard,
    };
    use windows_sys::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
    use windows_sys::Win32::System::Ole::CF_UNICODETEXT;

    /// Closes the clipboard when dropped, so that no early return leaves it open (which would block every other program's copy and
    /// paste).
    struct Open;

    impl Drop for Open {
        fn drop(&mut self) {
            // SAFETY: only built after `OpenClipboard` succeeded on this thread, so this thread owns the open clipboard.
            unsafe {
                CloseClipboard();
            }
        }
    }

    /// The clipboard's text, or `None` when it holds none or another program has it open.
    pub(super) fn read_unicode_text() -> Option<String> {
        // SAFETY: a null window associates the open clipboard with this task; nothing else is passed.
        if unsafe { OpenClipboard(ptr::null_mut()) } == 0 {
            return None;
        }
        let _open = Open;
        // SAFETY: the clipboard is open (above). The handle stays owned by the clipboard: it is only read, never freed.
        let handle = unsafe { GetClipboardData(u32::from(CF_UNICODETEXT)) };
        if handle.is_null() {
            return None;
        }
        // SAFETY: `handle` is the clipboard's global memory object for CF_UNICODETEXT, valid while the clipboard is open; locking it
        // gives a pointer to its bytes until the unlock below.
        let data = unsafe { GlobalLock(handle) }.cast::<u16>();
        if data.is_null() {
            return None;
        }
        // SAFETY: the same live handle; its size is that of the locked block, in bytes.
        let units = unsafe { GlobalSize(handle) } / 2;
        // SAFETY: `data` points to the locked block of at least `units` UTF-16 code units, which nothing writes while the clipboard
        // is open, and u16 needs only 2-byte alignment, which global memory always has. The slice is dropped before the unlock.
        let text = {
            let all = unsafe { std::slice::from_raw_parts(data, units) };
            let len = all.iter().position(|&u| u == 0).unwrap_or(all.len());
            String::from_utf16_lossy(&all[..len])
        };
        // SAFETY: balances the `GlobalLock` above on the same handle; `data` is not used afterwards.
        unsafe {
            GlobalUnlock(handle);
        }
        Some(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only reads (a test must not change the clipboard of the person running it): whatever is there comes back as text without
    /// the terminating NUL, or not at all.
    #[test]
    fn reading_the_clipboard_never_fails_and_stops_at_the_terminator() {
        if let Some(text) = SystemClipboard.text() {
            assert!(!text.contains('\0'));
        }
    }
}
