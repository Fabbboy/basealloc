use core::{
  cmp,
  mem::MaybeUninit,
  ops::Range,
};

use crate::{
  GLOBAL_SYSTEM,
  Giveup,
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

  pub fn check(&self, range: Range<usize>) -> ExtentResult<()> {
    if range.start > range.end || range.end > self.slice.len() {
      return Err(ExtentError::OutOfBounds);
    }
    Ok(())
  }

  #[inline(always)]
  pub fn ord(one: &Extent, other: &Extent) -> cmp::Ordering {
    let one_len = one.slice.len();
    let other_len = other.slice.len();

    one_len.cmp(&other_len)
  }
}

impl Giveup for Extent {
  type Failure = ();

  fn giveup(mut self) -> Result<Self, Self::Failure>
  where
    Self: Sized,
  {
    let mut target = MaybeUninit::uninit();
    core::mem::swap(&mut self, unsafe { &mut *target.as_mut_ptr() });
    self.slice = &mut [];
    Ok(unsafe { target.assume_init() })
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

  #[test]
  fn test_extent_giveup() {
    let ps = page_size();
    let extent = Extent::new(ps, SysOption::Commit).unwrap();
    let len = extent.as_ref().len();
    let extent2 = extent.giveup();
    assert!(extent2.is_ok());
    assert_eq!(extent2.unwrap().as_ref().len(), len);
  }
}
