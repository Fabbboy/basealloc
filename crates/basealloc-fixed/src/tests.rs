use core::alloc::Layout;

use basealloc_sys::math::align_up;

use crate::{
  bump::Bump,
  fixed::{
    Fixed,
    FixedError,
  },
};

#[test]
fn fixed_allocation_alignment() {
  let mut storage = [0u8; 128];
  let mut fixed = Fixed::new(&storage);
  let layout = Layout::from_size_align(24, 16).unwrap();
  let expected_len = align_up(layout.size(), layout.align()).unwrap();

  let (first_addr, first_len) = {
    let first = fixed.allocate(&mut storage, layout).unwrap();
    let addr = first.as_ptr() as usize;
    let len = first.len();
    assert_eq!(addr % layout.align(), 0);
    assert_eq!(len, expected_len);
    (addr, len)
  };

  let second_layout = Layout::from_size_align(8, 8).unwrap();
  let expected_second_len = align_up(second_layout.size(), second_layout.align()).unwrap();
  let second = fixed.allocate(&mut storage, second_layout).unwrap();

  assert_eq!(second.as_ptr() as usize % second_layout.align(), 0);
  assert_eq!(second.len(), expected_second_len);
  assert!(second.as_ptr() as usize >= first_addr + first_len);
}

#[test]
fn fixed_reports_oom() {
  let mut storage = [0u8; 64];
  let mut fixed = Fixed::new(&storage);
  let layout = Layout::from_size_align(32, 1).unwrap();

  {
    let _ = fixed.allocate(&mut storage, layout).unwrap();
  }
  {
    let _ = fixed.allocate(&mut storage, layout).unwrap();
  }

  let err = fixed
    .allocate(&mut storage, Layout::from_size_align(1, 1).unwrap())
    .unwrap_err();
  assert!(matches!(err, FixedError::OOM));
}

#[test]
fn bump_allocation_alignment() {
  let mut bump = Bump::new(0);
  let layout = Layout::from_size_align(48, 16).unwrap();
  let expected_len = align_up(layout.size(), layout.align()).unwrap();

  let slice = bump.allocate(layout).unwrap();
  assert_eq!(slice.as_ptr() as usize % layout.align(), 0);
  assert_eq!(slice.len(), expected_len);
}

#[test]
fn bump_grows_chunks() {
  let mut bump = Bump::new(0);
  let layout = Layout::from_size_align(512, 8).unwrap();

  let first_addr = {
    let slice = bump.allocate(layout).unwrap();
    slice.as_ptr() as usize
  };

  let second_addr = {
    let slice = bump.allocate(layout).unwrap();
    slice.as_ptr() as usize
  };

  assert_ne!(first_addr, second_addr);
}

#[test]
fn bump_create_zeroed() {
  #[repr(C)]
  struct Sample {
    a: u32,
    b: u32,
  }

  let mut bump = Bump::new(0);
  let ptr = bump.create::<Sample>().unwrap();

  unsafe {
    let sample = &mut *ptr;
    assert_eq!(sample.a, 0);
    assert_eq!(sample.b, 0);
    sample.a = 1;
    sample.b = 2;
  }
}
