#![cfg_attr(not(test), no_std)]
use basealloc_bitmap::BitmapWord;
use basealloc_sys::prim::word_width;

pub mod arena;
pub mod bin;
pub mod classes;
pub mod lookup;
pub mod slab;
pub mod static_;
pub mod tcache;

const WORD: usize = word_width();
const BITS_PER_BYTE: usize = 8;

const WORD_BITS: usize = WORD * BITS_PER_BYTE;
const WORD_TRAILING: usize = WORD.trailing_zeros() as usize;

const MAX_ARENAS: usize = 256;
pub const ARENA_BMS: usize = core::mem::size_of::<BitmapWord>() * MAX_ARENAS;

pub const CHUNK_SHIFT: usize = 16 + WORD_TRAILING;
pub const CHUNK_SIZE: usize = 1 << CHUNK_SHIFT;

const BITS_PER_LEVEL: usize = 9;
pub const FANOUT: usize = 1 << BITS_PER_LEVEL;
