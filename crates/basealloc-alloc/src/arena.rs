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
use getset::CloneGetters;

use crate::{
  bin::{
    Bin,
    BinError,
  },
  classes::{
    NSCLASSES,
    SizeClassIndex,
  },
  slab::Slab,
  static_::{
    LookupError,
    register_large,
    unregister_range,
  },
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

#[derive(CloneGetters)]
pub struct Arena {
  #[getset(get_clone = "pub")]
  index: usize,
  bins: [Bin; NSCLASSES],
  bump: Bump,
}

impl Arena {
  pub unsafe fn new(index: usize, chunk_size: usize) -> ArenaResult<NonNull<Self>> {
    let mut bump = Bump::new(chunk_size);
    let this_uninit = bump.create::<Self>().map_err(ArenaError::BumpError)? as *mut Self;

    unsafe { core::ptr::addr_of_mut!((*this_uninit).index).write(index) };
    unsafe { core::ptr::addr_of_mut!((*this_uninit).bump).write(bump) };

    let bins = core::array::from_fn(|i| {
      let class = SizeClassIndex(i);
      Bin::new(class)
    });
    unsafe { core::ptr::addr_of_mut!((*this_uninit).bins).write(bins) };

    Ok(unsafe { NonNull::new_unchecked(this_uninit) })
  }

  pub fn allocate(&mut self, sc: SizeClassIndex) -> ArenaResult<NonNull<u8>> {
    let self_nn = unsafe { NonNull::new_unchecked(self as *mut Arena) };
    let bin = &mut self.bins[sc.0];
    bin
      .allocate(&mut self.bump, self_nn)
      .map_err(ArenaError::BinError)
  }

  pub fn allocate_many<const N: usize>(
    //TODO: rollback on failure
    &mut self,
    sc: SizeClassIndex,
    out: &mut [NonNull<u8>; N],
  ) -> ArenaResult<()> {
    for slot in out.iter_mut() {
      let ptr = self.allocate(sc)?;
      *slot = ptr;
    }
    Ok(())
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
    register_large(extent_nn).map_err(ArenaError::LookupError)?;

    Ok(unsafe { NonNull::new_unchecked(ptr) })
  }

  pub fn deallocate_large(&mut self, mut extent: NonNull<Extent>) -> ArenaResult<()> {
    unregister_range(extent).map_err(ArenaError::LookupError)?;
    let extent = unsafe { extent.as_mut() };
    let owning = unsafe { core::ptr::read(extent) };
    let _ = owning.giveup();
    Ok(())
  }

  pub fn deallocate(
    &mut self,
    ptr: NonNull<u8>,
    sc: SizeClassIndex,
    slab: NonNull<Slab>,
  ) -> ArenaResult<()> {
    let bin = &mut self.bins[sc.0];
    bin.deallocate(ptr, slab).map_err(ArenaError::BinError)
  }
}

#[cfg(test)]
mod tests {
  use core::ptr::drop_in_place;

  use crate::CHUNK_SIZE;

  use super::*;

  #[test]
  fn test_arena_creation() {
    let arena = unsafe { Arena::new(0, CHUNK_SIZE).expect("Failed to create arena") };
    unsafe { drop_in_place(arena.as_ptr()) };
  }
}
