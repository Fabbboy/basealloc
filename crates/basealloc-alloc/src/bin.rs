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


#[derive(Debug)]
pub enum BinError {
  BumpError(BumpError),
  SlabError(SlabError),
}

impl From<BumpError> for BinError {
  fn from(err: BumpError) -> Self {
    BinError::BumpError(err)
  }
}

impl From<SlabError> for BinError {
  fn from(err: SlabError) -> Self {
    BinError::SlabError(err)
  }
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
    let _lock = self.lock.lock();

    if let Some(mut head_ptr) = self.head {
      let head_slab = unsafe { head_ptr.as_mut() };
      if let Ok(ptr) = head_slab.allocate() {
        return Ok(ptr);
      }
    }

    if let Some(mut free_ptr) = self.free.take() {
      let free_slab = unsafe { free_ptr.as_mut() };

      if let Some(mut head_ptr) = self.head {
        let head_slab = unsafe { head_ptr.as_mut() };
        List::insert_before(free_slab, head_slab);
      }
      self.head = Some(free_ptr);
      if self.tail.is_none() {
        self.tail = Some(free_ptr);
      }

      let allocated = free_slab.allocate()?;
      return Ok(allocated);
    }

    let new_slab = Slab::new(bump, self.class, self.pages.0, arena)?;

    let slab_mut = unsafe { new_slab.as_ptr().as_mut().unwrap() };

    if let Some(mut head_ptr) = self.head {
      let head_slab = unsafe { head_ptr.as_mut() };
      List::insert_before(slab_mut, head_slab);
    }
    self.head = Some(new_slab);
    if self.tail.is_none() {
      self.tail = Some(new_slab);
    }

    Ok(slab_mut.allocate()?)
  }

  pub fn deallocate(&mut self, ptr: NonNull<u8>, mut slab: NonNull<Slab>) -> BinResult<()> {
    let _lock = self.lock.lock();

    let slab_ref = unsafe { slab.as_mut() };
    slab_ref.deallocate(ptr)?;

    if slab_ref.is_empty() {
      List::remove(slab_ref);

      if Some(slab) == self.head {
        self.head = slab_ref.link().next();
      }
      if Some(slab) == self.tail {
        self.tail = slab_ref.link().prev();
      }

      if let Some(mut old_free) = self.free {
        let old_free_ref = unsafe { old_free.as_mut() };
        List::insert_before(slab_ref, old_free_ref);
      }
      self.free = Some(slab);
    }

    Ok(())
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
