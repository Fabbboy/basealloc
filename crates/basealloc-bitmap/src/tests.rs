use super::*;
use core::sync::atomic::AtomicUsize;

#[test]
fn test_multi_word_operations() {
  let storage: [AtomicUsize; 2] = [AtomicUsize::new(0), AtomicUsize::new(0)];
  let bits = storage.len() * usize::BITS as usize;
  let bitmap = Bitmap::zero(&storage, bits).unwrap();

  bitmap.set(0).unwrap();
  bitmap.set(63).unwrap();
  bitmap.set(64).unwrap();
  bitmap.set(99).unwrap();

  assert!(bitmap.get(0).unwrap());
  assert!(bitmap.get(63).unwrap());
  assert!(bitmap.get(64).unwrap());
  assert!(bitmap.get(99).unwrap());
  assert!(!bitmap.get(32).unwrap());
  assert!(!bitmap.get(96).unwrap());
}

#[test]
fn test_bulk_operations() {
  let storage: [AtomicUsize; 3] = [
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
  ];
  let bits = storage.len() * usize::BITS as usize;
  let bitmap = Bitmap::zero(&storage, bits).unwrap();

  bitmap.set(5).unwrap();
  bitmap.set(35).unwrap();
  bitmap.set(65).unwrap();

  assert!(bitmap.get(5).unwrap());
  assert!(bitmap.get(35).unwrap());
  assert!(bitmap.get(65).unwrap());

  bitmap.clear_all();
  assert!(!bitmap.get(5).unwrap());
  assert!(!bitmap.get(35).unwrap());
  assert!(!bitmap.get(65).unwrap());

  bitmap.set_all();
  assert!(bitmap.get(0).unwrap());
  assert!(bitmap.get(31).unwrap());
  assert!(bitmap.get(32).unwrap());
  assert!(bitmap.get(63).unwrap());
  assert!(bitmap.get(64).unwrap());
  assert!(bitmap.get(191).unwrap()); // 3 * 64 = 192 bits, so 191 is the last valid bit
}

#[test]
fn test_search_operations() {
  let storage: [AtomicUsize; 2] = [AtomicUsize::new(0), AtomicUsize::new(0)];
  let bits = storage.len() * usize::BITS as usize;
  let bitmap = Bitmap::zero(&storage, bits).unwrap();

  assert_eq!(bitmap.find_fs(), None);
  assert_eq!(bitmap.find_fc(), Some(0));

  bitmap.set(5).unwrap();
  bitmap.set(65).unwrap();

  assert_eq!(bitmap.find_fs(), Some(5));
  assert_eq!(bitmap.find_fc(), Some(0));

  bitmap.set(0).unwrap();
  assert_eq!(bitmap.find_fc(), Some(1));

  bitmap.set_all();
  assert_eq!(bitmap.find_fc(), None);
  assert_eq!(bitmap.find_fs(), Some(0));
}

#[test]
fn test_error_handling() {
  let storage: [AtomicUsize; 1] = [AtomicUsize::new(0)];
  let bits = storage.len() * usize::BITS as usize;
  let err = Bitmap::zero(&storage, bits + 1);
  assert!(matches!(
    err,
    Err(BitmapError::InsufficientSize { have, need }) if have
      < need
  ));

  let bitmap = Bitmap::zero(&storage, bits).unwrap();

  assert!(bitmap.set(63).is_ok());
  assert!(bitmap.set(64).is_err());
  assert!(bitmap.get(64).is_err());
  assert!(bitmap.clear(64).is_err());

  let result = bitmap.check(128);
  assert!(result.is_err());
}

#[test]
fn test_partial_word_handling() {
  let storage: [AtomicUsize; 1] = [AtomicUsize::new(0)];
  let bits = storage.len() * usize::BITS as usize;
  let bitmap = Bitmap::zero(&storage, bits).unwrap();

  bitmap.set_all();
  for i in 0..64 {
    assert!(bitmap.get(i).unwrap());
  }

  bitmap.clear_all();
  for i in 0..64 {
    assert!(!bitmap.get(i).unwrap());
  }

  bitmap.set(63).unwrap();
  assert_eq!(bitmap.find_fs(), Some(63));
}

#[test]
fn test_usize_word_type() {
  let storage: [AtomicUsize; 2] = [AtomicUsize::new(0), AtomicUsize::new(0)];
  let bits = storage.len() * usize::BITS as usize;
  let bitmap = Bitmap::zero(&storage, bits).unwrap();
  bitmap.set(7).unwrap();
  bitmap.set(64).unwrap();
  assert!(bitmap.get(7).unwrap());
  assert!(bitmap.get(64).unwrap());

  let single_storage: [AtomicUsize; 1] = [AtomicUsize::new(0)];
  let single_bits = single_storage.len() * usize::BITS as usize;
  let single_bitmap = Bitmap::zero(&single_storage, single_bits).unwrap();
  single_bitmap.set(9).unwrap();
  assert!(single_bitmap.get(9).unwrap());
  assert_eq!(single_bitmap.find_fs(), Some(9));
}

#[test]
fn test_zero_and_one_constructors() {
  let storage: [AtomicUsize; 2] = [AtomicUsize::new(0), AtomicUsize::new(0)];

  // Test zero constructor
  let bits = storage.len() * usize::BITS as usize;
  let bitmap_zero = Bitmap::zero(&storage, bits).unwrap();
  assert!(bitmap_zero.is_clear());
  assert_eq!(bitmap_zero.find_fs(), None);
  assert_eq!(bitmap_zero.find_fc(), Some(0));

  // Test one constructor
  let storage2: [AtomicUsize; 2] = [AtomicUsize::new(0), AtomicUsize::new(0)];
  let bits2 = storage2.len() * usize::BITS as usize;
  let bitmap_one = Bitmap::one(&storage2, bits2).unwrap();
  assert!(!bitmap_one.is_clear());
  assert_eq!(bitmap_one.find_fs(), Some(0));
  assert_eq!(bitmap_one.find_fc(), None);
}

#[test]
fn test_const_functionality() {
  // Test const functions can be used in const contexts
  const WORDS_FOR_64_BITS: usize = Bitmap::words(64);
  const BYTES_FOR_64_BITS: usize = Bitmap::bytes(64);

  assert_eq!(WORDS_FOR_64_BITS, 1);
  assert_eq!(BYTES_FOR_64_BITS, 8);

  // Test methods on bitmap instance
  let storage: [AtomicUsize; 1] = [AtomicUsize::new(0)];
  let bitmap = Bitmap::zero(&storage, 64).unwrap();

  assert_eq!(bitmap.bits(), 64);
  assert_eq!(bitmap.available(), 64);
}
