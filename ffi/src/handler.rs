#![allow(dead_code)]
use libc::FILE;

#[cfg(target_os = "linux")]
unsafe extern "C" {
  static mut stderr: *mut FILE;
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
  static mut __stderrp: *mut FILE;
}

#[cfg(target_os = "windows")]
unsafe extern "C" {
  static mut _stderr: *mut FILE;
}

#[cfg(not(test))]
#[panic_handler]
pub fn panic_handler(info: &core::panic::PanicInfo) -> ! {
  unsafe {
    let message = info.message();
    if let Some(message_str) = message.as_str() {
      libc::fprintf(
        stderr,
        b"panic: %s\n\0".as_ptr() as *const i8,
        message_str.as_ptr() as *const i8,
      );
    } else {
      libc::fprintf(stderr, b"panic: (no message)\n\0".as_ptr() as *const i8);
    }

    if let Some(loc) = info.location() {
      libc::fprintf(
        stderr,
        b"at %s:%d:%d\n\0".as_ptr() as *const i8,
        loc.file().as_ptr(),
        loc.line() as i32,
        loc.column() as i32,
      );
    }

    libc::abort();
  }
}
