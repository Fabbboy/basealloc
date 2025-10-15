
use core::sync::atomic::AtomicPtr;

use basealloc_rtree::RTree;
use basealloc_sys::extent::Extent;
use heapless::Vec;

use crate::{arena::Arena, config::{CHUNK_SIZE, FANOUT, MAX_ARENAS}};

pub static ARENAS: Vec<AtomicPtr<Arena>, { MAX_ARENAS }> = Vec::new();
pub static EMAP: RTree<Extent, FANOUT> = RTree::new(CHUNK_SIZE);

