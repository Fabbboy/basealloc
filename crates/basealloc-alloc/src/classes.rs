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
pub const NREGULAR: usize = (MAX_NREGULAR - FIRST_REGULAR + 1) - 1 ;

pub const NSCLASSES: usize = NTINY + NREGULAR;
pub const SCLASS_CUTOFF: usize = ?  // the size at which we no longer provision size classes computed from the above

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
  let mut arr = [const { SizeClass(0) }; NSCLASSES];

  let mut i = 0;
  while i < NTINY {
    arr[i] = SizeClass((i + 1) * QUANTUM);
    i += 1;
  }

  let mut j = 0;
  let mut idx = i;
  while j < NREGULAR {
    let size = 1 << (FIRST_REGULAR + j);
    arr[idx] = SizeClass(size);
    idx += 1;
    j += 1;
  }

  arr
}

pub fn class_for(size: usize) -> Option<SizeClassIndex> {
  todo!()
}
