
use basealloc_rtree::RTree;
use basealloc_sys::extent::Extent;

use crate::config::{CHUNK_SIZE, FANOUT};

pub static EMAP: RTree<Extent, FANOUT> = RTree::new(CHUNK_SIZE);

