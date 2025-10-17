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
  bin::Bin,
  classes::SizeClass,
  slab::Slab,
};

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
pub struct ClassEntry {
  #[getset(get = "pub")]
  bin: NonNull<Bin>,
  #[getset(get = "pub")]
  slab: NonNull<Slab>,
  #[getset(get = "pub")]
  class: SizeClass,
}

impl ClassEntry {
  fn new(bin: NonNull<Bin>, slab: NonNull<Slab>, class: SizeClass) -> Self {
    Self { bin, slab, class }
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

  pub const unsafe fn tree_mut(&self) -> &mut RTree<Entry, FANOUT> {
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
  bin: NonNull<Bin>,
  slab: NonNull<Slab>,
  class: SizeClass,
) -> Result<(), LookupError> {
  let Some(range) = extent_range(range)? else {
    return Ok(());
  };

  let tree = unsafe { LOOKUP.tree_mut() };

  register_entries(tree, &range, || {
    Entry::new_sc(ClassEntry::new(bin, slab, class))
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
  use crate::{
    bin::Bin,
    classes::{
      QUANTUM,
      class_for,
    },
    slab::Slab,
  };
  use basealloc_fixed::bump::Bump;
  use basealloc_sys::prelude::SysOption;
  use core::ptr::drop_in_place;
  use std::sync::Mutex;

  static TEST_LOCK: Mutex<()> = Mutex::new(());

  fn create_test_bin_and_slab() -> (NonNull<Bin>, NonNull<Slab>, SizeClass) {
    let mut bump = Bump::new(CHUNK_SIZE);
    let class_idx = class_for(QUANTUM).expect("get class");
    let class = crate::classes::class_at(class_idx);
    let slab_size = crate::classes::pages_for(class_idx).0;

    let bin_uninit_ptr = bump.create::<Bin>().expect("create bin");
    let bin = Bin::new(class_idx);
    let bin_raw_ptr = bin_uninit_ptr as *mut Bin;
    unsafe { core::ptr::write(bin_raw_ptr, bin) };
    let mut bin_ptr = unsafe { NonNull::new_unchecked(bin_raw_ptr) };

    let slab_ptr =
      Slab::new(&mut bump, class, slab_size, unsafe { bin_ptr.as_mut() }).expect("create slab");

    (bin_ptr, slab_ptr, class)
  }

  #[test]
  fn lookup_returns_owner_for_interior_pointer() {
    let _guard = TEST_LOCK.lock().unwrap();
    let ps = page_size();

    let mut extent = Extent::new(ps * 2, SysOption::Reserve).expect("reserve extent");
    let extent_ptr = NonNull::from(&mut extent);

    let arena = unsafe { Arena::new(0, CHUNK_SIZE).expect("arena") };
    let owner = arena;
    let (bin_ptr, slab_ptr, class) = create_test_bin_and_slab();

    register_sc(extent_ptr, bin_ptr, slab_ptr, class).expect("register extent");

    let base = extent.as_ref().as_ptr() as usize;
    let interior = base + (ps / 2);

    let found = lookup(interior).expect("lookup interior");
    assert!(matches!(found, Entry::Class(_)));

    unregister_range(extent_ptr).expect("unregister");
    assert!(lookup(interior).is_none());

    unsafe {
      drop_in_place(owner.as_ptr());
    }
  }

  #[test]
  fn unregister_allows_reregistration() {
    let _guard = TEST_LOCK.lock().unwrap();
    let ps = page_size();

    let mut extent = Extent::new(ps, SysOption::Reserve).expect("reserve extent");
    let extent_ptr = NonNull::from(&mut extent);

    let first_arena = unsafe { Arena::new(3, CHUNK_SIZE).expect("first arena") };
    let second_arena = unsafe { Arena::new(4, CHUNK_SIZE).expect("second arena") };
    let (bin_ptr, slab_ptr, class) = create_test_bin_and_slab();

    register_sc(extent_ptr, bin_ptr, slab_ptr, class).expect("register once");

    let base = extent.as_ref().as_ptr() as usize;
    assert!(matches!(lookup(base).unwrap(), Entry::Class(_)));

    unregister_range(extent_ptr).expect("unregister");
    assert!(lookup(base).is_none(), "mapping should be cleared");

    let (bin_ptr2, slab_ptr2, class2) = create_test_bin_and_slab();
    register_sc(extent_ptr, bin_ptr2, slab_ptr2, class2).expect("register twice");
    assert!(matches!(lookup(base).unwrap(), Entry::Class(_)));

    unregister_range(extent_ptr).expect("final unregister");

    unsafe {
      drop_in_place(first_arena.as_ptr());
      drop_in_place(second_arena.as_ptr());
    }
  }

  #[test]
  fn failed_registration_leaves_existing_mapping_intact() {
    let _guard = TEST_LOCK.lock().unwrap();
    let ps = page_size();

    let mut extent = Extent::new(ps, SysOption::Reserve).expect("reserve extent");
    let extent_ptr = NonNull::from(&mut extent);

    let first_arena = unsafe { Arena::new(5, CHUNK_SIZE).expect("first arena") };
    let second_arena = unsafe { Arena::new(6, CHUNK_SIZE).expect("second arena") };
    let (bin_ptr, slab_ptr, class) = create_test_bin_and_slab();

    register_sc(extent_ptr, bin_ptr, slab_ptr, class).expect("initial register");

    let base = extent.as_ref().as_ptr() as usize;
    assert!(matches!(lookup(base).unwrap(), Entry::Class(_)));

    let (bin_ptr2, slab_ptr2, class2) = create_test_bin_and_slab();
    let err =
      register_sc(extent_ptr, bin_ptr2, slab_ptr2, class2).expect_err("duplicate should fail");
    assert!(matches!(err, LookupError::Tree(RTreeError::AlreadyPresent)));

    assert!(matches!(lookup(base).unwrap(), Entry::Class(_)));

    unregister_range(extent_ptr).expect("unregister");
    assert!(lookup(base).is_none());

    unsafe {
      drop_in_place(first_arena.as_ptr());
      drop_in_place(second_arena.as_ptr());
    }
  }
}
