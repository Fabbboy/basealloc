#![no_std]

use core::alloc::{
  GlobalAlloc,
  Layout,
};
pub mod prelude {
  pub use basealloc_list::prelude::*;
  pub use basealloc_sys::prelude::*;
}

pub struct BaseAlloc {}

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
