#[cfg(any(target_os = "linux", target_os = "macos"))]
use crate::{
  prim::is_page_aligned,
  system::{
    SysError,
    SysOption,
    SysResult,
    System,
  },
};

pub struct UnixSystem {}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub static UNIX_SYSTEM: UnixSystem = UnixSystem {};

#[cfg(any(target_os = "linux", target_os = "macos"))]
impl UnixSystem {
  const fn prot_as(options: SysOption) -> i32 {
    match options {
      SysOption::ReadWrite => libc::PROT_READ | libc::PROT_WRITE,
      _ => libc::PROT_NONE,
    }
  }

  const fn flags() -> i32 {
    libc::MAP_PRIVATE | libc::MAP_ANONYMOUS
  }

  const fn reserve_prot() -> i32 {
    libc::PROT_NONE
  }

  const fn reclaim_flags() -> i32 {
    libc::MADV_DONTNEED
  }

  const fn as_c(slice: &[u8]) -> *mut libc::c_void {
    slice.as_ptr() as *mut libc::c_void
  }

  fn protect(slice: &[u8], options: SysOption) -> Result<(), SysError> {
    let prot = match options {
      SysOption::Reserve => Self::reserve_prot(),
      SysOption::ReadWrite => Self::prot_as(options),
      SysOption::Reclaim => return Err(SysError::InvalidArgument),
    };
    let result = unsafe { libc::mprotect(Self::as_c(slice), slice.len(), prot) };
    if result == 0 {
      return Ok(());
    }

    Err(SysError::InvalidArgument)
  }

  fn advise(slice: &[u8], options: SysOption) -> SysResult<()> {
    let flags = match options {
      SysOption::Reclaim => Self::reclaim_flags(),
      _ => return Err(SysError::InvalidArgument), // only reclaim uses madvise
    };

    let result = unsafe { libc::madvise(Self::as_c(slice), slice.len(), flags) };
    if result == 0 {
      Ok(())
    } else {
      Err(SysError::InvalidArgument)
    }
  }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
unsafe impl System for UnixSystem {
  unsafe fn alloc<'mem>(&self, size: usize, options: SysOption) -> SysResult<&'mem mut [u8]> {
    if is_page_aligned(size) != Some(true) {
      return Err(SysError::InvalidArgument);
    }

    let prot = match options {
      SysOption::Reserve => Self::reserve_prot(),
      SysOption::ReadWrite => Self::prot_as(options),
      SysOption::Reclaim => return Err(SysError::InvalidArgument),
    };

    let ptr = unsafe { libc::mmap(core::ptr::null_mut(), size, prot, Self::flags(), -1, 0) };

    match ptr {
      libc::MAP_FAILED => Err(SysError::OutOfMemory),
      _ => {
        let slice = unsafe { core::slice::from_raw_parts_mut(ptr as *mut u8, size) };
        Ok(slice)
      }
    }
  }

  unsafe fn modify(&self, slice: &[u8], options: SysOption) -> SysResult<()> {
    match options {
      SysOption::Reserve | SysOption::ReadWrite => Self::protect(slice, options),
      SysOption::Reclaim => Self::advise(slice, options),
    }
  }

  unsafe fn dealloc(&self, slice: &[u8]) -> SysResult<()> {
    let result = unsafe { libc::munmap(Self::as_c(slice), slice.len()) };
    if result == 0 {
      return Ok(());
    }

    Err(SysError::InvalidArgument)
  }
}
