use core::{
  cell::UnsafeCell,
  ptr::NonNull,
};

use basealloc_extent::Extent;
use basealloc_rtree::{
  RTree,
  RTreeError,
};
use basealloc_sys::{
  prelude::page_align_down,
  prim::{
    PrimError,
    page_size,
  },
};

use crate::{
  FANOUT,
  arena::ArenaId,
  classes::ScIdx,
  slab::Slab,
};

#[derive(Debug, Clone, Copy)]
pub enum OwnerInfo {
  Slab {
    slab: NonNull<Slab>,
    size_class: ScIdx,
  },
  Extent {
    extent: NonNull<Extent>,
  },
}

impl OwnerInfo {
  pub fn new_slab(slab: NonNull<Slab>, size_class: ScIdx) -> Self {
    Self::Slab { slab, size_class }
  }

  pub fn new_extent(extent: NonNull<Extent>) -> Self {
    Self::Extent { extent }
  }
}

unsafe impl Send for OwnerInfo {}
unsafe impl Sync for OwnerInfo {}

#[derive(Debug)]
pub enum LookupError {
  Tree(RTreeError),
  Align(PrimError),
  RangeOverflow,
  NotFound,
  InvalidArena,
}

impl From<RTreeError> for LookupError {
  fn from(err: RTreeError) -> Self {
    LookupError::Tree(err)
  }
}

impl From<PrimError> for LookupError {
  fn from(err: PrimError) -> Self {
    LookupError::Align(err)
  }
}

pub struct ArenaMap {
  tree: UnsafeCell<RTree<ArenaId, FANOUT>>,
}

impl ArenaMap {
  pub const fn new(chunk_size: usize) -> Self {
    Self {
      tree: UnsafeCell::new(RTree::new(chunk_size)),
    }
  }

  pub unsafe fn tree(&self) -> &RTree<ArenaId, FANOUT> {
    unsafe { &*self.tree.get() }
  }

  #[allow(clippy::mut_from_ref)]
  pub unsafe fn tree_mut(&self) -> &mut RTree<ArenaId, FANOUT> {
    unsafe { &mut *self.tree.get() }
  }

  fn page_range(&self, extent: NonNull<Extent>) -> Result<Option<(usize, usize)>, LookupError> {
    let extent_ref = unsafe { extent.as_ref() };
    let slice = extent_ref.as_ref();
    let base = slice.as_ptr() as usize;
    let len = slice.len();

    if len == 0 {
      return Ok(None);
    }

    let start = page_align_down(base)?;
    let end_addr = base.checked_add(len).ok_or(LookupError::RangeOverflow)?;
    let last_page = page_align_down(end_addr.saturating_sub(1))?;

    Ok(Some((start, last_page)))
  }

  fn range_execute<F>(
    &self,
    start: usize,
    stop: usize,
    step: usize,
    f: F,
  ) -> Result<(), LookupError>
  where
    F: FnMut(usize) -> Result<(), LookupError>,
  {
    let mut current = start;
    let mut func = f;
    while current <= stop {
      func(current)?;
      current = current
        .checked_add(step)
        .ok_or(LookupError::RangeOverflow)?;
    }
    Ok(())
  }

  pub fn associate(&self, extent: NonNull<Extent>, id: ArenaId) -> Result<(), LookupError> {
    let Some((start, last_page)) = self.page_range(extent)? else {
      return Ok(());
    };

    let page_sz = page_size();
    self.range_execute(start, last_page, page_sz, |addr| {
      let tree = unsafe { self.tree_mut() };
      Ok(tree.insert(addr, id)?)
    })?;

    Ok(())
  }

  pub fn detach(&self, extent: NonNull<Extent>) -> Result<(), LookupError> {
    let Some((start, last_page)) = self.page_range(extent)? else {
      return Ok(());
    };

    let page_sz = page_size();
    let mut removed_any = false;
    self.range_execute(start, last_page, page_sz, |addr| {
      let tree = unsafe { self.tree_mut() };
      if tree.remove(addr).is_some() {
        removed_any = true;
      }
      Ok(())
    })?;

    if removed_any {
      Ok(())
    } else {
      Err(LookupError::NotFound)
    }
  }

