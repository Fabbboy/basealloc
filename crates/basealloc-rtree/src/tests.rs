use super::{
  RTree,
  RTreeError,
};

const CHUNK: usize = 4096;
const FANOUT: usize = 1 << 4;

#[test]
fn insert_and_lookup_round_trip() {
  let mut tree: RTree<usize, FANOUT> = RTree::new(CHUNK);

  tree
    .insert(0x1234, Some(42))
    .expect("insert should succeed");
  assert_eq!(tree.lookup(0x1234), Some(&42));
  assert_eq!(tree.lookup(0x9999), None);

  let removed = tree.remove(0x1234);
  assert_eq!(removed, Some(42));
}

#[test]
fn duplicate_insert_fails() {
  let mut tree: RTree<usize, FANOUT> = RTree::new(CHUNK);
  tree.insert(0xDEAD, Some(1)).expect("first insert succeeds");
  let err = tree
    .insert(0xDEAD, Some(2))
    .expect_err("duplicate should fail");
  assert!(matches!(err, RTreeError::AlreadyPresent));

  let removed = tree.remove(0xDEAD);
  assert_eq!(removed, Some(1));
}

#[test]
fn remove_prunes_empty_path() {
  let mut tree: RTree<usize, FANOUT> = RTree::new(CHUNK);
  let key = 0xABCD;
  tree.insert(key, Some(77)).expect("insert");
  let removed = tree.remove(key);
  assert_eq!(removed, Some(77));
  assert_eq!(tree.lookup(key), None, "node should be pruned");
}

#[test]
fn insert_none_acts_as_remove() {
  let mut tree: RTree<usize, FANOUT> = RTree::new(CHUNK);
  let key = 0x1111usize;
  tree.insert(key, Some(100)).expect("insert");
  tree.insert(key, None).expect("None insert acts as remove");
  assert_eq!(tree.lookup(key), None);
}

#[test]
fn multiple_entries_share_prefix() {
  let mut tree: RTree<usize, FANOUT> = RTree::new(CHUNK);
  let base = 0x12340000usize;
  let keys = [base, base + 1, base + FANOUT];

  for &key in &keys {
    tree.insert(key, Some(key)).expect("insert");
  }

  for key in keys {
    assert_eq!(tree.lookup(key), Some(&key));
    let removed = tree.remove(key);
    assert_eq!(removed, Some(key));
    assert_eq!(tree.lookup(key), None);
  }
}

#[test]
fn lookup_mut_allows_updates() {
  let mut tree: RTree<usize, FANOUT> = RTree::new(CHUNK);
  let key = 0x5555usize;
  tree.insert(key, Some(10)).expect("insert");

  let value = tree.lookup_mut(key).expect("value should exist");
  *value = 99;

  assert_eq!(tree.lookup(key), Some(&99));
  assert_eq!(tree.remove(key), Some(99));
}
