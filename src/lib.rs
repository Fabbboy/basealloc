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
  arena::Arena,
  static_::{
    acquire_this_arena,
    lookup_arena,
  }, CHUNK_SIZE,
};
use basealloc_sync::lazy::LazyLock;

static FALLBACK: LazyLock<AtomicPtr<Arena>> =
  LazyLock::new(|| AtomicPtr::new(unsafe { Arena::new(usize::MAX, CHUNK_SIZE).unwrap().as_ptr() }));

pub struct BaseAlloc {}

impl BaseAlloc {
  pub unsafe fn sizeof(ptr: *mut u8) -> Option<usize> {
    if Self::is_invalid(ptr) {
      return None;
    }

    if let Some(mut arena_ptr) = lookup_arena(ptr as usize) {
      let arena = unsafe { arena_ptr.as_mut() };
      return arena.sizeof(unsafe { NonNull::new_unchecked(ptr) });
    }

    let fallback = FALLBACK.load(Ordering::Acquire);
    if fallback.is_null() {
      return None;
    }

    let arena = unsafe { &mut *fallback };
    arena.sizeof(unsafe { NonNull::new_unchecked(ptr) })
  }

  pub fn is_invalid(ptr: *mut u8) -> bool {
    ptr.is_null() || ptr == Self::sentinel()
  }

  pub fn sentinel() -> *mut u8 {
    NonNull::dangling().as_ptr()
  }
}

unsafe impl GlobalAlloc for BaseAlloc {
  unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
    if let Some(mut arena_ptr) = acquire_this_arena() {
      let arena = unsafe { arena_ptr.as_mut() };
      if let Ok(ptr) = arena.allocate(layout) {
        return ptr.as_ptr();
      }
    }

    let fallback = FALLBACK.load(Ordering::Acquire);
    if fallback.is_null() {
      return core::ptr::null_mut();
    }

    let arena = unsafe { &mut *fallback };
    if let Ok(ptr) = arena.allocate(layout) {
      return ptr.as_ptr();
    }

    core::ptr::null_mut()
  }

  unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
    if Self::is_invalid(ptr) {
      return;
    }

    if let Some(mut arena_ptr) = lookup_arena(ptr as usize) {
      let arena = unsafe { arena_ptr.as_mut() };
      let _ = arena.deallocate(unsafe { NonNull::new_unchecked(ptr) }, layout);
      return;
    }

    let fallback = FALLBACK.load(Ordering::Acquire);
    if fallback.is_null() {
      return;
    }

    let arena = unsafe { &mut *fallback };
    let _ = arena.deallocate(unsafe { NonNull::new_unchecked(ptr) }, layout);
  }
}
