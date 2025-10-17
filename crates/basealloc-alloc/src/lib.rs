use basealloc_sys::prim::word_width;

pub mod arena;
pub mod bin;
pub mod classes;
pub mod static_;
pub mod tcache;
pub mod slab;

const WORD: usize = word_width();
const BITS_PER_BYTE: usize = 8;
const WORD_BITS: usize = WORD * BITS_PER_BYTE;
const WORD_TRAILING: usize = WORD.trailing_zeros() as usize;
const MAX_ARENAS: usize = 256;
