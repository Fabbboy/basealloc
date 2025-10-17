use core::ptr::NonNull;

use basealloc_fixed::bump::{
  Bump,
  BumpError,
};
use basealloc_list::{
  HasLink,
  List,
};
use spin::Mutex;

use crate::{
  arena::Arena,
  classes::{
    SizeClass,
    SizeClassIndex,
    SlabSize,
    class_at,
    pages_for,
  },
  slab::{
    Slab,
    SlabError,
  },
};

const MIN_SLABS: usize = 1;

#[derive(Debug)]
pub enum BinError {
  BumpError(BumpError),
  SlabError(SlabError),
}

pub type BinResult<T> = Result<T, BinError>;

pub struct Bin {
  // SAFETY: User must ensure bin is dropped before bump.
  class: SizeClass,
  pages: SlabSize,
  lock: Mutex<()>,
  free: Option<NonNull<Slab>>,
  head: Option<NonNull<Slab>>,
  tail: Option<NonNull<Slab>>,
}

impl Bin {
  pub fn new(idx: SizeClassIndex) -> Self {
    Self {
      class: class_at(idx),
      pages: pages_for(idx),
      lock: Mutex::new(()),
      free: None,
      head: None,
      tail: None,
    }
  }

  pub fn allocate(&mut self, bump: &mut Bump, arena: NonNull<Arena>) -> BinResult<NonNull<u8>> {
    todo!()
  }

  pub fn deallocate(&mut self, ptr: NonNull<u8>, mut slab: NonNull<Slab>) -> BinResult<()> {
    todo!()
  }
}

impl Drop for Bin {
  fn drop(&mut self) {
    if let Some(mut head) = self.free {
      unsafe {
        let _ = List::drain(head.as_mut());
      }
    }

    if let Some(mut head) = self.head {
      unsafe {
        let _ = List::drain(head.as_mut());
      }
    }
  }
}
