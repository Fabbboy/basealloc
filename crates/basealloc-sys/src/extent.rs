use core::{
  cmp,
  ops::Range,
  sync::atomic::{
    AtomicBool,
    Ordering,
  },
};

use basealloc_list::{
  HasLink,
  Link,
};
use basealloc_rbtree::{
  HasNode,
  RBNode,
};

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
  NonOwning,
}

pub type ExtentResult<T> = Result<T, ExtentError>;

pub struct Extent {
  slice: &'static mut [u8],
  owning: AtomicBool,
  node: RBNode<Extent>,
  link: Link<Extent>,
}

impl Extent {
  pub fn new(size: usize, options: SysOption) -> ExtentResult<Extent> {
    let slice = unsafe { GLOBAL_SYSTEM.alloc(size, options) }.map_err(ExtentError::SystemError)?;

    Ok(Extent {
      slice,
      owning: AtomicBool::new(true),
      node: RBNode::default(),
      link: Link::default(),
    })
  }

  pub fn is_owning(&self) -> bool {
    self.owning.load(Ordering::Acquire)
  }

  // Other considerations:
  // - giveup
  // - handover
  // - die_and_inherit
  // - capitulate
  // - relinquish
  // - abdicate
  // - takeover
  // - invasion
  pub fn giveup(mut self, other: &mut Extent) -> ExtentResult<()> {
    if !self.is_owning() {
      return Err(ExtentError::NonOwning);
    }

    core::mem::swap(&mut self.slice, &mut other.slice);
    self.owning.store(false, Ordering::Release);
    Ok(())
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

impl HasLink for Extent {
  fn link(&self) -> &Link<Self> {
    &self.link
  }

  fn link_mut(&mut self) -> &mut Link<Self> {
    &mut self.link
  }
}

impl HasNode for Extent {
  fn node(&self) -> &RBNode<Self> {
    &self.node
  }

  fn node_mut(&mut self) -> &mut RBNode<Self> {
    &mut self.node
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
}
