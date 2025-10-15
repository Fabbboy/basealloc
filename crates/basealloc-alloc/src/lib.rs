pub mod arena;
pub mod bin;
pub mod classes;
pub mod static_;

mod config {
  use basealloc_bitmap::BitmapWord;
  use basealloc_sys::prim::word_width;

  pub const MAX_ARENAS: usize = 256;
  pub const ARENA_BMS: usize = core::mem::size_of::<BitmapWord>() * MAX_ARENAS;

  const BITS_PER_LEVEL: usize = 9;
  pub const FANOUT: usize = 1 << BITS_PER_LEVEL;

  pub const WORD: usize = word_width();
  pub const WORD_TRAILING: usize = WORD.trailing_zeros() as usize;

  pub const CHUNK_SHIFT: usize = 16 + WORD_TRAILING;
  pub const CHUNK_SIZE: usize = 1 << CHUNK_SHIFT;

  pub const NSCLASSES: usize = 128;
  pub const QUANTUM: usize = WORD * 2;
}