  pub fn lookup(&self, addr: usize) -> Option<ArenaId> {
    let aligned_addr = page_align_down(addr).ok()?;
    unsafe { self.tree() }.lookup(aligned_addr).copied()
  }
}

unsafe impl Send for ArenaMap {}
unsafe impl Sync for ArenaMap {}

pub struct ExtentTree {
  tree: UnsafeCell<RTree<OwnerInfo, FANOUT>>,
}

impl ExtentTree {
  pub const fn new(chunk_size: usize) -> Self {
    Self {
      tree: UnsafeCell::new(RTree::new(chunk_size)),
    }
  }

  pub unsafe fn tree(&self) -> &RTree<OwnerInfo, FANOUT> {
    unsafe { &*self.tree.get() }
  }

  #[allow(clippy::mut_from_ref)]
  pub unsafe fn tree_mut(&self) -> &mut RTree<OwnerInfo, FANOUT> {
    unsafe { &mut *self.tree.get() }
  }

  fn page_range(extent: NonNull<Extent>) -> Result<Option<(usize, usize)>, LookupError> {
    let extent_ref = unsafe { extent.as_ref() };
    let slice = extent_ref.as_ref();
    let base = slice.as_ptr() as usize;
    let len = slice.len();

    if len == 0 {
      return Ok(None);
    }

    let start = page_align_down(base)?;
    let end_addr = base.checked_add(len).ok_or(LookupError::RangeOverflow)?;
    let last_page = page_align_down(end_addr.saturating_sub(1))?;

    Ok(Some((start, last_page)))
  }

  fn range_execute<F>(
    &self,
    start: usize,
    stop: usize,
    step: usize,
    f: F,
  ) -> Result<(), LookupError>
  where
    F: FnMut(usize) -> Result<(), LookupError>,
  {
    let mut current = start;
    let mut func = f;
    while current <= stop {
      func(current)?;
      current = current
        .checked_add(step)
        .ok_or(LookupError::RangeOverflow)?;
    }
    Ok(())
  }

  pub fn register(&self, extent: NonNull<Extent>, info: OwnerInfo) -> Result<(), LookupError> {
    let Some((start, last_page)) = Self::page_range(extent)? else {
      return Ok(());
    };

    let page_sz = page_size();
    self.range_execute(start, last_page, page_sz, |addr| {
      let tree = unsafe { self.tree_mut() };
      Ok(tree.insert(addr, info.clone())?)
    })?;

    Ok(())
  }

  pub fn unregister(&self, extent: NonNull<Extent>) -> Result<(), LookupError> {
    let Some((start, last_page)) = Self::page_range(extent)? else {
      return Ok(());
    };

    let page_sz = page_size();
    let mut removed_any = false;
    self.range_execute(start, last_page, page_sz, |addr| {
      let tree = unsafe { self.tree_mut() };
      if tree.remove(addr).is_some() {
        removed_any = true;
      }
      Ok(())
    })?;

    if removed_any {
      Ok(())
    } else {
      Err(LookupError::NotFound)
    }
  }

  pub fn lookup(&self, addr: usize) -> Option<&OwnerInfo> {
    let aligned_addr = page_align_down(addr).ok()?;
    unsafe { self.tree() }.lookup(aligned_addr)
  }
}

unsafe impl Send for ExtentTree {}
unsafe impl Sync for ExtentTree {}

#[derive(Debug, Clone, Copy)]
pub struct LookupResult {
  pub arena_id: ArenaId,
  pub owner_info: OwnerInfo,
}

impl LookupResult {
  pub fn new(arena_id: ArenaId, owner_info: OwnerInfo) -> Self {
    Self {
      arena_id,
      owner_info,
    }
  }
}
