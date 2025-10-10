#![no_std]

pub mod bump;

mod config {
  use basealloc_sys::prim::word_width;

  pub const WORD: usize = word_width();
  pub const WORD_TRAILING: usize = WORD.trailing_zeros() as usize;

  pub const CHUNK_SHIFT: usize = 16 + WORD_TRAILING;
  pub const CHUNK_SIZE: usize = 1 << CHUNK_SHIFT;
}
