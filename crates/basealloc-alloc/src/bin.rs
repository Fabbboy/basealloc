use core::{
  alloc::Layout,
  ptr::NonNull,
};

use basealloc_fixed::bump::{
  Bump,
  BumpError,
};
use basealloc_list::List;
use getset::{
  Getters,
  MutGetters,
};
use spin::Mutex;

use crate::{
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

#[derive(Getters, MutGetters)]
struct Used {
  #[getset(get = "pub", get_mut = "pub")]
  head: Option<NonNull<Slab>>,
  #[getset(get = "pub", get_mut = "pub")]
  tail: Option<NonNull<Slab>>,
}

impl Used {
  pub fn new() -> Self {
    Self {
      head: None,
      tail: None,
    }
  }
}

impl Drop for Used {
  fn drop(&mut self) {
    if let Some(mut head) = self.head {
      unsafe {
        let _ = List::drain(head.as_mut());
      }
    }
  }
}

pub struct Bin {
  // SAFETY: User must ensure bin is dropped before bump.
  class: SizeClass,
  pages: SlabSize,
  lock: Mutex<()>,
  free_slabs: Option<NonNull<Slab>>,
  used: Used,
}

impl Bin {
  pub fn new(idx: SizeClassIndex) -> Self {
    Self {
      class: class_at(idx),
      pages: pages_for(idx),
      lock: Mutex::new(()),
      free_slabs: None,
      used: Used::new(),
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

impl Drop for Bin {
  fn drop(&mut self) {
    if let Some(mut head) = self.free_slabs {
      unsafe {
        let _ = List::drain(head.as_mut());
      }
    }
  }
}
