use core::{
  ptr::NonNull,
  sync::atomic::{
    AtomicPtr,
    AtomicUsize,
    Ordering,
  },
};

use basealloc_bitmap::{
  Bitmap,
  BitmapWord,
};
use basealloc_sync::{
  lazy::LazyLock,
  local::ThreadLocal,
};
use getset::Getters;

use crate::{
  ARENA_BMS,
  CHUNK_SIZE,
  MAX_ARENAS,
  arena::{
    Arena,
    ArenaId,
    ArenaResult,
  },
  lookup::{
    ArenaMap,
    LookupError,
  },
};

// Storage
static BM_STORE: [BitmapWord; ARENA_BMS] = [const { BitmapWord::new(0) }; ARENA_BMS];
static BM_LAST: AtomicUsize = AtomicUsize::new(0);
static STATIC: LazyLock<Static> = LazyLock::new(|| Static::new(&BM_STORE));
static ARENA_MAP: ArenaMap = ArenaMap::new(CHUNK_SIZE);

static THREAD_ARENA: ThreadLocal<AtomicPtr<Arena>> =
  ThreadLocal::new(|| AtomicPtr::new(acquire_arena().unwrap()));

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

// Basic access functions
pub fn register_chunk(addr: usize, arena: ArenaId) -> Result<(), LookupError> {
  ARENA_MAP.register(addr, arena)
}

pub fn lookup(addr: usize) -> Option<ArenaId> {
  ARENA_MAP.lookup(addr)
}

pub fn get_arena(arena_id: ArenaId) -> Option<&'static mut Arena> {
  let static_ = &*STATIC;
  let arena_ptr = static_.arenas()[arena_id.0].load(Ordering::Acquire);
  if arena_ptr.is_null() {
    None
  } else {
    Some(unsafe { &mut *arena_ptr })
  }
}

fn create_arena(at: usize) -> ArenaResult<&'static mut Arena> {
  let static_ = &*STATIC;
  let mut arena = unsafe { Arena::new(at, CHUNK_SIZE)? };
  static_.arenas()[at].store(arena.as_ptr(), Ordering::Release);
  
  let arena_id = ArenaId(at);
  let chunk_addr = arena.as_ptr() as usize;
  let _ = register_chunk(chunk_addr, arena_id);
  
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
    let arena = unsafe { &mut *arena_ptr };
    return Some(arena);
  }

  create_arena(idx).ok()
}

pub fn acquire_this_arena() -> Option<NonNull<Arena>> {
  THREAD_ARENA.with(|ta| {
    let ptr = ta.load(Ordering::Acquire);
    if ptr.is_null() {
      return None;
    }

    Some(unsafe { NonNull::new_unchecked(ptr) })
  })
}