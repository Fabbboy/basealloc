use std::sync::OnceLock;

use basealloc_sys::{
  prelude::*,
  prim::{
    likely,
    unlikely,
  },
};

use crate::{
  WORD,
  WORD_BITS,
  WORD_TRAILING,
};

pub const QUANTUM: usize = WORD * 2;
pub const NGROUPSEX: usize = 2;
pub const NGROUPS: usize = 1 << NGROUPSEX;

pub const NTINY: usize = NGROUPS * QUANTUM;
pub const TINY_CUTOFF: usize = NTINY * QUANTUM;

const FIRST_REGULAR: usize = log2c(TINY_CUTOFF);
const MAX_REGULAR: usize = 2 * WORD_BITS / 6;
pub const NREGULAR: usize = (MAX_REGULAR - FIRST_REGULAR) * NGROUPS;

pub const NSCLASSES: usize = NTINY + NREGULAR;
pub const SCLASS_CUTOFF: usize = 1 << MAX_REGULAR;

const LOOKUP_SHIFT: usize = WORD_TRAILING + 1;

// Size class structure:
// - QUANTUM (16): minimum allocation unit
// - Tiny classes [0..NTINY): linear spacing by QUANTUM up to TINY_CUTOFF (1024)
//   NTINY = 64 classes = NGROUPS * QUANTUM
// - Regular classes [NTINY..NSCLASSES): exponential spacing
//   NGROUPS (4) classes per power-of-2 group from 2^FIRST_REGULAR to 2^MAX_REGULAR
//   FIRST_REGULAR = log2(TINY_CUTOFF) = 10, MAX_REGULAR = 2*WORD_BITS/6 ≈ 21
//   NREGULAR = (MAX_REGULAR - FIRST_REGULAR) * NGROUPS ≈ 44 classes
// - SCLASS_CUTOFF = 2^MAX_REGULAR ≈ 2MB: maximum size handled by size classes

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SizeClass(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SizeClassIndex(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageSize(usize);

static CLASSES: [SizeClass; NSCLASSES] = generate_classes();
static TINY_LOOKUP: [u8; TINY_CUTOFF >> LOOKUP_SHIFT] = generate_tiny_lookup();
static PAGES: OnceLock<[PageSize; NSCLASSES]> = OnceLock::new();

const fn log2c(mut x: usize) -> usize {
  let mut log = 0;
  x -= 1;
  while x > 0 {
    x >>= 1;
    log += 1;
  }
  log
}

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
  let mut classes = [const { SizeClass(0) }; NSCLASSES];
  let mut idx = 0;

  while idx < NTINY {
    classes[idx] = SizeClass((idx + 1) * QUANTUM);
    idx += 1;
  }

  let mut group = FIRST_REGULAR;
  while group < MAX_REGULAR && idx < NSCLASSES {
    let base = 1 << group;
    let delta = base >> NGROUPSEX;

    let mut i = 0;
    while i < NGROUPS && idx < NSCLASSES {
      classes[idx] = SizeClass(base + delta * (i + 1));
      idx += 1;
      i += 1;
    }
    group += 1;
  }

  classes
}

const fn generate_tiny_lookup() -> [u8; TINY_CUTOFF >> LOOKUP_SHIFT] {
  let mut table = [0u8; TINY_CUTOFF >> LOOKUP_SHIFT];
  let mut i = 0;
  while i < table.len() {
    let size = (i + 1) << LOOKUP_SHIFT;
    let idx = size.div_ceil(QUANTUM) - 1;
    table[i] = idx as u8;
    i += 1;
  }
  table
}

#[inline]
fn class_for_regular(size: usize) -> SizeClassIndex {
  let log = (usize::BITS - size.leading_zeros()) as usize - 1;
  let group_idx = log - FIRST_REGULAR;
  let base = 1 << log;
  let delta = base >> NGROUPSEX;
  let offset = if unlikely(size > base) {
    (size - base - 1) / delta
  } else {
    0
  };

  SizeClassIndex(NTINY + group_idx * NGROUPS + offset)
}

#[inline(always)]
pub fn class_for(size: usize) -> Option<SizeClassIndex> {
  if unlikely(size == 0 || size > SCLASS_CUTOFF) {
    return None;
  }

  if likely(size <= TINY_CUTOFF) {
    let idx = TINY_LOOKUP[(size - 1) >> LOOKUP_SHIFT] as usize;
    return Some(SizeClassIndex(idx));
  }

  Some(class_for_regular(size))
}

#[inline]
pub fn pages_for(class: SizeClassIndex) -> PageSize {
  let pages = ensure_pages();
  pages[class.0]
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn constants_are_valid() {
    assert_eq!(QUANTUM, 16);
    assert_eq!(NGROUPS, 4);
    assert_eq!(NTINY, 64);
    assert_eq!(TINY_CUTOFF, 1024);
    assert_eq!(FIRST_REGULAR, 10);
    assert_eq!(SCLASS_CUTOFF, 2097152);
    assert!(NSCLASSES > 0 && NSCLASSES < 256);
  }

  #[test]
  fn classes_are_monotonic() {
    for i in 1..NSCLASSES {
      let SizeClass(prev) = CLASSES[i - 1];
      let SizeClass(curr) = CLASSES[i];
      assert!(
        curr > prev,
        "class[{}]={} not > class[{}]={}",
        i,
        curr,
        i - 1,
        prev
      );
    }
  }

  #[test]
  fn tiny_classes_correct() {
    for i in 0..NTINY {
      let SizeClass(size) = CLASSES[i];
      assert_eq!(size, (i + 1) * QUANTUM);
    }
  }

  #[test]
  fn class_for_boundary_cases() {
    assert_eq!(class_for(0), None);
    assert_eq!(class_for(SCLASS_CUTOFF + 1), None);

    let SizeClassIndex(idx) = class_for(1).unwrap();
    assert_eq!(idx, 0);

    let SizeClassIndex(idx) = class_for(QUANTUM).unwrap();
    assert_eq!(idx, 0);

    let SizeClassIndex(idx) = class_for(QUANTUM + 1).unwrap();
    assert_eq!(idx, 1);
  }

  #[test]
  fn class_for_all_sizes_valid() {
    for idx in 0..NSCLASSES {
      let SizeClass(size) = CLASSES[idx];
      let result = class_for(size);
      assert!(
        result.is_some(),
        "size {} (class {}) should have a class",
        size,
        idx
      );

      if idx > 0 {
        let SizeClass(prev_size) = CLASSES[idx - 1];
        let SizeClassIndex(found_idx) = class_for(prev_size + 1).unwrap();
        assert_eq!(
          found_idx,
          idx,
          "size {} should map to class {}",
          prev_size + 1,
          idx
        );
      }
    }
  }

  #[test]
  fn regular_classes_exponential() {
    for i in NTINY + 1..NSCLASSES {
      let SizeClass(size) = CLASSES[i];
      assert!(size > TINY_CUTOFF);
    }
  }

  #[test]
  fn page_sizes_are_multiples() {
    let pages = ensure_pages();
    let ps = page_size();

    for (i, page) in pages.iter().enumerate() {
      let PageSize(page_bytes) = *page;
      assert!(page_bytes > 0, "page size for class {} is zero", i);
      assert_eq!(
        page_bytes % ps,
        0,
        "page size {} not multiple of page_size {}",
        page_bytes,
        ps
      );
    }
  }

  #[test]
  fn page_sizes_minimize_waste() {
    let pages = ensure_pages();

    for (i, page) in pages.iter().enumerate() {
      let SizeClass(class_size) = CLASSES[i];
      let PageSize(page_bytes) = *page;

      let objects_per_page = page_bytes / class_size;
      assert!(
        objects_per_page > 0,
        "class {} size {} too large for page {}",
        i,
        class_size,
        page_bytes
      );

      let waste = page_bytes - (objects_per_page * class_size);
      assert!(
        waste < class_size,
        "waste {} >= class_size {} for class {}",
        waste,
        class_size,
        i
      );
    }
  }
}
