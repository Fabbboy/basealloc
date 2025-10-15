use core::sync::atomic::{
  AtomicPtr,
  Ordering,
};
use std::{
  ptr::NonNull,
  sync::LazyLock,
};

use basealloc_bitmap::{
  Bitmap,
  BitmapWord,
};
use basealloc_rtree::RTree;
use basealloc_sys::extent::Extent;
use getset::Getters;
use heapless::Vec;

use crate::{
  arena::{
    Arena,
    ArenaResult,
  },
  config::{
    ARENA_BMS,
    CHUNK_SIZE,
    FANOUT,
    MAX_ARENAS,
  },
};

thread_local! {
  pub static THREAD_ARENA: LazyLock<AtomicPtr<Arena>> = LazyLock::new(|| {
    AtomicPtr::new(aquire_arena().unwrap())
  });

  pub static ARENA_GUARD: ArenaGuard = ArenaGuard;
}

#[derive(Getters)]
struct Static {
  #[getset(get = "pub")]
  arenas: Vec<AtomicPtr<Arena>, { MAX_ARENAS }>,
  #[getset(get = "pub")]
  bitmap: Bitmap<'static>,
}

impl Static {
  pub fn new(store: &'static [BitmapWord]) -> Self {
    let bitmap = Bitmap::zero(store, ARENA_BMS).unwrap();
    let mut arenas = Vec::new();
    let mut i = 0;
    while i < MAX_ARENAS {
      arenas.push(AtomicPtr::new(core::ptr::null_mut())).unwrap();
      i += 1;
    }

    Self { arenas, bitmap }
  }
}

static BM_STORE: [BitmapWord; ARENA_BMS] = [const { BitmapWord::new(0) }; ARENA_BMS];
static STATIC: LazyLock<Static> = LazyLock::new(|| {
  let s = Static::new(&BM_STORE);
  s
});

pub static EMAP: RTree<Extent, FANOUT> = RTree::new(CHUNK_SIZE);

fn create_arena(at: usize) -> ArenaResult<&'static mut Arena> {
  let static_ = &*STATIC;
  let mut arena = unsafe { Arena::new(at, CHUNK_SIZE)? };
  static_.arenas()[at].store(arena.as_ptr(), Ordering::Release);
  Ok(unsafe { arena.as_mut() })
}

fn aquire_arena() -> Option<&'static mut Arena> {
  let static_ = &*STATIC;
  let idx = static_.bitmap().find_fc()?;
  static_.bitmap().set(idx).ok()?;

  let arena_ptr = static_.arenas()[idx].load(Ordering::Acquire);
  if !arena_ptr.is_null() {
    // SAFETY: We ensure that once an arena is created, its pointer is never changed.
    let arena = unsafe { &mut *arena_ptr };
    return Some(arena);
  }

  create_arena(idx).ok()
}

pub unsafe fn aquire_this_arena() -> Option<NonNull<Arena>> {
  THREAD_ARENA.with(|ta| {
    let ptr = ta.load(Ordering::Acquire);
    if ptr.is_null() {
      None
    } else {
      Some(unsafe { NonNull::new_unchecked(ptr) })
    }
  })
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
    let arena_ptr = unsafe { aquire_this_arena() };
    if let None = arena_ptr {
      return;
    }

    THREAD_ARENA.with(|ta| {
      ta.store(core::ptr::null_mut(), Ordering::Release);
    });

    let mut arena_ptr = arena_ptr.unwrap();
    let arena = unsafe { arena_ptr.as_mut() };
    unsafe { release_arena(arena) };
  }
}
