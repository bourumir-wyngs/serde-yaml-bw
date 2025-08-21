use std::fmt::{self, Debug, Display, Write as _};
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::slice;
use std::str;

/// Maximum length of libyaml strings such as anchor and tag names.
///
/// The limit is enforced while scanning null-terminated strings to avoid
/// reading arbitrary memory when libyaml provides malformed data.
pub const MAX_NAME_LENGTH: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CStrError;

/// Wrapper around a null-terminated C string used by libyaml.
///
/// `CStr` is parametrized by the lifetime of the referenced bytes. The type is
/// only `Send` and `Sync` when the bytes live for the `'static` lifetime, which
/// ensures they remain valid across threads.

#[derive(Copy, Clone)]
pub(crate) struct CStr<'a> {
    ptr: NonNull<u8>,
    marker: PhantomData<&'a [u8]>,
}

// CStr only implements Send and Sync for pointers that are valid for the
// `'static` lifetime. This prevents accidentally sending a reference to data
// that may be freed before another thread finishes using it.
// SAFETY: A `CStr<'static>` points to bytes that live for the entire program,
// so sharing or sending it between threads cannot cause dangling references.
unsafe impl Send for CStr<'static> {}
// SAFETY: As above, the referenced bytes outlive all threads making it safe to
// share `CStr<'static>` across threads.
unsafe impl Sync for CStr<'static> {}

impl<'a> CStr<'a> {
    #[cfg(test)]
    pub fn from_bytes_with_nul(bytes: &'static [u8]) -> Self {
        assert_eq!(bytes.last(), Some(&b'\0'));
        let ptr = NonNull::from(bytes).cast();
        // SAFETY: `bytes` is checked to be NUL-terminated and originates from a
        // static slice, so the pointer is valid for `'static`.
        unsafe { Self::from_ptr(ptr) }
    }

    /// # Safety
    ///
    /// - `ptr` must be non-null.
    /// - `ptr` must point to a valid NUL-terminated byte sequence.
    /// - The pointed-to data must remain valid for the returned `CStr`'s lifetime.
    // SAFETY: `ptr` must be non-null and point to a valid NUL-terminated string
    // that lives for at least `'a`.
    pub unsafe fn from_ptr(ptr: NonNull<i8>) -> Self {
        CStr {
            ptr: ptr.cast(),
            marker: PhantomData,
        }
    }

    pub fn len(self) -> Result<usize, CStrError> {
        let start = self.ptr.as_ptr();
        let mut end = start;
        let mut len = 0usize;
        // SAFETY: `self.ptr` points to a valid NUL-terminated string. We bound
        // the scan by `MAX_NAME_LENGTH` to avoid reading past the terminator.
        unsafe {
            while *end != 0 {
                if len >= MAX_NAME_LENGTH {
                    return Err(CStrError);
                }
                end = end.add(1);
                len += 1;
            }
            Ok(len)
        }
    }

    pub fn to_bytes(self) -> Result<&'a [u8], CStrError> {
        let len = self.len()?;
        // SAFETY: `self.ptr` is valid for `len` bytes, as ensured by `len()`
        // returning successfully above.
        unsafe { Ok(slice::from_raw_parts(self.ptr.as_ptr(), len)) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr::NonNull;
    use std::thread;

    #[test]
    fn send_sync_static() {
        static BYTES: &[u8] = b"static\0";
        let cstr = CStr::from_bytes_with_nul(BYTES);
        thread::spawn(move || {
            assert_eq!(cstr.to_bytes().unwrap(), b"static");
        })
        .join()
        .unwrap();
    }

    #[test]
    fn len_ok() {
        static BYTES: &[u8] = b"abc\0";
        let cstr = CStr::from_bytes_with_nul(BYTES);
        assert_eq!(cstr.len().unwrap(), 3);
    }

    #[test]
    fn len_too_long() {
        let mut bytes = vec![b'a'; MAX_NAME_LENGTH + 1];
        bytes.push(0);
        let ptr = NonNull::new(bytes.as_mut_ptr() as *mut i8).unwrap();
        // SAFETY: `ptr` comes from `bytes` which is NUL-terminated and lives for
        // the duration of the test.
        let cstr = unsafe { CStr::from_ptr(ptr) };
        assert!(cstr.len().is_err());
    }
}

impl Display for CStr<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let ptr = self.ptr.as_ptr();
        let len = self.len().map_err(|_| fmt::Error)?;
        // SAFETY: `ptr` is valid for `len` bytes from the preceding `len()` call.
        let bytes = unsafe { slice::from_raw_parts(ptr, len) };
        display_lossy(bytes, formatter)
    }
}

impl Debug for CStr<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let ptr = self.ptr.as_ptr();
        let len = self.len().map_err(|_| fmt::Error)?;
        // SAFETY: `ptr` is valid for `len` bytes as above.
        let bytes = unsafe { slice::from_raw_parts(ptr, len) };
        debug_lossy(bytes, formatter)
    }
}

fn display_lossy(mut bytes: &[u8], formatter: &mut fmt::Formatter) -> fmt::Result {
    loop {
        match str::from_utf8(bytes) {
            Ok(valid) => return formatter.write_str(valid),
            Err(utf8_error) => {
                let valid_up_to = utf8_error.valid_up_to();

                // The substring `[..valid_up_to]` is guaranteed valid UTF-8.
                match str::from_utf8(&bytes[..valid_up_to]) {
                    Ok(valid) => formatter.write_str(valid)?,
                    Err(_) => return Err(fmt::Error),
                }
                formatter.write_char(char::REPLACEMENT_CHARACTER)?;

                if let Some(error_len) = utf8_error.error_len() {
                    bytes = &bytes[valid_up_to + error_len..];
                } else {
                    // No further data can be parsed (incomplete sequence at end)
                    return Ok(());
                }
            }
        }
    }
}

pub(crate) fn debug_lossy(mut bytes: &[u8], formatter: &mut fmt::Formatter) -> fmt::Result {
    const EMPTY: &str = "";

    formatter.write_char('"')?;

    while !bytes.is_empty() {
        let from_utf8_result = str::from_utf8(bytes);
        let valid = match from_utf8_result {
            Ok(valid) => valid,
            Err(utf8_error) => {
                let valid_up_to = utf8_error.valid_up_to();
                // Guaranteed valid UTF-8; safe to unwrap
                str::from_utf8(&bytes[..valid_up_to]).unwrap_or(EMPTY)
            }
        };

        let mut written = 0;
        for (i, ch) in valid.char_indices() {
            let esc = ch.escape_debug();
            if esc.len() != 1 && ch != '\'' {
                formatter.write_str(&valid[written..i])?;
                for ch in esc {
                    formatter.write_char(ch)?;
                }
                written = i + ch.len_utf8();
            }
        }
        formatter.write_str(&valid[written..])?;

        match from_utf8_result {
            Ok(_valid) => break,
            Err(utf8_error) => {
                let end_of_broken = if let Some(error_len) = utf8_error.error_len() {
                    valid.len() + error_len
                } else {
                    bytes.len()
                };
                for b in &bytes[valid.len()..end_of_broken] {
                    write!(formatter, "\\x{:02x}", b)?;
                }
                bytes = &bytes[end_of_broken..];
            }
        }
    }

    formatter.write_char('"')
}
