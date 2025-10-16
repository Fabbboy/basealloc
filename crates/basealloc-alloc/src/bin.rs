use std::ptr::NonNull;

use basealloc_fixed::bump::Bump;

pub struct Bin { 
  // SAFETY: User must ensure bin is dropped before bump.
  bump: NonNull<Bump>,
}

impl Bin {
  pub fn new(bump: &mut Bump) -> Self {
    Self {
      bump: NonNull::from(bump),
    }
  }
}