#![no_std]

pub mod math;
pub mod prim;
pub mod system;
pub mod unix;

pub use system::GLOBAL_SYSTEM;

pub mod prelude {
  pub use super::{
    GLOBAL_SYSTEM,
    math::{
      align_mut_ptr,
      align_offset,
      align_ptr,
      align_up,
      is_aligned,
    },
    prim::{
      is_page_aligned,
      min_align,
      page_align,
      word_width,
    },
    system::{
      SysError,
      SysOption,
      SysResult,
      System,
    },
  };
}
