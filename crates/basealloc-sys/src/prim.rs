use crate::math::{
  align_up,
  is_aligned,
};
use core::sync::atomic::{
  AtomicBool,
  AtomicUsize,
  Ordering,
};

#[derive(Debug, PartialEq)]
pub enum PrimError {
  InvalidAlignment,
  Overflow,
}

pub type PrimResult<T> = Result<T, PrimError>;

#[cfg(any(
    not(any(target_os = "linux", target_os = "macos")),
    target_os = "windows" // temporary
))]
const COMMON_PAGE_SIZE: usize = 4096;

pub const fn word_width() -> usize {
  core::mem::size_of::<usize>()
}

pub const fn min_align() -> usize {
  core::mem::align_of::<u128>()
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn page_size_helper() -> usize {
  unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize }
}

#[cfg(target_os = "windows")]
fn page_size_helper() -> usize {
  COMMON_PAGE_SIZE // no support for Windows yet
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn page_size_helper() -> usize {
  COMMON_PAGE_SIZE
}

pub fn page_size() -> usize {
  static PAGE_SIZE: AtomicUsize = AtomicUsize::new(0);
  static INIT: AtomicBool = AtomicBool::new(false);

  if !INIT.load(Ordering::Acquire) {
    let size = page_size_helper();
    PAGE_SIZE.store(size, Ordering::Release);
    INIT.store(true, Ordering::Release);
    size
  } else {
    PAGE_SIZE.load(Ordering::Acquire)
  }
}

pub fn page_align(value: usize) -> PrimResult<usize> {
  align_up(value, page_size()).ok_or(PrimError::Overflow)
}

pub fn is_page_aligned(value: usize) -> PrimResult<bool> {
  is_aligned(value, page_size()).ok_or(PrimError::InvalidAlignment)
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn test_word_width() {
    assert_eq!(word_width(), core::mem::size_of::<usize>());
  }

  #[test]
  fn test_min_align() {
    assert_eq!(min_align(), core::mem::align_of::<u128>());
  }

  #[test]
  fn test_page_size() {
    let size = page_size();
    assert!(size > 0);
    assert!(size.is_power_of_two());
    assert_eq!(page_size(), size);
  }

  #[test]
  fn test_page_align() {
    let ps = page_size();
    assert_eq!(page_align(0), Ok(0));
    assert_eq!(page_align(1), Ok(ps));
    assert_eq!(page_align(ps), Ok(ps));
    assert_eq!(page_align(ps + 1), Ok(ps * 2));
    assert_eq!(page_align(ps - 1), Ok(ps));

    assert!(matches!(page_align(usize::MAX), Err(PrimError::Overflow)));
    assert!(matches!(
      page_align(usize::MAX - ps + 2),
      Err(PrimError::Overflow)
    ));
  }

  #[test]
  fn test_is_page_aligned() {
    let ps = page_size();
    assert_eq!(is_page_aligned(0), Ok(true));
    assert_eq!(is_page_aligned(1), Ok(false));
    assert_eq!(is_page_aligned(ps), Ok(true));
    assert_eq!(is_page_aligned(ps + 1), Ok(false));
    assert_eq!(is_page_aligned(ps - 1), Ok(false));
    assert_eq!(is_page_aligned(ps * 2), Ok(true));
  }
}
