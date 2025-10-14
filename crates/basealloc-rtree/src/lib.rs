#![cfg_attr(not(test), no_std)]

use core::{
  marker::PhantomData,
  ptr::NonNull,
};

use basealloc_fixed::bump::{
  Bump,
  BumpError,
};
use basealloc_sys::prim::va_size;
use getset::{
  Getters,
  MutGetters,
};

#[derive(Debug)]
pub enum RTreeError {
  Bump(BumpError),
  AlreadyPresent,
}

pub type RTreeResult<T> = Result<T, RTreeError>;
type OptNull<T> = Option<NonNull<T>>;
type OptNode<T, const FANOUT: usize> = OptNull<RNode<T, FANOUT>>;
type NodePtr<T, const FANOUT: usize> = NonNull<RNode<T, FANOUT>>;

#[derive(Getters, MutGetters)]
struct RNode<T, const FANOUT: usize> {
  #[getset(get = "pub", get_mut = "pub")]
  value: OptNull<T>,
  #[getset(get = "pub", get_mut = "pub")]
  children: [OptNode<T, FANOUT>; FANOUT],
  #[getset(get = "pub", get_mut = "pub")]
  parent: OptNode<T, FANOUT>,
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

  #[inline(always)]
  fn child(&self, idx: usize) -> OptNode<T, FANOUT> {
    self.children[idx]
  }

  #[inline(always)]
  fn set_child(&mut self, idx: usize, node: NodePtr<T, FANOUT>) {
    self.children[idx] = Some(node);
  }

  fn clear_slot(&mut self, target: NodePtr<T, FANOUT>) -> bool {
    for child in self.children.iter_mut() {
      if child.map_or(false, |ptr| ptr == target) {
        *child = None;
        return true;
      }
    }
    false
  }
}

pub struct RTree<T, const FANOUT: usize> {
  bump: Bump,
  root: OptNode<T, FANOUT>,
  marker: PhantomData<T>,
}

impl<T, const FANOUT: usize> RTree<T, FANOUT> {
  const BPL: usize = FANOUT.trailing_zeros() as usize;
  const MASK: usize = FANOUT - 1;

  pub const fn new(chunk_size: usize) -> Self {
    Self {
      bump: Bump::new(chunk_size),
      root: None,
      marker: PhantomData,
    }
  }

  const fn levels() -> usize {
    (va_size() + Self::BPL - 1) / Self::BPL
  }

  fn new_node(&mut self, value: OptNull<T>) -> RTreeResult<NodePtr<T, FANOUT>> {
    let node = self
      .bump
      .create::<RNode<T, FANOUT>>()
      .map_err(RTreeError::Bump)?;

    let tmp = RNode::new(value);
    unsafe { node.write(tmp) };
    Ok(unsafe { NonNull::new_unchecked(node) })
  }

  #[inline(always)]
  const fn index_for(key: usize, level: usize) -> usize {
    let shift = (Self::levels() - 1 - level) * Self::BPL;
    (key >> shift) & Self::MASK
  }

  fn ensure_root(&mut self) -> RTreeResult<NodePtr<T, FANOUT>> {
    if self.root.is_none() {
      self.root = Some(self.new_node(None)?);
    }
    Ok(self.root.unwrap())
  }

  fn store(mut node: NodePtr<T, FANOUT>, val: NonNull<T>) -> RTreeResult<()> {
    let n = unsafe { node.as_mut() };
    if n.value.is_some() {
      Err(RTreeError::AlreadyPresent)
    } else {
      n.value = Some(val);
      Ok(())
    }
  }

  pub fn insert(&mut self, key: usize, val: OptNull<T>) -> RTreeResult<()> {
    let Some(value) = val else {
      let _ = self.remove(key);
      return Ok(());
    };

    let leaf = self.ensure_leaf(key)?;
    Self::store(leaf, value)
  }

  pub fn lookup(&self, key: usize) -> Option<&T> {
    let node = self.leaf(key)?;
    unsafe { node.as_ref().value().as_ref().map(|value| value.as_ref()) }
  }

  pub fn lookup_mut(&mut self, key: usize) -> Option<&mut T> {
    let mut node = self.leaf(key)?;
    unsafe {
      node
        .as_mut()
        .value_mut()
        .as_mut()
        .map(|value| value.as_mut())
    }
  }

  pub fn remove(&mut self, key: usize) -> Option<T> {
    let mut node = self.leaf(key)?;
    let value_ptr = unsafe { node.as_mut().value_mut().take()? };
    let value = unsafe { value_ptr.as_ptr().read() };
    self.prune(node);
    Some(value)
  }

  fn leaf(&self, key: usize) -> OptNode<T, FANOUT> {
    let mut current = self.root?;
    let levels = Self::levels();

    for level in 0..levels {
      let idx = Self::index_for(key, level);
      let next = unsafe { current.as_ref().child(idx)? };
      current = next;
    }

    Some(current)
  }

  fn ensure_leaf(&mut self, key: usize) -> RTreeResult<NodePtr<T, FANOUT>> {
    let mut current = self.ensure_root()?;
    let levels = Self::levels();

    for level in 0..levels {
      let idx = Self::index_for(key, level);
      current = self.ensure_child(current, idx)?;
    }

    Ok(current)
  }

  fn ensure_child(
    &mut self,
    mut parent: NodePtr<T, FANOUT>,
    idx: usize,
  ) -> RTreeResult<NodePtr<T, FANOUT>> {
    if let Some(child) = unsafe { parent.as_ref().child(idx) } {
      return Ok(child);
    }

    let mut new_child = self.new_node(None)?;
    unsafe {
      new_child.as_mut().parent = Some(parent);
      parent.as_mut().set_child(idx, new_child);
    }

    Ok(new_child)
  }

  fn prune(&mut self, mut node: NodePtr<T, FANOUT>) {
    loop {
      let should_remove = {
        let n = unsafe { node.as_ref() };
        n.value().is_none() && n.children().iter().all(|child| child.is_none())
      };

      if !should_remove {
        break;
      }

      let parent = unsafe { *node.as_ref().parent() };

      match parent {
        Some(mut parent_ptr) => {
          unsafe {
            let parent_node = parent_ptr.as_mut();
            let _ = parent_node.clear_slot(node);
          }
          node = parent_ptr;
        }
        None => {
          self.root = None;
          break;
        }
      }
    }
  }
}

unsafe impl<T: Send, const FANOUT: usize> Send for RTree<T, FANOUT> {}
unsafe impl<T: Sync, const FANOUT: usize> Sync for RTree<T, FANOUT> {}

#[cfg(test)]
mod tests;
