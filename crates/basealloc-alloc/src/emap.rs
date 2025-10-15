use core::sync::atomic::AtomicPtr;

use basealloc_rtree::RTree;
use basealloc_sys::extent::Extent;

use crate::config::{CHUNK_SIZE, FANOUT};

pub struct EMeta {
  extent: AtomicPtr<Extent>,
  start: usize,
  size: usize,
}

pub static EMAP: RTree<EMeta, FANOUT> = RTree::new(CHUNK_SIZE);

