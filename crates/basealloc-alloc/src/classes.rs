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

static CLASSES: [SizeClass; NSCLASSES] = generate_classes();

const fn generate_classes() -> [SizeClass; NSCLASSES] {
  todo!()
}

pub fn slab_size(class: SizeClassIndex) -> usize {
  page_size() * CLASSES[class.0].pages
}

pub fn num_regions(class: SizeClassIndex) -> usize {
  slab_size(class.clone()) / CLASSES[class.0].size
}

pub fn class_for(size: usize) -> Option<SizeClassIndex> {
  todo!()
}
