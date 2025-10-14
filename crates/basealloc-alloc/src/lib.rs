#![no_std]

pub mod arena;
pub mod static_;

mod config {
  use basealloc_sys::prim::word_width;

  pub const MAX_ARENAS: usize = 256;
  const BITS_PER_LEVEL: usize = 9;
  pub const FANOUT: usize = 1 << BITS_PER_LEVEL;

  pub const WORD: usize = word_width();
  pub const WORD_TRAILING: usize = WORD.trailing_zeros() as usize;

  pub const CHUNK_SHIFT: usize = 16 + WORD_TRAILING;
  pub const CHUNK_SIZE: usize = 1 << CHUNK_SHIFT;
}
