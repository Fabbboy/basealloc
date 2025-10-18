#[cfg(any(target_os = "linux", target_os = "macos"))]
use crate::unix::UNIX_SYSTEM;

#[derive(Debug)]
pub enum SysError {
  Unsupported,
  OutOfMemory,
  InvalidArgument,
}

#[derive(Debug, Clone, Copy)]
pub enum SysOption {
  Commit,
  Reserve,
  Reclaim,
}

pub type SysResult<T> = Result<T, SysError>;

/// Low-level system memory management trait.
///
/// # Safety
///
/// Implementors must ensure that:
/// - `alloc` returns valid, page-aligned memory that can be safely accessed
/// - `modify` only operates on memory previously allocated by this system
/// - `dealloc` only operates on memory previously allocated by this system
/// - Memory is not accessed after `dealloc` is called
/// - All operations respect the system's memory protection model
pub unsafe trait System
where
  Self: Send + Sync,
{
  /// Allocates memory from the system.
  ///
  /// # Safety
  ///
  /// Caller must ensure `size` is page-aligned and the returned memory
  /// is only used according to the specified `options`.
  unsafe fn alloc<'mem>(&self, size: usize, options: SysOption) -> SysResult<&'mem mut [u8]> {
    _ = (size, options);
    Err(SysError::Unsupported)
  }

  /// Modifies memory protection or advisory settings.
  ///
  /// # Safety
  ///
  /// Caller must ensure `slice` was previously allocated by this system
  /// and is still valid (not deallocated).
  unsafe fn modify(&self, slice: &[u8], options: SysOption) -> SysResult<()> {
    _ = (slice, options);
    Err(SysError::Unsupported)
  }

  /// Deallocates memory previously allocated by this system.
  ///
  /// # Safety
  ///
  /// Caller must ensure `slice` was previously allocated by this system,
  /// is still valid, and will not be accessed after this call.
  unsafe fn dealloc(&self, slice: &[u8]) -> SysResult<()> {
    _ = slice;
    Err(SysError::Unsupported)
  }
}

pub struct UnsupportedSystem {}
unsafe impl System for UnsupportedSystem {}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub static GLOBAL_SYSTEM: &dyn System = &UNIX_SYSTEM;

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub static GLOBAL_SYSTEM: &dyn System = &UnsupportedSystem {};

#[cfg(test)]
mod tests;
