use core::{
  mem::MaybeUninit,
  ptr::NonNull,
};

use basealloc_fixed::bump::{
  Bump,
  BumpError,
};
use getset::CloneGetters;
use spin::Mutex;

use crate::{
  bin::Bin,
  classes::NSCLASSES,
};

#[derive(Debug)]
pub enum ArenaError {
  Bump(BumpError),
}

pub type ArenaResult<T> = Result<T, ArenaError>;

#[derive(CloneGetters)]
pub struct Arena {
  #[getset(get_clone = "pub")]
  index: usize,
  bins: [Bin; NSCLASSES],
  bump: Bump,
  _lock: Mutex<()>,
}

impl Arena {
  pub unsafe fn new(index: usize, chunk_size: usize) -> ArenaResult<NonNull<Self>> {
    let mut bump = Bump::new(chunk_size);
    let this_uninit = bump
      .create::<Self>()
      .map_err(ArenaError::Bump)?;

    unsafe { core::ptr::addr_of_mut!((*this_uninit).index).write(index) };
    unsafe { core::ptr::addr_of_mut!((*this_uninit).bump).write(bump) };
    unsafe { core::ptr::addr_of_mut!((*this_uninit)._lock).write(Mutex::new(())) };

    let bins = core::array::from_fn(|_| Bin::new(unsafe { &mut (*this_uninit).bump }));
    unsafe { core::ptr::addr_of_mut!((*this_uninit).bins).write(bins) };

    Ok(unsafe { NonNull::new_unchecked(this_uninit) })
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
