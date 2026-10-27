//! macOS's atomic directory exchange: the installed path exists on both sides of the syscall.
//! This command is private to the installation helper; it is never registered on renderer RPC.
use std::path::Path;

#[cfg(target_os = "macos")]
pub fn exchange(left: &Path, right: &Path) -> std::io::Result<()> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    for path in [left, right] {
        let metadata = std::fs::symlink_metadata(path)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() || !path.is_absolute() {
            return Err(std::io::ErrorKind::InvalidInput.into());
        }
    }
    if std::fs::canonicalize(left)? == std::fs::canonicalize(right)? {
        return Err(std::io::ErrorKind::InvalidInput.into());
    }
    let left = CString::new(left.as_os_str().as_bytes())?;
    let right = CString::new(right.as_os_str().as_bytes())?;
    unsafe extern "C" {
        fn renamex_np(
            left: *const std::ffi::c_char,
            right: *const std::ffi::c_char,
            flags: u32,
        ) -> i32;
    }
    // RENAME_SWAP from the macOS SDK sys/stdio.h. Fails without mutation across volumes.
    if unsafe { renamex_np(left.as_ptr(), right.as_ptr(), 0x00000002) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn exchange(_left: &Path, _right: &Path) -> std::io::Result<()> {
    Err(std::io::ErrorKind::Unsupported.into())
}
