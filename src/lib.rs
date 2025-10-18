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
  },
  tcache::acquire_tcache,
};
use basealloc_sync::lazy::LazyLock;

static FALLBACK: LazyLock<AtomicPtr<Arena>> =
  LazyLock::new(|| AtomicPtr::new(unsafe { Arena::new(usize::MAX, CHUNK_SIZE).unwrap().as_ptr() }));

pub struct BaseAlloc {}

impl BaseAlloc {
  /// Returns the size of the allocation pointed to by `ptr`.
  ///
  /// # Safety
  ///
  /// The caller must ensure that `ptr` is either null or points to a valid
  /// allocation made by this allocator.
  pub unsafe fn sizeof(ptr: *mut u8) -> Option<usize> {
    if Self::is_invalid(ptr) {
      return None;
    }

    lookup(ptr as usize)?;

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
      if let Some(mut tcache_ptr) = acquire_tcache() {
        let tcache = unsafe { tcache_ptr.as_mut() };
        let arena = unsafe { Self::acquire_arena().as_mut() };
        if let Ok(ptr) = tcache.allocate(arena, class) {
          return ptr.as_ptr();
        }
      }
 
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

    if let Some(entry) = lookup(ptr as usize) {
      match entry {
        Entry::Class(class_entry) => {
          let ptr_nn = unsafe { NonNull::new_unchecked(ptr) };

          if let Some(mut tcache_ptr) = acquire_tcache() {
            let tcache = unsafe { tcache_ptr.as_mut() };
            let arena = unsafe { Self::acquire_arena().as_mut() };
            if tcache
              .deallocate(arena, ptr_nn, class_entry.class().1)
              .is_ok()
            {
              return;
            }
          }
          let arena = unsafe { Self::acquire_arena().as_mut() };
          let _ = arena.deallocate(ptr_nn, class_entry);
        }
        &Entry::Large(lrg) => {
          let arena = unsafe { Self::acquire_arena().as_mut() };
          let _ = arena.deallocate_large(lrg);
        }
      }
    }
  }
}
