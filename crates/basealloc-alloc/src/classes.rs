use std::sync::OnceLock;

use basealloc_sys::prelude::*;

use crate::WORD;

pub const NSCLASSES: usize = 128;
pub const QUANTUM: usize = WORD * 2;
pub const QUANTUM_SHIFT: usize = log2c(QUANTUM);

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

#[derive(Clone)]
pub struct PageSize(usize);

static CLASSES: [SizeClass; NSCLASSES] = generate_classes();
static PAGES: OnceLock<[PageSize; NSCLASSES]> = OnceLock::new();

fn gcd(mut a: usize, mut b: usize) -> usize {
  while b != 0 {
    let t = b;
    b = a % b;
    a = t;
  }
  a
}

fn generate_pages() -> [PageSize; NSCLASSES] {
  let mut pages = [const { PageSize(0) }; NSCLASSES];
  let ps = page_size();

  for (i, class) in CLASSES.iter().enumerate() {
    let SizeClass(size) = *class;
    let g = gcd(ps, size);
    let num_pages = size / g;
    pages[i] = PageSize(num_pages * ps);
  }
  pages
}

fn ensure_pages() -> &'static [PageSize; NSCLASSES] {
  PAGES.get_or_init(|| generate_pages())
}

const fn generate_classes() -> [SizeClass; NSCLASSES] {
  todo!()
}

pub fn class_for(size: usize) -> Option<SizeClassIndex> {
  todo!()
}
