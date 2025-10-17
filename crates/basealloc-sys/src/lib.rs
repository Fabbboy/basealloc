#![cfg_attr(not(test), no_std)]

pub mod math;
pub mod prim;
pub mod system;
pub mod unix;

pub trait Giveup {
  type Failure: Default;

  fn giveup(self) -> Result<Self, Self::Failure>
  where
    Self: Sized;
}

pub use system::GLOBAL_SYSTEM;

pub mod prelude {
  pub use super::{
    GLOBAL_SYSTEM,
    math::{
      align_down,
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
      page_align_down,
      page_size,
      va_size,
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
