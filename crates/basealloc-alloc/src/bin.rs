use core::{
  alloc::Layout,
  ptr::NonNull,
};

use basealloc_fixed::bump::{
  Bump,
  BumpError,
};
use spin::Mutex;

use crate::classes::{
  SizeClass,
  SizeClassIndex,
  SlabSize,
  class_at,
  pages_for,
};

#[derive(Debug)]
pub enum BinError {
  BumpError(BumpError),
}

pub type BinResult<T> = Result<T, BinError>;

pub struct Bin {
  // SAFETY: User must ensure bin is dropped before bump.
  class: SizeClass,
  pages: SlabSize,
  lock: Mutex<()>,
}

impl Bin {
  pub fn new(idx: SizeClassIndex) -> Self {
    Self {
      class: class_at(idx),
      pages: pages_for(idx),
      lock: Mutex::new(()),
    }
  }

  pub fn allocate(&mut self, bump: &mut Bump, layout: Layout) -> BinResult<NonNull<u8>> {
    _ = layout;
    todo!()
  }

  pub fn deallocate(&mut self, ptr: NonNull<u8>, layout: Layout) -> BinResult<()> {
    _ = ptr;
    _ = layout;
    todo!()
  }
}
