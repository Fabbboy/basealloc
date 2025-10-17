use core::sync::atomic::{
  AtomicPtr,
  Ordering,
};
use std::{
  cell::UnsafeCell,
  ptr::NonNull,
  sync::atomic::AtomicUsize,
};

use basealloc_bitmap::{
  Bitmap,
  BitmapWord,
};
use basealloc_extent::Extent;
use basealloc_rtree::{
  RTree,
  RTreeError,
};
use basealloc_sync::lazy::LazyLock;
use basealloc_sys::{
  prelude::{
    page_align,
    page_align_down,
    page_size,
  },
  prim::PrimError,
};
use getset::Getters;

use crate::{
  MAX_ARENAS,
  WORD_TRAILING,
  arena::{
    Arena,
    ArenaResult,
  },
};

pub const ARENA_BMS: usize = core::mem::size_of::<BitmapWord>() * MAX_ARENAS;

pub const CHUNK_SHIFT: usize = 16 + WORD_TRAILING;
pub const CHUNK_SIZE: usize = 1 << CHUNK_SHIFT;

const BITS_PER_LEVEL: usize = 9;
pub const FANOUT: usize = 1 << BITS_PER_LEVEL;

thread_local! {
  pub static THREAD_ARENA: LazyLock<AtomicPtr<Arena>> = LazyLock::new(|| {
    AtomicPtr::new(acquire_arena().unwrap())
  });

  pub static ARENA_GUARD: ArenaGuard = ArenaGuard;
}

static BM_STORE: [BitmapWord; ARENA_BMS] = [const { BitmapWord::new(0) }; ARENA_BMS];
static BM_LAST: AtomicUsize = AtomicUsize::new(0);
static STATIC: LazyLock<Static> = LazyLock::new(|| Static::new(&BM_STORE));

#[derive(Getters)]
struct Static {
  #[getset(get = "pub")]
  arenas: [AtomicPtr<Arena>; MAX_ARENAS],
  #[getset(get = "pub")]
  bitmap: Bitmap,
}

impl Static {
  pub fn new(store: &'static [BitmapWord]) -> Self {
    let bitmap = Bitmap::zero(store, ARENA_BMS).unwrap();
    let arenas: [AtomicPtr<Arena>; MAX_ARENAS] =
      core::array::from_fn(|_| AtomicPtr::new(core::ptr::null_mut()));

    Self { arenas, bitmap }
  }
}

#[derive(Getters)]
struct LUEntry {
  #[getset(get = "pub")]
  owner: AtomicPtr<Arena>,
}

impl LUEntry {
  pub fn new(owner: NonNull<Arena>) -> Self {
    Self {
      owner: AtomicPtr::new(owner.as_ptr()),
    }
  }
}

unsafe impl Send for LUEntry {}
unsafe impl Sync for LUEntry {}

struct Lookup {
  tree: UnsafeCell<RTree<LUEntry, FANOUT>>,
}

impl Lookup {
  pub const fn new() -> Self {
    Self {
      tree: UnsafeCell::new(RTree::new(CHUNK_SIZE)),
    }
  }

  pub const unsafe fn tree(&self) -> &RTree<LUEntry, FANOUT> {
    unsafe { &*self.tree.get() }
  }

  pub const unsafe fn tree_mut(&self) -> &mut RTree<LUEntry, FANOUT> {
    unsafe { &mut *self.tree.get() }
  }
}

unsafe impl Send for Lookup {}
unsafe impl Sync for Lookup {}

#[derive(Debug)]
pub enum LookupError {
  Tree(RTreeError),
  Align(PrimError),
  RangeOverflow,
  NotFound,
}

static LOOKUP: Lookup = Lookup::new();

struct PageRange {
  start: usize,
  end: usize,
  step: usize,
}

impl PageRange {
  fn iter(&self) -> PageIter {
    PageIter {
      current: self.start,
      end: self.end,
      step: self.step,
    }
  }
}

struct PageIter {
  current: usize,
  end: usize,
  step: usize,
}

impl Iterator for PageIter {
  type Item = usize;

  fn next(&mut self) -> Option<Self::Item> {
    if self.current >= self.end {
      return None;
    }

    let addr = self.current;
    self.current = self.current.saturating_add(self.step);
    Some(addr)
  }
}

fn extent_page_range(extent: NonNull<Extent>) -> Result<Option<PageRange>, LookupError> {
  let extent_ref = unsafe { extent.as_ref() };
  let slice = extent_ref.as_ref();
  let base = slice.as_ptr() as usize;
  let len = slice.len();

  if len == 0 {
    return Ok(None);
  }

  let start = page_align_down(base).map_err(LookupError::Align)?;
  let end_addr = base.checked_add(len).ok_or(LookupError::RangeOverflow)?;
  let end = page_align(end_addr).map_err(LookupError::Align)?;
  let step = page_size();

  Ok(Some(PageRange { start, end, step }))
}

