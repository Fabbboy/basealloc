use std::{
  ops::Range,
  ptr::NonNull,
};

use basealloc_extent::{
  Extent,
  ExtentError,
};
use basealloc_ring::Ring;
use basealloc_sync::lazy::LazyLock;
use basealloc_sys::{
  misc::UnsafeStore,
  prim::{
    PrimError,
    page_align,
  },
  system::SysOption,
};

use crate::{
  arena::{
    Arena,
    ArenaError,
  },
  classes::{
    CacheSize,
    NSCLASSES,
    SizeClassIndex,
    cache_for,
    total_cache_size,
  },
};

#[derive(Debug)]
pub enum TCacheError {
  ExtentError(ExtentError),
  PrimError(PrimError),
  ArenaError(ArenaError),
}

pub type TCacheResult<T> = Result<T, TCacheError>;

struct CacheBin {
  store: UnsafeStore<*mut u8>,
  ring: Ring,
}

pub struct TCache {
  _backing: Extent,
  caches: [CacheBin; NSCLASSES],
}

impl TCache {
  pub fn new(size: usize) -> TCacheResult<Self> {
    let pga_size = page_align(size).map_err(TCacheError::PrimError)?;
    let extent = Extent::new(pga_size, SysOption::Commit).map_err(TCacheError::ExtentError)?;
    let caches = Self::new_caches(&extent);

    Ok(Self {
      _backing: extent,
      caches,
    })
  }

  fn new_caches(extent: &Extent) -> [CacheBin; NSCLASSES] {
    let exstart = extent.as_ref().as_ptr() as *mut u8;

    core::array::from_fn(|i| {
      let class_idx = SizeClassIndex(i);
      let range = Self::get_range(0, class_idx);
      let store = Self::construct_store(exstart, range);
      let ring = Ring::new();

      CacheBin { store, ring }
    })
  }

  fn get_range(offset: usize, class_idx: SizeClassIndex) -> Range<usize> {
    let csize = cache_for(class_idx);
    offset..offset + csize.0
  }

  fn from_offset(exstart: *mut u8, offset: usize) -> *mut *mut u8 {
    unsafe { exstart.add(offset) as *mut *mut u8 }
  }

  fn construct_store(exstart: *mut u8, range: Range<usize>) -> UnsafeStore<*mut u8> {
    UnsafeStore::from(unsafe {
      core::slice::from_raw_parts(
        Self::from_offset(exstart, range.start) as *const *mut u8,
        range.end - range.start,
      )
    })
  }

  fn cache_for(&mut self, class_idx: SizeClassIndex) -> &mut CacheBin {
    &mut self.caches[class_idx.0]
  }

  fn refill_cache(&mut self, backing: &mut Arena, sc: SizeClassIndex) -> TCacheResult<()> {
    let CacheSize(cache_size) = cache_for(sc);
    let cache = self.cache_for(sc);
    let buf = cache.store.as_mut_slice();

    let refill_count = cache_size.min(buf.len() - cache.ring.len());

    for _ in 0..refill_count {
      let ptr = backing.allocate(sc).map_err(TCacheError::ArenaError)?;
      if cache.ring.push(buf, ptr.as_ptr()).is_err() {
        break;
      }
    }

    Ok(())
  }

  pub fn allocate(&mut self, backing: &mut Arena, sc: SizeClassIndex) -> TCacheResult<NonNull<u8>> {
    let cache = self.cache_for(sc);
    let buf = cache.store.as_mut_slice();

    if let Some(ptr_ref) = cache.ring.pop(buf) {
      return Ok(unsafe { NonNull::new_unchecked(*ptr_ref) });
    }

    self.refill_cache(backing, sc)?;

    let cache = self.cache_for(sc);
    let buf = cache.store.as_mut_slice();
    let ptr_ref = cache.ring.pop(buf).unwrap();
    Ok(unsafe { NonNull::new_unchecked(*ptr_ref) })
  }
}

thread_local! {
  pub static TCACHE: LazyLock<TCache> = LazyLock::new(|| TCache::new(total_cache_size()).unwrap());
}

pub fn acquire_tcache() -> Option<NonNull<TCache>> {
  TCACHE.try_with(|tc| NonNull::from(&**tc)).ok()
}
