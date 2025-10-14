use core::ptr::NonNull;

use basealloc_fixed::bump::{
  Bump,
  BumpError,
};
use spin::Mutex;

#[derive(Debug)]
pub enum ArenaError {
  Bump(BumpError),
}

pub struct Arena {
  _bump: Bump,
  _lock: Mutex<()>,
}

impl Arena {
  pub unsafe fn new(chunk_size: usize) -> Result<NonNull<Self>, ArenaError> {
    let mut bump = Bump::new(chunk_size);
    let this = bump.create::<Self>().map_err(ArenaError::Bump)?;
    let tmp = Self {
      _bump: bump,
      _lock: Mutex::new(()),
    };
    unsafe { this.write(tmp) };
    Ok(unsafe { NonNull::new_unchecked(this) })
  }
}

#[cfg(test)]
mod tests {
  use core::ptr::drop_in_place;

  use super::*;
  use crate::config::CHUNK_SIZE;

  #[test]
  fn test_arena_creation() {
    let arena = unsafe { Arena::new(CHUNK_SIZE).expect("Failed to create arena") };
    unsafe { drop_in_place(arena.as_ptr()) };
  }
}
