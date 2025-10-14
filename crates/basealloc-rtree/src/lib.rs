#![cfg_attr(not(test), no_std)]

use core::{
  marker::PhantomData,
  ptr::NonNull,
};

use basealloc_fixed::bump::{
  Bump,
  BumpError,
};
use basealloc_sys::prim::{
  va_size,
  word_width,
};
use getset::{
  Getters,
  MutGetters,
};

#[derive(Debug)]
pub enum RTreeError {
  Bump(BumpError),
  Duplicated,
}

pub type RTreeResult<T> = Result<T, RTreeError>;
type OptNull<T> = Option<NonNull<T>>;
type OptNullNode<T, const FANOUT: usize> = OptNull<RNode<T, FANOUT>>;
type NNullNode<T, const FANOUT: usize> = NonNull<RNode<T, FANOUT>>;

#[derive(Getters, MutGetters)]
struct RNode<T, const FANOUT: usize> {
  #[getset(get = "pub", get_mut = "pub")]
  value: OptNull<T>,
  #[getset(get = "pub", get_mut = "pub")]
  children: [OptNullNode<T, FANOUT>; FANOUT],
  #[getset(get = "pub", get_mut = "pub")]
  parent: OptNullNode<T, FANOUT>,
}

impl<T, const FANOUT: usize> RNode<T, FANOUT> {
  pub fn new(value: OptNull<T>) -> Self {
    let children = core::array::from_fn(|_| None);

    Self {
      value,
      children,
      parent: None,
    }
  }

  pub fn is_leaf(&self) -> bool {
    self.children.iter().all(|c| c.is_none())
  }

  #[inline(always)]
  fn child(&self, idx: usize) -> OptNullNode<T, FANOUT> {
    self.children[idx]
  }

  #[inline(always)]
  fn set_child(&mut self, idx: usize, node: NNullNode<T, FANOUT>) {
    self.children[idx] = Some(node);
  }
}

pub struct RTree<T, const FANOUT: usize> {
  bump: Bump,
  root: Option<NonNull<RNode<T, FANOUT>>>,
}

impl<T, const FANOUT: usize> RTree<T, FANOUT> {
  const BPL: usize = FANOUT.trailing_zeros() as usize;
  const MASK: usize = FANOUT - 1;

  pub const fn new(chunk_size: usize) -> Self {
    Self {
      bump: Bump::new(chunk_size),
      root: None,
    }
  }

  fn levels() -> usize {
    (va_size() + Self::BPL - 1) / Self::BPL
  }

  fn new_node(&mut self, value: OptNull<T>) -> RTreeResult<NNullNode<T, FANOUT>> {
    let node = self
      .bump
      .create::<RNode<T, FANOUT>>()
      .map_err(RTreeError::Bump)?;

    let tmp = RNode::new(value);
    unsafe { node.write(tmp) };
    Ok(unsafe { NonNull::new_unchecked(node) })
  }

  #[inline(always)]
  fn index_for(key: usize, level: usize) -> usize {
    let shift = (Self::levels() - 1 - level) * Self::BPL;
    (key >> shift) & Self::MASK
  }

  fn ensure_root(&mut self) -> RTreeResult<NNullNode<T, FANOUT>> {
    if self.root.is_none() {
      self.root = Some(self.new_node(None)?);
    }
    Ok(self.root.unwrap())
  }

  fn install(mut node: NNullNode<T, FANOUT>, val: NonNull<T>) -> RTreeResult<()> {
    let n = unsafe { node.as_mut() };
    if n.value.is_some() {
      Err(RTreeError::Duplicated)
    } else {
      n.value = Some(val);
      Ok(())
    }
  }

  pub fn insert(&mut self, key: usize, val: OptNull<T>) -> RTreeResult<()> {
    todo!()
  }

  pub fn lookup(&self, key: usize) -> Option<&T> {
    todo!()
  }

  pub fn lookup_mut(&mut self, key: usize) -> Option<&mut T> {
    todo!()
  }

  pub fn remove(&mut self, key: usize) -> Option<T> {
    todo!()
  }
}
