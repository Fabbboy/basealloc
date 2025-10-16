#![no_std]

use core::{
  alloc::{
    GlobalAlloc,
    Layout,
  },
  ptr::NonNull,
};

use basealloc_alloc::static_::acquire_this_arena;

pub struct BaseAlloc {}

impl BaseAlloc {
  pub fn sizeof(ptr: *mut u8) -> Option<usize> {
    if Self::is_invalid(ptr) {
      return None;
    }

    if let Some(mut arena_ptr) = acquire_this_arena() {
      let arena = unsafe { arena_ptr.as_mut() };
      return arena.sizeof(unsafe { NonNull::new_unchecked(ptr) });
    }

    None
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

    todo!("no fallback yet");
  }

  unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
    if Self::is_invalid(ptr) {
      return;
    }

    if let Some(mut arena_ptr) = acquire_this_arena() {
      let arena = unsafe { arena_ptr.as_mut() };
      arena.deallocate(unsafe { NonNull::new_unchecked(ptr) }, layout);
      return;
    }

    todo!("no fallback yet");
  }
}
