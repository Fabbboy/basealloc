use core::ptr::NonNull;

use super::{
  RTree,
  RTreeError,
};

const CHUNK: usize = 4096;
const FANOUT: usize = 1 << 4;

fn boxed(value: usize) -> NonNull<usize> {
  let boxed = Box::new(value);
  let ptr = Box::into_raw(boxed);
  unsafe { NonNull::new_unchecked(ptr) }
}

fn from_ptr(ptr: NonNull<usize>) -> usize {
  unsafe {
    let boxed = Box::from_raw(ptr.as_ptr());
    *boxed
  }
}

#[test]
fn insert_and_lookup_round_trip() {
  let mut tree: RTree<usize, FANOUT> = RTree::new(CHUNK);
  let value = boxed(42);

  tree
    .insert(0x1234, Some(value))
    .expect("insert should succeed");
  assert_eq!(tree.lookup(0x1234), Some(&42));
  assert_eq!(tree.lookup(0x9999), None);

  let removed = tree.remove(0x1234);
  assert_eq!(removed, Some(42));
  let _ = from_ptr(value);
}

#[test]
fn duplicate_insert_fails() {
  let mut tree: RTree<usize, FANOUT> = RTree::new(CHUNK);
  let first = boxed(1);
  let second = boxed(2);

  tree
    .insert(0xDEAD, Some(first))
    .expect("first insert succeeds");
  let err = tree
    .insert(0xDEAD, Some(second))
    .expect_err("duplicate should fail");
  assert!(matches!(err, RTreeError::AlreadyPresent));

  let removed = tree.remove(0xDEAD);
  assert_eq!(removed, Some(1));
  let _ = from_ptr(first);
  let _ = from_ptr(second);
}

#[test]
fn remove_prunes_empty_path() {
  let mut tree: RTree<usize, FANOUT> = RTree::new(CHUNK);
  let key = 0xABCD;
  let value = boxed(77);

  tree.insert(key, Some(value)).expect("insert");
  let removed = tree.remove(key);
  assert_eq!(removed, Some(77));
  assert_eq!(tree.lookup(key), None, "node should be pruned");
  let _ = from_ptr(value);
}

#[test]
fn insert_none_acts_as_remove() {
  let mut tree: RTree<usize, FANOUT> = RTree::new(CHUNK);
  let key = 0x1111usize;
  let value = boxed(100);

  tree.insert(key, Some(value)).expect("insert");
  tree.insert(key, None).expect("None insert acts as remove");
  assert_eq!(tree.lookup(key), None);

  let _ = from_ptr(value);
}

#[test]
fn multiple_entries_share_prefix() {
  let mut tree: RTree<usize, FANOUT> = RTree::new(CHUNK);
  let base = 0x12340000usize;
  let keys = [base, base + 1, base + FANOUT];
  let entries = keys.map(|key| (key, boxed(key)));

  for &(key, ptr) in entries.iter() {
    tree.insert(key, Some(ptr)).expect("insert");
  }

  for (key, ptr) in entries {
    assert_eq!(tree.lookup(key), Some(&key));
    let removed = tree.remove(key);
    assert_eq!(removed, Some(key));
    let _ = from_ptr(ptr);
    assert_eq!(tree.lookup(key), None);

    // ptr already consumed via from_ptr
  }
}
