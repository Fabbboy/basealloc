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
pub struct SizeClass(usize);

#[derive(Clone)]
pub struct SizeClassIndex(usize);

static CLASSES: [SizeClass; NSCLASSES] = generate_classes();

const fn generate_classes() -> [SizeClass; NSCLASSES] {
  todo!()
}

fn gcd(mut a: usize, mut b: usize) -> usize {
  while b != 0 {
    let t = b;
    b = a % b;
    a = t;
  }
  a
}
pub fn num_pages(class: SizeClassIndex) -> usize {
  let ps = page_size();
  let reg_size = CLASSES[class.0].0;
  let g = gcd(reg_size, ps);
  reg_size / g
}

pub fn slab_size(class: SizeClassIndex) -> usize {
  let ps = page_size();
  let np = num_pages(class.clone());
  ps * np
}

pub fn num_regions(class: SizeClassIndex) -> usize {
  slab_size(class.clone()) / CLASSES[class.0].0
}

pub fn class_for(size: usize) -> Option<SizeClassIndex> {
  todo!()
}
