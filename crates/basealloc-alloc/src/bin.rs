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
    let _g = self.lock.lock();

    if let Some(mut head_ptr) = self.head {
      let head = unsafe { head_ptr.as_mut() };
      match head.allocate() {
        Ok(p) => return Ok(p),
        Err(SlabError::OutOfMemory) => {}
        Err(e) => return Err(BinError::SlabError(e)),
      }
    }

    if let Some(mut free_ptr) = self.free.take() {
      let free_slab = unsafe { free_ptr.as_mut() };
      if let Some(mut head_ptr) = self.head {
        let head = unsafe { head_ptr.as_mut() };
        List::insert_before(free_slab, head);
        self.head = Some(free_ptr);
      } else {
        self.head = Some(free_ptr);
        self.tail = Some(free_ptr);
      }

      match free_slab.allocate() {
        Ok(p) => return Ok(p),
        Err(SlabError::OutOfMemory) => {}
        Err(e) => return Err(BinError::SlabError(e)),
      }
    }

    let SlabSize(size) = self.pages;
    let mut slab_nn = Slab::new(bump, self.class, size, arena).map_err(|e| match e {
      SlabError::BumpError(be) => BinError::BumpError(be),
      other => BinError::SlabError(other),
    })?;

    let slab_ref = unsafe { slab_nn.as_mut() };
    if let Some(mut head_ptr) = self.head {
      let head = unsafe { head_ptr.as_mut() };
      List::insert_before(slab_ref, head);
      self.head = Some(slab_nn);
    } else {
      self.head = Some(slab_nn);
      self.tail = Some(slab_nn);
    }

    slab_ref.allocate().map_err(|e| match e {
      SlabError::BumpError(be) => BinError::BumpError(be),
      other => BinError::SlabError(other),
    })
  }

  pub fn deallocate(&mut self, ptr: NonNull<u8>, mut slab: NonNull<Slab>) -> BinResult<()> {
    let _g = self.lock.lock();

    let slab_ref = unsafe { slab.as_mut() };
    slab_ref.deallocate(ptr).map_err(|e| match e {
      SlabError::BumpError(be) => BinError::BumpError(be),
      other => BinError::SlabError(other),
    })
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
