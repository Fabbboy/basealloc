use core::sync::atomic::AtomicPtr;

use heapless::Vec;
use spin::Mutex;

use crate::{
  bump::{
    Bump,
    BumpError,
    GLOBAL_BUMP,
  },
  config::MAX_ARENAS,
};

pub static ARENAS: Vec<AtomicPtr<Arena>, { MAX_ARENAS }> = Vec::new();

#[derive(Debug)]
pub enum ArenaError {
  Bump(BumpError),
}

pub struct Arena {
  _bump: Bump,
  _lock: Mutex<()>,
}

impl Arena {
  const SELF_SIZED: usize = core::mem::size_of::<Self>();
  const SELF_ALIGNED: usize = core::mem::align_of::<Self>();

  pub unsafe fn new(chunk_size: usize) -> Result<Self, ArenaError> {
    let mut gbb = GLOBAL_BUMP.lock();
    let this = gbb
      .allocate(Self::SELF_SIZED, Self::SELF_ALIGNED)
      .map_err(ArenaError::Bump)?;

    let ptr = this.as_mut_ptr() as *mut Self;

    let bump = Bump::new(chunk_size);
    let tmp = Self {
      _bump: bump,
      _lock: Mutex::new(()),
    };
    unsafe {
      ptr.write(tmp);
    }
    Ok(unsafe { ptr.read() })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::config::CHUNK_SIZE;

  #[test]
  fn test_arena_creation() {
    let _ = unsafe { Arena::new(CHUNK_SIZE).expect("Failed to create arena") };
  }
}