fn rollback_pages(tree: &mut RTree<LUEntry, FANOUT>, pages: &[usize]) {
  for &key in pages {
    let _ = tree.remove(key);
  }
}

pub fn register_extent(extent: NonNull<Extent>, owner: NonNull<Arena>) -> Result<(), LookupError> {
  let Some(range) = extent_page_range(extent)? else {
    return Ok(());
  };

  let tree = unsafe { LOOKUP.tree_mut() };
  let mut inserted = Vec::new();

  for addr in range.iter() {
    match tree.insert(addr, LUEntry::new(owner)) {
      Ok(()) => inserted.push(addr),
      Err(err) => {
        rollback_pages(tree, &inserted);
        return Err(LookupError::Tree(err));
      }
    }
  }

  Ok(())
}

pub fn unregister_extent(extent: NonNull<Extent>) -> Result<(), LookupError> {
  let Some(range) = extent_page_range(extent)? else {
    return Ok(());
  };

  let tree = unsafe { LOOKUP.tree_mut() };
  let mut removed_any = false;

  for addr in range.iter() {
    removed_any |= tree.remove(addr).is_some();
  }

  if removed_any {
    Ok(())
  } else {
    Err(LookupError::NotFound)
  }
}

pub fn lookup_arena(at: usize) -> Option<NonNull<Arena>> {
  let key = page_align_down(at).ok()?;
  let meta = unsafe { LOOKUP.tree() }.lookup(key)?;
  let owner_ptr = meta.owner.load(Ordering::Acquire);
  NonNull::new(owner_ptr)
}

fn create_arena(at: usize) -> ArenaResult<&'static mut Arena> {
  let static_ = &*STATIC;
  let mut arena = unsafe { Arena::new(at, CHUNK_SIZE)? };
  static_.arenas()[at].store(arena.as_ptr(), Ordering::Release);
  Ok(unsafe { arena.as_mut() })
}

fn acquire_arena() -> Option<&'static mut Arena> {
  let static_ = &*STATIC;
  let last = BM_LAST.load(Ordering::Acquire);
  let idx = static_.bitmap().find_fc(Some(last))?;
  static_.bitmap().set(idx).ok()?;

  BM_LAST.store((idx + 1) % ARENA_BMS, Ordering::Release);

  let arena_ptr = static_.arenas()[idx].load(Ordering::Acquire);
  if !arena_ptr.is_null() {
    // SAFETY: We ensure that once an arena is created, its pointer is never changed.
    let arena = unsafe { &mut *arena_ptr };
    return Some(arena);
  }

  create_arena(idx).ok()
}

pub fn acquire_this_arena() -> Option<NonNull<Arena>> {
  THREAD_ARENA
    .try_with(|ta| {
      let ptr = ta.load(Ordering::Acquire);
      if ptr.is_null() {
        return None;
      }

      Some(unsafe { NonNull::new_unchecked(ptr) })
    })
    .ok()
    .flatten()
}

pub unsafe fn release_arena(arena: &'static mut Arena) {
  let static_ = &*STATIC;
  let idx = arena.index();
  static_.bitmap.clear(idx).ok();
  static_.arenas[idx].store(core::ptr::null_mut(), Ordering::Release);
  unsafe {
    core::ptr::drop_in_place(arena);
  }
}

struct ArenaGuard;

impl Drop for ArenaGuard {
  fn drop(&mut self) {
    let arena_ptr = acquire_this_arena();
    if let None = arena_ptr {
      return;
    }

    THREAD_ARENA
      .try_with(|ta| {
        ta.store(core::ptr::null_mut(), Ordering::Release);
      })
      .ok();

    let mut arena_ptr = arena_ptr.unwrap();
    let arena = unsafe { arena_ptr.as_mut() };
    unsafe { release_arena(arena) };
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use basealloc_sys::prelude::SysOption;
  use core::ptr::drop_in_place;

  #[test]
  fn lookup_returns_owner_for_interior_pointer() {
    let ps = page_size();

    let mut extent = Extent::new(ps * 2, SysOption::Reserve).expect("reserve extent");
    let extent_ptr = NonNull::from(&mut extent);

    let arena = unsafe { Arena::new(0, CHUNK_SIZE).expect("arena") };
    let owner = arena;

    register_extent(extent_ptr, owner).expect("register extent");

    let base = extent.as_ref().as_ptr() as usize;
    let interior = base + (ps / 2);

    let found = lookup_arena(interior).expect("lookup interior");
    assert_eq!(found.as_ptr(), owner.as_ptr());

    unregister_extent(extent_ptr).expect("unregister");
    assert!(lookup_arena(interior).is_none());

    unsafe {
      drop_in_place(owner.as_ptr());
    }
  }
}
