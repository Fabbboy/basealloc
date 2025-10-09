#![no_std]

#[cfg_attr(all(feature = "panic-handler", not(test)), panic_handler)]
pub fn panic_handler(_info: &core::panic::PanicInfo) -> ! {
  loop {}
}

pub mod prelude {
  pub use basealloc_sys::prelude::*;
}
