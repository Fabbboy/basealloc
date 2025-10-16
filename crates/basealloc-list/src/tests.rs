use super::*;
use core::ptr::NonNull;

#[derive(Debug)]
struct TestNode {
  value: i32,
  link: Link<Self>,
}

impl TestNode {
  fn new(value: i32) -> Self {
    Self {
      value,
      link: Link::default(),
    }
  }
}

impl HasLink for TestNode {
  fn link(&self) -> &Link<Self> {
    &self.link
  }

  fn link_mut(&mut self) -> &mut Link<Self> {
    &mut self.link
  }
}

#[test]
fn test_insert_before() {
  let mut node1 = TestNode::new(1);
  let mut node2 = TestNode::new(2);

  List::insert_before(&mut node2, &mut node1);

  let node1_ptr = NonNull::from(&node1);
  let node2_ptr = NonNull::from(&node2);

  assert_eq!(node2.link().next(), Some(node1_ptr));
  assert_eq!(node1.link().prev(), Some(node2_ptr));
}

#[test]
fn test_insert_after() {
  let mut node1 = TestNode::new(1);
  let mut node2 = TestNode::new(2);

  List::insert_after(&mut node2, &mut node1);

  let node1_ptr = NonNull::from(&node1);
  let node2_ptr = NonNull::from(&node2);

  assert_eq!(node1.link().next(), Some(node2_ptr));
  assert_eq!(node2.link().prev(), Some(node1_ptr));
}

#[test]
fn test_remove() {
  let mut node1 = TestNode::new(1);
  let mut node2 = TestNode::new(2);
  let mut node3 = TestNode::new(3);

  List::insert_after(&mut node2, &mut node1);
  List::insert_after(&mut node3, &mut node2);
  List::remove(&mut node2);

  let node1_ptr = NonNull::from(&node1);
  let node3_ptr = NonNull::from(&node3);

  assert_eq!(node1.link().next(), Some(node3_ptr));
  assert_eq!(node3.link().prev(), Some(node1_ptr));
  assert!(node2.link().next().is_none());
  assert!(node2.link().prev().is_none());
}

#[test]
fn test_iter() {
  let mut node1 = TestNode::new(1);
  let mut node2 = TestNode::new(2);
  let mut node3 = TestNode::new(3);

  List::insert_after(&mut node2, &mut node1);
  List::insert_after(&mut node3, &mut node2);

  let iter = ListIter::from(&node1);
  let values: Vec<i32> = iter.map(|n| n.value).collect();

  assert_eq!(values, vec![1, 2, 3]);
}

#[test]
fn test_drainer() {
  let mut node1 = TestNode::new(1);
  let mut node2 = TestNode::new(2);
  let mut node3 = TestNode::new(3);

  List::insert_after(&mut node2, &mut node1);
  List::insert_after(&mut node3, &mut node2);

  let drainer = ListDrainer::from(&node1);
  let values: Vec<i32> = drainer.map(|n| n.value).collect();

  assert_eq!(values, vec![1, 2, 3]);

  assert!(node1.link().next().is_none());
  assert!(node1.link().prev().is_none());
  assert!(node2.link().next().is_none());
  assert!(node2.link().prev().is_none());
  assert!(node3.link().next().is_none());
  assert!(node3.link().prev().is_none());
}
