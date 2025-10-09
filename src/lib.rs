#![no_std]
pub mod prelude {
  pub use basealloc_sys::prelude::*;
  pub use basealloc_list::{HasLink, Link, List, ListIter, ListDrainer};
}
