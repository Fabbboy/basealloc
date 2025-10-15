use core::ops::Range;

use crate::{
  GLOBAL_SYSTEM,
  system::{
    SysError,
    SysOption,
  },
};

#[derive(Debug)]
pub enum ExtentError {
  SystemError(SysError),
  OutOfBounds,
}

pub type ExtentResult<T> = Result<T, ExtentError>;

pub struct Extent {
  slice: &'static mut [u8],
}

impl Extent {
  pub fn new(size: usize, options: SysOption) -> ExtentResult<Extent> {
    let slice = unsafe { GLOBAL_SYSTEM.alloc(size, options) }.map_err(ExtentError::SystemError)?;

    Ok(Extent { slice })
  }

  fn check(&self, range: Range<usize>) -> ExtentResult<()> {
    if range.start > range.end || range.end > self.slice.len() {
      return Err(ExtentError::OutOfBounds);
    }
    Ok(())
  }

  pub fn modify(&mut self, range: Range<usize>, options: SysOption) -> ExtentResult<()> {
    self.check(range.clone())?;
    unsafe { GLOBAL_SYSTEM.modify(&self.slice[range], options) }.map_err(ExtentError::SystemError)
  }
}

impl AsRef<[u8]> for Extent {
  fn as_ref(&self) -> &[u8] {
    self.slice
  }
}

impl AsMut<[u8]> for Extent {
  fn as_mut(&mut self) -> &mut [u8] {
    self.slice
  }
}

impl Drop for Extent {
  fn drop(&mut self) {
    let _ = unsafe { GLOBAL_SYSTEM.dealloc(self.slice) };
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    prim::page_size,
    system::SysOption,
  };

  #[test]
  fn test_extent_new() {
    let ps = page_size();
    let extent = Extent::new(ps, SysOption::Commit);
    assert!(extent.is_ok());
    let extent = extent.unwrap();
    assert_eq!(extent.as_ref().len(), ps);
  }

  #[test]
  fn test_extent_zero_size() {
    let extent = Extent::new(0, SysOption::Commit);
    assert!(extent.is_ok());
    let extent = extent.unwrap();
    assert_eq!(extent.as_ref().len(), 0);
  }

  #[test]
  fn test_extent_as_ref() {
    let ps = page_size();
    let extent = Extent::new(ps, SysOption::Commit).unwrap();
    let slice = extent.as_ref();
    assert_eq!(slice.len(), ps);
  }

  #[test]
  fn test_extent_as_mut() {
    let ps = page_size();
    let mut extent = Extent::new(ps, SysOption::Commit).unwrap();
    let slice = extent.as_mut();
    assert_eq!(slice.len(), ps);
    slice[0] = 42;
    assert_eq!(slice[0], 42);
  }

  #[test]
  fn test_extent_partial_valid_range() {
    let ps = page_size();
    let mut extent = Extent::new(ps * 2, SysOption::Commit).unwrap();
    let result = extent.modify(0..ps, SysOption::Reserve);
    assert!(result.is_ok());
  }

  #[test]
  fn test_extent_partial_full_range() {
    let ps = page_size();
    let mut extent = Extent::new(ps, SysOption::Commit).unwrap();
    let result = extent.modify(0..ps, SysOption::Reserve);
    assert!(result.is_ok());
  }

  #[test]
  fn test_extent_partial_empty_range() {
    let ps = page_size();
    let mut extent = Extent::new(ps, SysOption::Commit).unwrap();
    let result = extent.modify(100..100, SysOption::Reserve);
    assert!(result.is_ok());
  }

  #[test]
  fn test_extent_partial_oob_end() {
    let ps = page_size();
    let mut extent = Extent::new(ps, SysOption::Commit).unwrap();
    let result = extent.modify(0..ps + 1, SysOption::Reserve);
    assert!(matches!(result, Err(ExtentError::OutOfBounds)));
  }

  #[test]
  fn test_extent_partial_oob_start() {
    let ps = page_size();
    let mut extent = Extent::new(ps, SysOption::Commit).unwrap();
    let result = extent.modify(ps + 1..ps + 2, SysOption::Reserve);
    assert!(matches!(result, Err(ExtentError::OutOfBounds)));
  }

  #[test]
  fn test_extent_partial_invalid_range() {
    let ps = page_size();
    let mut extent = Extent::new(ps, SysOption::Commit).unwrap();
    let result = extent.modify(100..50, SysOption::Reserve);
    assert!(matches!(result, Err(ExtentError::OutOfBounds)));
  }

  #[test]
  fn test_extent_check_valid() {
    let ps = page_size();
    let extent = Extent::new(ps, SysOption::Commit).unwrap();
    assert!(extent.check(0..ps / 2).is_ok());
    assert!(extent.check(0..ps).is_ok());
    assert!(extent.check(100..100).is_ok());
  }

  #[test]
  fn test_extent_check_invalid() {
    let ps = page_size();
    let extent = Extent::new(ps, SysOption::Commit).unwrap();
    assert!(matches!(
      extent.check(0..ps + 1),
      Err(ExtentError::OutOfBounds)
    ));
    assert!(matches!(
      extent.check(100..50),
      Err(ExtentError::OutOfBounds)
    ));
    assert!(matches!(
      extent.check(ps + 1..ps + 2),
      Err(ExtentError::OutOfBounds)
    ));
  }

  #[test]
  fn test_extent_drop() {
    let ps = page_size();
    let extent = Extent::new(ps, SysOption::Commit).unwrap();
    drop(extent);
  }
}
