use core::{
  alloc::Layout,
  ptr::NonNull,
};

use basealloc_fixed::bump::Bump;

use crate::classes::SizeClass;

#[derive(Debug)]
pub enum BinError {}

pub type BinResult<T> = Result<T, BinError>;

pub struct Bin {
  // SAFETY: User must ensure bin is dropped before bump.
  bump: NonNull<Bump>,
  class: SizeClass,
}

impl Bin {
  pub fn new(bump: &mut Bump, class: SizeClass) -> Self {
    Self {
      bump: NonNull::from(bump),
      class,
    }
  }

  pub fn allocate(&mut self, layout: Layout) -> BinResult<NonNull<u8>> {
    _ = layout;
    todo!()
  }
}
