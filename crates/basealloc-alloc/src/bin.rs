use core::{
  alloc::Layout,
  ptr::NonNull,
};

use basealloc_fixed::bump::{
  Bump,
  BumpError,
};
use basealloc_list::List;
use basealloc_rtree::RTree;
use spin::Mutex;

use crate::{
  FANOUT,
  classes::{
    SizeClass,
    SizeClassIndex,
    SlabSize,
    class_at,
    pages_for,
  },
  slab::Slab,
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
  free_slabs: Option<NonNull<Slab>>,
  head: Option<NonNull<Slab>>,
  tail: Option<NonNull<Slab>>,
}

impl Bin {
  pub fn new(idx: SizeClassIndex) -> Self {
    Self {
      class: class_at(idx),
      pages: pages_for(idx),
      lock: Mutex::new(()),
      free_slabs: None,
      head: None,
      tail: None,
    }
  }

  pub fn allocate(&mut self, bump: &mut Bump) -> BinResult<NonNull<u8>> {
    todo!()
  }

  pub fn deallocate(&mut self, ptr: NonNull<u8>) -> BinResult<()> {
    _ = ptr;
    todo!()
  }
}

impl Drop for Bin {
  fn drop(&mut self) {
    if let Some(mut head) = self.free_slabs {
      unsafe {
        let _ = List::drain(head.as_mut());
      }
    }
  }
}
