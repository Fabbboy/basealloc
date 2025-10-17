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
    Entry,
    acquire_this_arena,
    lookup,
    unregister_range,
  },
  tcache::acquire_tcache,
};
use basealloc_sync::lazy::LazyLock;
use basealloc_sys::misc::Giveup;

static FALLBACK: LazyLock<AtomicPtr<Arena>> =
  LazyLock::new(|| AtomicPtr::new(unsafe { Arena::new(usize::MAX, CHUNK_SIZE).unwrap().as_ptr() }));

pub struct BaseAlloc {}

impl BaseAlloc {
  pub unsafe fn sizeof(ptr: *mut u8) -> Option<usize> {
    if Self::is_invalid(ptr) {
      return None;
    }

    if let None = lookup(ptr as usize) {
      return None;
    }

    let entry = lookup(ptr as usize).unwrap();
    match entry {
      Entry::Class(cls) => Some(cls.class().0),
      Entry::Large(lrg) => Some(unsafe { lrg.as_ref().size() }),
    }
  }

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
      let cache = unsafe { acquire_tcache().unwrap().as_mut() }; // TODO: handle None
      let ptr = cache.allocate(arena, class);
      return match ptr {
        Ok(p) => p.as_ptr(),
        Err(_) => core::ptr::null_mut(),
      };
    }

    let arena = unsafe { Self::acquire_arena().as_mut() };
    let ptr = arena.allocate_large(layout);
    return match ptr {
      Ok(p) => p.as_ptr(),
      Err(_) => core::ptr::null_mut(),
    };
  }

  unsafe fn dealloc(&self, ptr: *mut u8, _: Layout) {
    if Self::is_invalid(ptr) {
      return;
    }

    if let Some(entry) = lookup(ptr as usize) {
      match entry {
        Entry::Class(_) => {
          todo!()
        }
        &Entry::Large(mut lrg) => {
          let extent = unsafe { lrg.as_mut() };
          let _ = unregister_range(lrg);
          let _ = unsafe { core::ptr::read(extent) }.giveup();
          return;
        }
      }
    }

    todo!()
  }
}
