#![cfg_attr(not(test), no_std)]
use basealloc::BaseAlloc;
use core::{
  alloc::{
    GlobalAlloc,
    Layout,
  },
  ptr::{
    self,
  },
};

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
  loop {}
}

#[global_allocator]
static ALLOC: BaseAlloc = BaseAlloc {};

#[unsafe(no_mangle)]
pub extern "C" fn malloc(size: usize) -> *mut u8 {
  if size == 0 {
    return BaseAlloc::sentinel();
  }

  let layout = match Layout::from_size_align(size, 1) {
    Ok(l) => l,
    Err(_) => return ptr::null_mut(),
  };

  unsafe { ALLOC.alloc(layout) }
}

#[unsafe(no_mangle)]
pub extern "C" fn free(ptr: *mut u8) {
  if BaseAlloc::is_invalid(ptr) {
    return;
  }

  let sizeof = BaseAlloc::sizeof(ptr);
  if sizeof.is_none() {
    return;
  }

  let layout = unsafe { Layout::from_size_align_unchecked(sizeof.unwrap(), 1) };
  unsafe { ALLOC.dealloc(ptr, layout) };
}

#[unsafe(no_mangle)]
pub extern "C" fn realloc(ptr: *mut u8, size: usize) -> *mut u8 {
  if BaseAlloc::is_invalid(ptr) {
    return malloc(size);
  }

  if size == 0 {
    free(ptr);
    return BaseAlloc::sentinel();
  }

  let old_size = BaseAlloc::sizeof(ptr);
  if old_size.is_none() {
    return ptr::null_mut();
  }

  let old_layout = unsafe { Layout::from_size_align_unchecked(old_size.unwrap(), 1) };
  let new_layout = match Layout::from_size_align(size, old_layout.align()) {
    Ok(l) => l,
    Err(_) => return ptr::null_mut(),
  };

  let new_ptr = unsafe { ALLOC.alloc(new_layout) };
  if new_ptr.is_null() {
    return new_ptr;
  }

  let copy_size = core::cmp::min(old_layout.size(), new_layout.size());
  unsafe { ptr::copy_nonoverlapping(ptr, new_ptr, copy_size) };
  unsafe { ALLOC.dealloc(ptr, old_layout) };
  new_ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn calloc(num: usize, size: usize) -> *mut u8 {
  let total_size = num.checked_mul(size);
  if total_size.is_none() {
    return ptr::null_mut();
  }

  let total_size = total_size.unwrap();
  if total_size == 0 {
    return BaseAlloc::sentinel();
  }
  let layout = Layout::from_size_align(total_size, 1).ok();
  if layout.is_none() {
    return ptr::null_mut();
  }

  let layout = layout.unwrap();
  let ptr = unsafe { ALLOC.alloc(layout) };
  if ptr.is_null() {
    return ptr;
  }

  unsafe { ptr::write_bytes(ptr, 0, total_size) };
  ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn aligned_alloc(align: usize, size: usize) -> *mut u8 {
  if size == 0 {
    return BaseAlloc::sentinel();
  }

  let layout = Layout::from_size_align(size, align).ok();
  if layout.is_none() {
    return ptr::null_mut();
  }

  let layout = layout.unwrap();
  unsafe { ALLOC.alloc(layout) }
}

#[unsafe(no_mangle)]
pub extern "C" fn malloc_usable_size(ptr: *mut u8) -> usize {
  if BaseAlloc::is_invalid(ptr) {
    return 0;
  }

  let sizeof = BaseAlloc::sizeof(ptr);
  if sizeof.is_none() {
    return 0;
  }

  sizeof.unwrap()
}
