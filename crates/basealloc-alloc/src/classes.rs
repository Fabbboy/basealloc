use std::sync::OnceLock;

use basealloc_sys::prelude::*;

use crate::{
  WORD,
  WORD_BITS,
};

pub const QUANTUM: usize = WORD * 2; // minimal size class
pub const NGROUPSEX: usize = 2; // exponent for classes per group
pub const NGROUPS: usize = 1 << NGROUPSEX; // classes per group

pub const NTINY: usize = NGROUPS * QUANTUM; // max size for tiny classes

pub const MAX_NREGULAR: usize = WORD_BITS - 2; // max regular size classes
pub const FIRST_REGULAR: usize = QUANTUM + NGROUPS;
pub const NREGULAR: usize = (MAX_NREGULAR - FIRST_REGULAR + 1) - 1;

pub const NSCLASSES: usize = NTINY + NREGULAR;

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

static CLASSES: OnceLock<[SizeClass; NSCLASSES]> = OnceLock::new();
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

  let classes = ensure_classes();
  for (i, class) in classes.iter().enumerate() {
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

fn ensure_classes() -> &'static [SizeClass; NSCLASSES] {
  CLASSES.get_or_init(|| generate_classes())
}

fn generate_classes() -> [SizeClass; NSCLASSES] {
  todo!()
}

pub fn class_for(size: usize) -> Option<SizeClassIndex> {
  todo!()
}
