use std::sync::OnceLock;

use basealloc_sys::prelude::*;

use crate::config::{
  NSCLASSES,
  QUANTUM,
};

const fn log2c(mut x: usize) -> usize {
  let mut log = 0;
  x -= 1;
  while x > 0 {
    x >>= 1;
    log += 1;
  }
  log
}

#[derive(Clone)]
pub struct SizeClass {
  size: usize,
  pages: usize,
}

#[derive(Clone)]
pub struct SizeClassIndex(usize);

static CLASSES: OnceLock<[SizeClass; NSCLASSES]> = OnceLock::new();

fn generate_classes() -> [SizeClass; NSCLASSES] {
  todo!()
}

fn ensure_classes() -> &'static [SizeClass; NSCLASSES] {
  CLASSES.get_or_init(|| generate_classes())
}

pub fn class_for(size: usize) -> Option<SizeClassIndex> {
  todo!()
}
