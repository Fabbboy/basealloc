use basealloc_sys::prim::word_width;

pub mod arena;
pub mod bin;
pub mod classes;
pub mod static_;

const WORD: usize = word_width();
const WORD_TRAILING: usize = WORD.trailing_zeros() as usize;
const MAX_ARENAS: usize = 256;
