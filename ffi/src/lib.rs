#![no_std]

pub use basealloc::prelude::*;
use core::ptr;

mod handler;

#[unsafe(no_mangle)]
pub extern "C" fn ba_page_size() -> usize {
  page_size()
}

#[unsafe(no_mangle)]
pub extern "C" fn malloc(size: usize) -> *mut u8 {
  _ = size;
  ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn free(ptr: *mut u8) {
  _ = ptr;
  panic!(
    "LOOOOOLOLOLOLOLOLOL where did you get a pointer from?????? THAT CAN ONLY BE NULLLLLL HAHAHAHHAAH"
  );
}

#[unsafe(no_mangle)]
pub extern "C" fn realloc(ptr: *mut u8, size: usize) -> *mut u8 {
  _ = ptr;
  _ = size;
  ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn calloc(num: usize, size: usize) -> *mut u8 {
  _ = num;
  _ = size;
  ptr::null_mut()
}

#[unsafe(no_mangle)]
pub extern "C" fn aligned_alloc(align: usize, size: usize) -> *mut u8 {
  _ = align;
  _ = size;
  ptr::null_mut()
}
