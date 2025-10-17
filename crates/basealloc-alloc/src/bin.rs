use core::{
  alloc::Layout,
  ptr::NonNull,
};

use basealloc_fixed::bump::Bump;

use crate::classes::{class_at, pages_for, BinSize, SizeClass, SizeClassIndex};

#[derive(Debug)]
pub enum BinError {}

pub type BinResult<T> = Result<T, BinError>;

pub struct Bin {
  // SAFETY: User must ensure bin is dropped before bump.
  bump: NonNull<Bump>,
  class: SizeClass,
  pages: BinSize,
}

impl Bin {
  pub fn new(bump: &mut Bump, idx: SizeClassIndex) -> Self {
    Self {
      bump: NonNull::from(bump),
      class: class_at(idx),
      pages: pages_for(idx),
    }
  }

  pub fn allocate(&mut self, layout: Layout) -> BinResult<NonNull<u8>> {
    _ = layout;
    todo!()
  }
}
