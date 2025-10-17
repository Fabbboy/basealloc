use super::*;

#[test]
fn new_creates_empty_ring() {
  let ring = Ring::new();
  let buf = [0u32; 4];
  assert_eq!(ring.len(), 0);
  assert!(ring.is_empty());
  assert!(!ring.is_full(&buf));
}

#[test]
fn push_pop_single_element() {
  let ring = Ring::new();
  let mut buf = [0u32; 4];

  assert!(ring.push(&mut buf, 42).is_ok());
  assert_eq!(ring.len(), 1);
  assert!(!ring.is_empty());

  assert_eq!(ring.pop(&buf), Some(&42));
  assert_eq!(ring.len(), 0);
  assert!(ring.is_empty());
}

#[test]
fn push_pop_multiple_elements() {
  let ring = Ring::new();
  let mut buf = [0u32; 4];

  for i in 1..=3 {
    assert!(ring.push(&mut buf, i * 10).is_ok());
  }

  assert_eq!(ring.len(), 3);
  assert_eq!(ring.pop(&buf), Some(&10));
  assert_eq!(ring.pop(&buf), Some(&20));
  assert_eq!(ring.pop(&buf), Some(&30));
  assert!(ring.is_empty());
}

#[test]
fn fill_to_capacity() {
  let ring = Ring::new();
  let mut buf = [0u32; 4];

  for i in 0..4 {
    assert!(ring.push(&mut buf, i).is_ok());
  }

  assert_eq!(ring.len(), 4);
  assert!(ring.is_full(&buf));
}

#[test]
fn overflow_returns_err() {
  let ring = Ring::new();
  let mut buf = [0u32; 4];

  for i in 0..4 {
    assert!(ring.push(&mut buf, i).is_ok());
  }

  let result = ring.push(&mut buf, 999);
  assert_eq!(result, Err(RingError::Full(999)));
  assert_eq!(ring.len(), 4);
}

#[test]
fn pop_empty_returns_none() {
  let ring = Ring::new();
  let buf = [0u32; 4];
  assert_eq!(ring.pop(&buf), None);
}

#[test]
fn wraparound_behavior() {
  let ring = Ring::new();
  let mut buf = [0u32; 4];

  for i in 0..4 {
    ring.push(&mut buf, i).unwrap();
  }

  assert_eq!(ring.pop(&buf), Some(&0));
  assert_eq!(ring.pop(&buf), Some(&1));

  ring.push(&mut buf, 100).unwrap();
  ring.push(&mut buf, 101).unwrap();

  assert_eq!(ring.len(), 4);
  assert_eq!(ring.pop(&buf), Some(&2));
  assert_eq!(ring.pop(&buf), Some(&3));
  assert_eq!(ring.pop(&buf), Some(&100));
  assert_eq!(ring.pop(&buf), Some(&101));
  assert!(ring.is_empty());
}

#[test]
fn interleaved_push_pop() {
  let ring = Ring::new();
  let mut buf = [0u32; 4];

  ring.push(&mut buf, 1).unwrap();
  ring.push(&mut buf, 2).unwrap();
  assert_eq!(ring.pop(&buf), Some(&1));

  ring.push(&mut buf, 3).unwrap();
  assert_eq!(ring.pop(&buf), Some(&2));
  assert_eq!(ring.pop(&buf), Some(&3));

  ring.push(&mut buf, 4).unwrap();
  ring.push(&mut buf, 5).unwrap();
  assert_eq!(ring.len(), 2);
}

#[test]
fn works_with_pointers() {
  let ring = Ring::new();
  let mut buf = [core::ptr::null_mut::<u8>(); 8];

  let ptr1 = 0x1000 as *mut u8;
  let ptr2 = 0x2000 as *mut u8;
  let ptr3 = 0x3000 as *mut u8;

  ring.push(&mut buf, ptr1).unwrap();
  ring.push(&mut buf, ptr2).unwrap();
  ring.push(&mut buf, ptr3).unwrap();

  assert_eq!(ring.pop(&buf), Some(&ptr1));
  assert_eq!(ring.pop(&buf), Some(&ptr2));
  assert_eq!(ring.pop(&buf), Some(&ptr3));
}

#[test]
fn stress_test_large_capacity() {
  let ring = Ring::new();
  let mut buf = [0usize; 128];

  for i in 0..128 {
    assert!(ring.push(&mut buf, i).is_ok());
  }

  assert!(ring.is_full(&buf));
  assert!(ring.push(&mut buf, 999).is_err());

  for i in 0..128 {
    assert_eq!(ring.pop(&buf), Some(&i));
  }

  assert!(ring.is_empty());
}
