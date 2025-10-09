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
  OOB,
}

pub type ExtentResult<T> = Result<T, ExtentError>;

pub struct Extent<'mem> {
  slice: &'mem mut [u8],
}

impl<'mem> Extent<'mem> {
  pub fn new(size: usize, options: SysOption) -> ExtentResult<Extent<'mem>> {
    let slice = unsafe { GLOBAL_SYSTEM.alloc(size, options) }.map_err(ExtentError::SystemError)?;

    Ok(Extent { slice })
  }

  fn check(&self, range: Range<usize>) -> ExtentResult<()> {
    if range.start > range.end || range.end > self.slice.len() {
      return Err(ExtentError::OOB);
    }
    Ok(())
  }

  pub fn modify(&mut self, options: SysOption) -> ExtentResult<()> {
    unsafe { GLOBAL_SYSTEM.modify(self.slice, options) }.map_err(ExtentError::SystemError)
  }

  pub fn partial(&mut self, range: Range<usize>, options: SysOption) -> ExtentResult<()> {
    self.check(range.clone())?;
    unsafe { GLOBAL_SYSTEM.modify(&mut self.slice[range], options) }
      .map_err(ExtentError::SystemError)
  }
}

impl<'mem> AsRef<[u8]> for Extent<'mem> {
  fn as_ref(&self) -> &[u8] {
    self.slice
  }
}

impl<'mem> AsMut<[u8]> for Extent<'mem> {
  fn as_mut(&mut self) -> &mut [u8] {
    self.slice
  }
}

impl<'mem> Drop for Extent<'mem> {
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
    let extent = Extent::new(ps, SysOption::ReadWrite);
    assert!(extent.is_ok());
    let extent = extent.unwrap();
    assert_eq!(extent.as_ref().len(), ps);
  }

  #[test]
  fn test_extent_zero_size() {
    let extent = Extent::new(0, SysOption::ReadWrite);
    assert!(extent.is_ok());
    let extent = extent.unwrap();
    assert_eq!(extent.as_ref().len(), 0);
  }

  #[test]
  fn test_extent_as_ref() {
    let ps = page_size();
    let extent = Extent::new(ps, SysOption::ReadWrite).unwrap();
    let slice = extent.as_ref();
    assert_eq!(slice.len(), ps);
  }

  #[test]
  fn test_extent_as_mut() {
    let ps = page_size();
    let mut extent = Extent::new(ps, SysOption::ReadWrite).unwrap();
    let slice = extent.as_mut();
    assert_eq!(slice.len(), ps);
    slice[0] = 42;
    assert_eq!(slice[0], 42);
  }

  #[test]
  fn test_extent_modify() {
    let ps = page_size();
    let mut extent = Extent::new(ps, SysOption::ReadWrite).unwrap();
    let result = extent.modify(SysOption::Reserve);
    assert!(result.is_ok());
  }

  #[test]
  fn test_extent_partial_valid_range() {
    let ps = page_size();
    let mut extent = Extent::new(ps * 2, SysOption::ReadWrite).unwrap();
    let result = extent.partial(0..ps, SysOption::Reserve);
    assert!(result.is_ok());
  }

  #[test]
  fn test_extent_partial_full_range() {
    let ps = page_size();
    let mut extent = Extent::new(ps, SysOption::ReadWrite).unwrap();
    let result = extent.partial(0..ps, SysOption::Reserve);
    assert!(result.is_ok());
  }

  #[test]
  fn test_extent_partial_empty_range() {
    let ps = page_size();
    let mut extent = Extent::new(ps, SysOption::ReadWrite).unwrap();
    let result = extent.partial(100..100, SysOption::Reserve);
    assert!(result.is_ok());
  }

  #[test]
  fn test_extent_partial_oob_end() {
    let ps = page_size();
    let mut extent = Extent::new(ps, SysOption::ReadWrite).unwrap();
    let result = extent.partial(0..ps + 1, SysOption::Reserve);
    assert!(matches!(result, Err(ExtentError::OOB)));
  }

  #[test]
  fn test_extent_partial_oob_start() {
    let ps = page_size();
    let mut extent = Extent::new(ps, SysOption::ReadWrite).unwrap();
    let result = extent.partial(ps + 1..ps + 2, SysOption::Reserve);
    assert!(matches!(result, Err(ExtentError::OOB)));
  }

  #[test]
  fn test_extent_partial_invalid_range() {
    let ps = page_size();
    let mut extent = Extent::new(ps, SysOption::ReadWrite).unwrap();
    let result = extent.partial(100..50, SysOption::Reserve);
    assert!(matches!(result, Err(ExtentError::OOB)));
  }

  #[test]
  fn test_extent_check_valid() {
    let ps = page_size();
    let extent = Extent::new(ps, SysOption::ReadWrite).unwrap();
    assert!(extent.check(0..ps/2).is_ok());
    assert!(extent.check(0..ps).is_ok());
    assert!(extent.check(100..100).is_ok());
  }

  #[test]
  fn test_extent_check_invalid() {
    let ps = page_size();
    let extent = Extent::new(ps, SysOption::ReadWrite).unwrap();
    assert!(matches!(extent.check(0..ps + 1), Err(ExtentError::OOB)));
    assert!(matches!(extent.check(100..50), Err(ExtentError::OOB)));
    assert!(matches!(extent.check(ps + 1..ps + 2), Err(ExtentError::OOB)));
  }

  #[test]
  fn test_extent_drop() {
    let ps = page_size();
    let extent = Extent::new(ps, SysOption::ReadWrite).unwrap();
    drop(extent);
  }
}
