use core::{
  alloc::Layout,
  ptr::NonNull,
};

use basealloc_fixed::bump::{
  Bump,
  BumpError,
};
use getset::CloneGetters;
use spin::Mutex;

use crate::{
  bin::{
    Bin,
    BinError,
  },
  classes::{
    NSCLASSES,
    SizeClassIndex,
    class_for,
  },
};

#[derive(Debug)]
pub enum ArenaError {
  BumpError(BumpError),
  BinError(BinError),
}

pub type ArenaResult<T> = Result<T, ArenaError>;

#[derive(CloneGetters)]
pub struct Arena {
  #[getset(get_clone = "pub")]
  index: usize,
  bins: [Bin; NSCLASSES],
  bump: Bump,
  lock: Mutex<()>,
}

impl Arena {
  pub unsafe fn new(index: usize, chunk_size: usize) -> ArenaResult<NonNull<Self>> {
    let mut bump = Bump::new(chunk_size);
    let this_uninit = bump.create::<Self>().map_err(ArenaError::BumpError)?;

    unsafe { core::ptr::addr_of_mut!((*this_uninit).index).write(index) };
    unsafe { core::ptr::addr_of_mut!((*this_uninit).bump).write(bump) };
    unsafe { core::ptr::addr_of_mut!((*this_uninit).lock).write(Mutex::new(())) };

    let bump = unsafe { &mut *core::ptr::addr_of_mut!((*this_uninit).bump) };
    let bins = core::array::from_fn(|i| {
      let class = SizeClassIndex(i);
      Bin::new(bump, class)
    });
    unsafe { core::ptr::addr_of_mut!((*this_uninit).bins).write(bins) };

    Ok(unsafe { NonNull::new_unchecked(this_uninit) })
  }

  fn allocate_large(&self, layout: Layout) -> ArenaResult<NonNull<u8>> {
    _ = layout;
    todo!()
  }

  pub fn allocate(&mut self, layout: Layout) -> ArenaResult<NonNull<u8>> {
    let _guard = self.lock.lock();
    let class = class_for(layout.size());
    if let None = class {
      return self.allocate_large(layout);
    }

    let class = class.unwrap();
    let bin = &mut self.bins[class.0];
    bin.allocate(layout).map_err(ArenaError::BinError)
  }

  fn deallocate_large(&mut self, ptr: NonNull<u8>, layout: Layout) {
    _ = ptr;
    _ = layout;
    todo!()
  }

  pub fn deallocate(&mut self, ptr: NonNull<u8>, layout: Layout) {
    _ = ptr;
    _ = layout;
    todo!()
  }

  pub fn sizeof(&self, ptr: NonNull<u8>) -> Option<usize> {
    _ = ptr;
    None
  }
}

#[cfg(test)]
mod tests {
  use core::ptr::drop_in_place;

  use crate::static_::CHUNK_SIZE;

  use super::*;

  #[test]
  fn test_arena_creation() {
    let arena = unsafe { Arena::new(0, CHUNK_SIZE).expect("Failed to create arena") };
    unsafe { drop_in_place(arena.as_ptr()) };
  }
}
