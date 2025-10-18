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
  ARENA_BMS,
  CHUNK_SIZE,
  FANOUT,
  MAX_ARENAS,
  arena::{
    Arena,
    ArenaResult,
  },
  classes::SizeClass,
  slab::Slab,
};

thread_local! {
  pub static THREAD_ARENA: LazyLock<AtomicPtr<Arena>> = LazyLock::new(|| {
    AtomicPtr::new(acquire_arena().unwrap())
  });

  pub static ARENA_GUARD: ArenaGuard = const { ArenaGuard };
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
pub struct ClassEntry {
  #[getset(get = "pub")]
  slab: NonNull<Slab>,
  #[getset(get = "pub")]
  class: SizeClass,
  #[getset(get = "pub")]
  arena: NonNull<Arena>,
}

impl ClassEntry {
  fn new(slab: NonNull<Slab>, class: SizeClass, arena: NonNull<Arena>) -> Self {
    Self { slab, class, arena }
  }
}

pub enum Entry {
  Class(ClassEntry),
  Large(NonNull<Extent>),
}

impl Entry {
  fn new_sc(entry: ClassEntry) -> Self {
    Entry::Class(entry)
  }

  fn new_large(extent: NonNull<Extent>) -> Self {
    Entry::Large(extent)
  }
}

unsafe impl Send for Entry {}
unsafe impl Sync for Entry {}

struct Lookup {
  tree: UnsafeCell<RTree<Entry, FANOUT>>,
}

impl Lookup {
  pub const fn new() -> Self {
    Self {
      tree: UnsafeCell::new(RTree::new(CHUNK_SIZE)),
    }
  }

  pub const unsafe fn tree(&self) -> &RTree<Entry, FANOUT> {
    unsafe { &*self.tree.get() }
  }

  #[allow(clippy::mut_from_ref)]
  pub unsafe fn tree_mut(&self) -> &mut RTree<Entry, FANOUT> {
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

struct LookupRange {
  start: usize,
  end: usize,
  step: usize,
}

impl LookupRange {
  fn next_addr(&self, current: usize) -> Result<usize, LookupError> {
    current
      .checked_add(self.step)
      .ok_or(LookupError::RangeOverflow)
  }
}

fn extent_range(extent: NonNull<Extent>) -> Result<Option<LookupRange>, LookupError> {
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

  Ok(Some(LookupRange { start, end, step }))
}

fn rollback_range(tree: &mut RTree<Entry, FANOUT>, range: &LookupRange, upto: usize) {
  let mut addr = range.start;
  while addr < upto {
    let _ = tree.remove(addr);
    addr = match range.next_addr(addr) {
      Ok(next) => next,
      Err(_) => break,
    };
  }
}

pub fn register_sc(
  range: NonNull<Extent>,
  slab: NonNull<Slab>,
  class: SizeClass,
  arena: NonNull<Arena>,
) -> Result<(), LookupError> {
  let Some(range) = extent_range(range)? else {
    return Ok(());
  };

  let tree = unsafe { LOOKUP.tree_mut() };

  register_entries(tree, &range, || {
    Entry::new_sc(ClassEntry::new(slab, class, arena))
  })
}

pub fn register_large(extent: NonNull<Extent>) -> Result<(), LookupError> {
  let Some(range) = extent_range(extent)? else {
    return Ok(());
  };

  let tree = unsafe { LOOKUP.tree_mut() };
  register_entries(tree, &range, || Entry::new_large(extent))
}

fn register_entries<F>(
  tree: &mut RTree<Entry, FANOUT>,
  range: &LookupRange,
  mut make_entry: F,
) -> Result<(), LookupError>
where
  F: FnMut() -> Entry,
{
  let mut addr = range.start;

  while addr < range.end {
    match tree.insert(addr, make_entry()) {
      Ok(()) => {
        addr = range.next_addr(addr)?;
      }
      Err(err) => {
        rollback_range(tree, range, addr);
        return Err(LookupError::Tree(err));
      }
    }
  }

  Ok(())
}

pub fn unregister_range(extent: NonNull<Extent>) -> Result<(), LookupError> {
  let Some(range) = extent_range(extent)? else {
    return Ok(());
  };

  let tree = unsafe { LOOKUP.tree_mut() };
  let mut removed_any = false;
  let mut addr = range.start;

  while addr < range.end {
    removed_any |= tree.remove(addr).is_some();
    addr = range.next_addr(addr)?;
  }

  if removed_any {
    Ok(())
  } else {
    Err(LookupError::NotFound)
  }
}

pub fn lookup(at: usize) -> Option<&'static Entry> {
  let key = page_align_down(at).ok()?;
  unsafe { LOOKUP.tree() }.lookup(key)
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

/// Releases an arena back to the global pool.
///
/// # Safety
///
/// The caller must ensure that:
/// - `arena` is a valid arena obtained from `acquire_arena`
/// - No references to the arena or its allocations remain after this call
/// - The arena is not accessed after this function returns
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
    if arena_ptr.is_none() {
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
