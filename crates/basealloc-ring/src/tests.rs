use super::*;

#[test]
fn new_creates_empty_ring() {
  let ring: Ring<u32, 4> = Ring::new(0);
  assert_eq!(ring.len(), 0);
  assert!(ring.is_empty());
  assert!(!ring.is_full());
  assert_eq!(ring.capacity(), 4);
}

#[test]
fn push_pop_single_element() {
  let mut ring: Ring<u32, 4> = Ring::new(0);

  assert!(ring.push(42).is_ok());
  assert_eq!(ring.len(), 1);
  assert!(!ring.is_empty());

  assert_eq!(ring.pop(), Some(42));
  assert_eq!(ring.len(), 0);
  assert!(ring.is_empty());
}

#[test]
fn push_pop_multiple_elements() {
  let mut ring: Ring<u32, 4> = Ring::new(0);

  for i in 1..=3 {
    assert!(ring.push(i * 10).is_ok());
  }

  assert_eq!(ring.len(), 3);
  assert_eq!(ring.pop(), Some(10));
  assert_eq!(ring.pop(), Some(20));
  assert_eq!(ring.pop(), Some(30));
  assert!(ring.is_empty());
}

#[test]
fn fill_to_capacity() {
  let mut ring: Ring<u32, 4> = Ring::new(0);

  for i in 0..4 {
    assert!(ring.push(i).is_ok());
  }

  assert_eq!(ring.len(), 4);
  assert!(ring.is_full());
}

#[test]
fn overflow_returns_err() {
  let mut ring: Ring<u32, 4> = Ring::new(0);

  for i in 0..4 {
    assert!(ring.push(i).is_ok());
  }

  let result = ring.push(999);
  assert_eq!(result, Err(RingError::Full(999)));
  assert_eq!(ring.len(), 4);
}

#[test]
fn pop_empty_returns_none() {
  let mut ring: Ring<u32, 4> = Ring::new(0);
  assert_eq!(ring.pop(), None);
}

#[test]
fn wraparound_behavior() {
  let mut ring: Ring<u32, 4> = Ring::new(0);

  for i in 0..4 {
    ring.push(i).unwrap();
  }

  assert_eq!(ring.pop(), Some(0));
  assert_eq!(ring.pop(), Some(1));

  ring.push(100).unwrap();
  ring.push(101).unwrap();

  assert_eq!(ring.len(), 4);
  assert_eq!(ring.pop(), Some(2));
  assert_eq!(ring.pop(), Some(3));
  assert_eq!(ring.pop(), Some(100));
  assert_eq!(ring.pop(), Some(101));
  assert!(ring.is_empty());
}

#[test]
fn interleaved_push_pop() {
  let mut ring: Ring<u32, 4> = Ring::new(0);

  ring.push(1).unwrap();
  ring.push(2).unwrap();
  assert_eq!(ring.pop(), Some(1));

  ring.push(3).unwrap();
  assert_eq!(ring.pop(), Some(2));
  assert_eq!(ring.pop(), Some(3));

  ring.push(4).unwrap();
  ring.push(5).unwrap();
  assert_eq!(ring.len(), 2);
}

#[test]
fn works_with_pointers() {
  let mut ring: Ring<*mut u8, 8> = Ring::new(core::ptr::null_mut());

  let ptr1 = 0x1000 as *mut u8;
  let ptr2 = 0x2000 as *mut u8;
  let ptr3 = 0x3000 as *mut u8;

  ring.push(ptr1).unwrap();
  ring.push(ptr2).unwrap();
  ring.push(ptr3).unwrap();

  assert_eq!(ring.pop(), Some(ptr1));
  assert_eq!(ring.pop(), Some(ptr2));
  assert_eq!(ring.pop(), Some(ptr3));
}

#[test]
fn stress_test_large_capacity() {
  let mut ring: Ring<usize, 128> = Ring::new(0);

  for i in 0..128 {
    assert!(ring.push(i).is_ok());
  }

  assert!(ring.is_full());
  assert!(ring.push(999).is_err());

  for i in 0..128 {
    assert_eq!(ring.pop(), Some(i));
  }

  assert!(ring.is_empty());
}
