#![cfg_attr(not(test), no_std)]

use core::{
  cmp,
  mem::{
    ManuallyDrop,
    MaybeUninit,
  },
  ops::Range,
};

use basealloc_list::{
  HasLink,
  Link,
};
use basealloc_sys::{
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
  link: ManuallyDrop<Link<Extent>>,
  slice: &'static mut [u8],
}

impl Extent {
  pub fn new(size: usize, options: SysOption) -> ExtentResult<Extent> {
    let slice = unsafe { GLOBAL_SYSTEM.alloc(size, options) }.map_err(ExtentError::SystemError)?;

    Ok(Extent {
      slice,
      link: ManuallyDrop::new(Link::default()),
    })
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

impl HasLink for Extent {
  fn link(&self) -> &Link<Self> {
    &*self.link
  }

  fn link_mut(&mut self) -> &mut Link<Self> {
    &mut *self.link
  }
}

#[cfg(test)]
mod tests;
