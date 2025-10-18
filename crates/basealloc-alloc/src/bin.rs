use core::ptr::NonNull;

use basealloc_extent::ExtentError;
use basealloc_fixed::bump::{
  Bump,
  BumpError,
};
use basealloc_list::{
  HasLink,
  List,
};

use crate::{
  arena::Arena,
  classes::{
    ScIdx,
    SizeClass,
    SlabPages,
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
  ExtentError(ExtentError),
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

impl From<ExtentError> for BinError {
  fn from(err: ExtentError) -> Self {
    BinError::ExtentError(err)
  }
}

pub type BinResult<T> = Result<T, BinError>;

pub struct Bin {
  // SAFETY: User must ensure bin is dropped before bump.
  class: SizeClass,
  pages: SlabPages,
  free_head: Option<NonNull<Slab>>,
  active_head: Option<NonNull<Slab>>,
  active_tail: Option<NonNull<Slab>>,
}

impl Bin {
  pub fn new(idx: ScIdx) -> Self {
    Self {
      class: class_at(idx),
      pages: pages_for(idx),
      free_head: None,
      active_head: None,
      active_tail: None,
    }
  }

  fn alloc_fast(&mut self) -> Option<NonNull<u8>> {
    let active_ptr = self.active_head?;
    let active_slab = unsafe { active_ptr.as_ptr().as_mut().unwrap() };
    active_slab.allocate().ok()
  }

  fn pop_free(&mut self) -> Option<NonNull<Slab>> {
    let free_ptr = self.free_head.take()?;
    let free_slab = unsafe { free_ptr.as_ptr().as_mut().unwrap() };

    self.free_head = if let Some(next) = free_slab.link().next() {
      List::remove(free_slab);
      Some(next)
    } else {
      None
    };

    if let Some(active_head_ptr) = self.active_head {
      let active_head_slab = unsafe { active_head_ptr.as_ptr().as_mut().unwrap() };
      List::insert_before(free_slab, active_head_slab);
    }
    self.active_head = Some(free_ptr);
    if self.active_tail.is_none() {
      self.active_tail = Some(free_ptr);
    }

    Some(free_ptr)
  }

  fn push_new(&mut self, bump: &mut Bump, arena: NonNull<Arena>) -> BinResult<NonNull<Slab>> {
    let new_slab = Slab::new(bump, self.class, self.pages.0, arena)?;
    let slab_mut = unsafe { new_slab.as_ptr().as_mut().unwrap() };

    if let Some(active_head_ptr) = self.active_head {
      let active_head_slab = unsafe { active_head_ptr.as_ptr().as_mut().unwrap() };
      List::insert_before(slab_mut, active_head_slab);
    }
    self.active_head = Some(new_slab);
    if self.active_tail.is_none() {
      self.active_tail = Some(new_slab);
    }

    Ok(new_slab)
  }

  fn retire_slab(&mut self, slab: NonNull<Slab>, slab_ref: &mut Slab) -> BinResult<()> {
    List::remove(slab_ref);

    if Some(slab) == self.active_head {
      self.active_head = slab_ref.link().next();
    }
    if Some(slab) == self.active_tail {
      self.active_tail = slab_ref.link().prev();
    }

    slab_ref.extent_mut().deactivate()?;

    if let Some(mut free_head_ptr) = self.free_head {
      let free_head_slab = unsafe { free_head_ptr.as_mut() };
      List::insert_before(slab_ref, free_head_slab);
    }
    self.free_head = Some(slab);

    Ok(())
  }

  pub fn allocate(&mut self, bump: &mut Bump, arena: NonNull<Arena>) -> BinResult<NonNull<u8>> {
    if let Some(ptr) = self.alloc_fast() {
      return Ok(ptr);
    }

    if let Some(slab) = self.pop_free() {
      let slab_mut = unsafe { slab.as_ptr().as_mut().unwrap() };
      return Ok(slab_mut.allocate()?);
    }

    let new_slab = self.push_new(bump, arena)?;
    let slab_mut = unsafe { new_slab.as_ptr().as_mut().unwrap() };
    Ok(slab_mut.allocate()?)
  }

  pub fn deallocate(&mut self, ptr: NonNull<u8>, mut slab: NonNull<Slab>) -> BinResult<()> {
    let slab_ref = unsafe { slab.as_mut() };
    slab_ref.deallocate(ptr)?;

    if slab_ref.is_empty() {
      self.retire_slab(slab, slab_ref)?;
    }

    Ok(())
  }
}

impl Drop for Bin {
  fn drop(&mut self) {
    if let Some(mut free_head) = self.free_head {
      unsafe {
        let _ = List::drain(free_head.as_mut());
      }
    }

    if let Some(mut active_head) = self.active_head {
      unsafe {
        let _ = List::drain(active_head.as_mut());
      }
    }
  }
}
