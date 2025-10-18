#![no_std]

use core::{
  alloc::{
    GlobalAlloc,
    Layout,
  },
  ptr::NonNull,
  sync::atomic::{
    AtomicPtr,
    Ordering,
  },
};

use basealloc_alloc::{
  CHUNK_SIZE,
  arena::Arena,
  classes::class_for,
  static_::{
    acquire_this_arena,
    lookup,
    get_arena,
  },
};
use basealloc_sync::lazy::LazyLock;

static FALLBACK: LazyLock<AtomicPtr<Arena>> =
  LazyLock::new(|| AtomicPtr::new(unsafe { Arena::new(usize::MAX, CHUNK_SIZE).unwrap().as_ptr() }));

pub struct BaseAlloc {}

impl BaseAlloc {
  pub fn is_invalid(ptr: *mut u8) -> bool {
    ptr.is_null() || ptr == Self::sentinel()
  }

  pub fn sentinel() -> *mut u8 {
    NonNull::dangling().as_ptr()
  }

  fn acquire_arena() -> NonNull<Arena> {
    acquire_this_arena().unwrap_or_else(|| {
      let fallback_ptr = FALLBACK.load(Ordering::Acquire);
      unsafe { NonNull::new_unchecked(fallback_ptr) }
    })
  }
}

unsafe impl GlobalAlloc for BaseAlloc {
  unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
    let class = class_for(layout.size());
    if let Some(class) = class {
      let arena = unsafe { Self::acquire_arena().as_mut() };
      let ptr = arena.allocate(class);
      return match ptr {
        Ok(p) => p.as_ptr(),
        Err(_) => core::ptr::null_mut(),
      };
    }

    let arena = unsafe { Self::acquire_arena().as_mut() };
    let ptr = arena.allocate_large(layout);
    match ptr {
      Ok(p) => p.as_ptr(),
      Err(_) => core::ptr::null_mut(),
    }
  }

  unsafe fn dealloc(&self, ptr: *mut u8, _: Layout) {
    if Self::is_invalid(ptr) {
      return;
    }

    let ptr_nn = unsafe { NonNull::new_unchecked(ptr) };
    let arena_id = lookup(ptr as usize).unwrap();
    let arena = get_arena(arena_id).unwrap();
    arena.deallocate(ptr_nn).unwrap();
  }
}
