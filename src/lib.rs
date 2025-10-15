#![no_std]

use core::{
  alloc::{
    GlobalAlloc,
    Layout,
  },
  ptr::NonNull,
};

pub struct BaseAlloc {}

impl BaseAlloc {
  pub fn info(ptr: *mut u8) -> Option<Layout> {
    _ = ptr;
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
    _ = layout;
    core::ptr::null_mut()
  }

  unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
    _ = layout;
    _ = ptr;
  }
}
