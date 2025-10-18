use core::{
  alloc::Layout,
  ptr::NonNull,
};

use basealloc_extent::{
  Extent,
  ExtentError,
};
use basealloc_fixed::bump::{
  Bump,
  BumpError,
};
use getset::{
  CloneGetters,
  Getters,
  MutGetters,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArenaId(pub usize);

use crate::{
  bin::{
    Bin,
    BinError,
  },
  classes::{
    NSCLASSES,
    ScIdx,
  },
  lookup::{
    ExtentTree,
    LookupError,
    OwnerInfo,
  },
  static_::ARENA_MAP,
};

use basealloc_sys::{
  misc::Giveup,
  prim::{
    PrimError,
    page_align,
  },
  system::SysOption,
};

#[derive(Debug)]
pub enum ArenaError {
  BumpError(BumpError),
  BinError(BinError),
  LookupError(LookupError),
  ExtentError(ExtentError),
  PrimError(PrimError),
}

pub type ArenaResult<T> = Result<T, ArenaError>;

#[derive(Getters, MutGetters, CloneGetters)]
pub struct Arena {
  #[getset(get_clone = "pub")]
  index: ArenaId,
  bins: [Bin; NSCLASSES],
  bump: Bump,
  #[getset(get = "pub", get_mut = "pub")]
  etree: ExtentTree,
}

impl Arena {
  /// Creates a new arena.
  ///
  /// # Safety
  ///
  /// The caller must ensure that the returned arena is properly managed and
  /// dropped before any referenced memory becomes invalid.
  pub unsafe fn new(index: ArenaId, chunk_size: usize) -> ArenaResult<NonNull<Self>> {
    let mut bump = Bump::new(chunk_size);
    let this_uninit = bump.create::<Self>().map_err(ArenaError::BumpError)? as *mut Self;

    unsafe { core::ptr::addr_of_mut!((*this_uninit).index).write(index) };
    unsafe { core::ptr::addr_of_mut!((*this_uninit).bump).write(bump) };

    let bins = core::array::from_fn(|i| {
      let class = ScIdx(i);
      Bin::new(class)
    });
    unsafe { core::ptr::addr_of_mut!((*this_uninit).bins).write(bins) };

    let etree = ExtentTree::new(chunk_size);
    unsafe { core::ptr::addr_of_mut!((*this_uninit).etree).write(etree) };

    Ok(unsafe { NonNull::new_unchecked(this_uninit) })
  }

  pub fn allocate(&mut self, sc: ScIdx) -> ArenaResult<NonNull<u8>> {
    let self_nn = unsafe { NonNull::new_unchecked(self as *mut Arena) };
    let bin = &mut self.bins[sc.0];
    bin
      .allocate(&mut self.bump, self_nn)
      .map_err(ArenaError::BinError)
  }

  pub fn allocate_large(&mut self, layout: Layout) -> ArenaResult<NonNull<u8>> {
    let extent_store = self
      .bump
      .create::<Extent>()
      .map_err(ArenaError::BumpError)? as *mut Extent;

    let pga_size = page_align(layout.size()).map_err(ArenaError::PrimError)?;

    let extent = Extent::new(pga_size, SysOption::Commit).map_err(ArenaError::ExtentError)?;
    let ptr = extent.as_ref().as_ptr() as *mut u8;
    unsafe {
      core::ptr::write(extent_store, extent);
    }

    let extent_nn = unsafe { NonNull::new_unchecked(extent_store) };
    let info = OwnerInfo::new_extent(extent_nn);
    self
      .etree_mut()
      .register(extent_nn, info)
      .map_err(ArenaError::LookupError)?;

    ARENA_MAP
      .associate(extent_nn, self.index())
      .map_err(ArenaError::LookupError)?;

    Ok(unsafe { NonNull::new_unchecked(ptr) })
  }

  pub fn deallocate_large(&mut self, extent: NonNull<Extent>) -> ArenaResult<()> {
    self
      .etree_mut()
      .unregister(extent)
      .map_err(ArenaError::LookupError)?;
    ARENA_MAP.detach(extent).map_err(ArenaError::LookupError)?;

    unsafe {
      core::ptr::drop_in_place(extent.as_ptr());
    }
    Ok(())
  }

  pub fn deallocate(&mut self, ptr: NonNull<u8>) -> ArenaResult<()> {
    let info = self
      .etree()
      .lookup(ptr.as_ptr() as usize)
      .ok_or(ArenaError::LookupError(LookupError::NotFound))?
      .clone();

    match info {
      OwnerInfo::Slab { slab, size_class } => {
        let slab_ref = unsafe { slab.as_ref() };
        let was_empty_before = slab_ref.is_empty();

        let bin = &mut self.bins[size_class.0];
        bin.deallocate(ptr, slab).map_err(ArenaError::BinError)?;

        if !was_empty_before && slab_ref.is_empty() {
          let extent_nn =
            unsafe { NonNull::new_unchecked(slab_ref.extent() as *const _ as *mut _) };
          let _ = self.etree_mut().unregister(extent_nn);
        }

        Ok(())
      }
      OwnerInfo::Extent { extent } => self.deallocate_large(extent),
    }
  }

  pub fn owns(&self, ptr: NonNull<u8>) -> bool {
    self.etree().lookup(ptr.as_ptr() as usize).is_some()
  }
}

#[cfg(test)]
mod tests {
  use core::ptr::drop_in_place;

  use crate::CHUNK_SIZE;

  use super::*;

  #[test]
  fn test_arena_creation() {
    let arena = unsafe { Arena::new(ArenaId(0), CHUNK_SIZE).expect("Failed to create arena") };
    unsafe { drop_in_place(arena.as_ptr()) };
  }
}
