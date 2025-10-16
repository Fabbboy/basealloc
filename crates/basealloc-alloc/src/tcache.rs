use basealloc_rbtree::RBTree;
use basealloc_sys::extent::Extent;
use std::cmp::Ordering;

thread_local! {
  pub static EFREE: RBTree<Extent, fn(&Extent, &Extent) -> Ordering> = RBTree::new(Extent::ord);
}
