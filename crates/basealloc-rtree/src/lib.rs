#![cfg_attr(not(test), no_std)]

use core::{
  ptr::NonNull,
  sync::atomic::{
    AtomicPtr,
    Ordering,
  },
};

use basealloc_fixed::bump::{
  Bump,
  BumpError,
};
use basealloc_sys::prim::va_size;

#[derive(Debug)]
pub enum RTreeError {
  Bump(BumpError),
  AlreadyPresent,
}

pub type RTreeResult<T> = Result<T, RTreeError>;

struct RNode<T, const FANOUT: usize> {
  value: Option<T>,
  children: [AtomicPtr<RNode<T, FANOUT>>; FANOUT],
  parent: AtomicPtr<RNode<T, FANOUT>>,
}

impl<T, const FANOUT: usize> RNode<T, FANOUT> {
  pub fn new(value: Option<T>) -> Self {
    Self {
      value,
      children: core::array::from_fn(|_| AtomicPtr::new(core::ptr::null_mut())),
      parent: AtomicPtr::new(core::ptr::null_mut()),
    }
  }

  #[inline(always)]
  fn child(&self, idx: usize) -> Option<NonNull<RNode<T, FANOUT>>> {
    let raw = self.children[idx].load(Ordering::Acquire);
    NonNull::new(raw)
  }

  #[inline(always)]
  fn set_child(&mut self, idx: usize, node: NonNull<RNode<T, FANOUT>>) {
    self.children[idx].store(node.as_ptr(), Ordering::Release);
  }

  fn clear_slot(&mut self, target: NonNull<RNode<T, FANOUT>>) -> bool {
    for child in self.children.iter() {
      let current_ptr = child.load(Ordering::Acquire);
      if current_ptr == target.as_ptr() {
        child.store(core::ptr::null_mut(), Ordering::Release);
        return true;
      }
    }
    false
  }
}

pub struct RTree<T, const FANOUT: usize> {
  bump: Bump,
  root: AtomicPtr<RNode<T, FANOUT>>,
}

impl<T, const FANOUT: usize> RTree<T, FANOUT> {
  const BPL: usize = FANOUT.trailing_zeros() as usize;
  const MASK: usize = FANOUT - 1;

  pub const fn new(chunk_size: usize) -> Self {
    Self {
      bump: Bump::new(chunk_size),
      root: AtomicPtr::new(core::ptr::null_mut()),
    }
  }

  const fn levels() -> usize {
    va_size().div_ceil(Self::BPL)
  }

  fn new_node(&mut self, value: Option<T>) -> RTreeResult<NonNull<RNode<T, FANOUT>>> {
    let node = self
      .bump
      .create::<RNode<T, FANOUT>>()
      .map_err(RTreeError::Bump)?;

    let tmp = RNode::new(value);
    unsafe { (*node).write(tmp) };
    // SAFETY: `Bump::create` never returns a null pointer on success.
    Ok(unsafe { NonNull::new_unchecked((*node).as_mut_ptr()) })
  }

  #[inline(always)]
  const fn index_for(key: usize, level: usize) -> usize {
    let shift = (Self::levels() - 1 - level) * Self::BPL;
    (key >> shift) & Self::MASK
  }

  fn ensure_root(&mut self) -> RTreeResult<NonNull<RNode<T, FANOUT>>> {
    let raw = self.root.load(Ordering::Acquire);
    NonNull::new(raw).map(Ok).unwrap_or_else(|| {
      let new_root = self.new_node(None)?;
      self.root.store(new_root.as_ptr(), Ordering::Release);
      Ok(new_root)
    })
  }

  fn store(mut node: NonNull<RNode<T, FANOUT>>, val: T) -> RTreeResult<()> {
    let n = unsafe { node.as_mut() };
    if n.value.is_some() {
      Err(RTreeError::AlreadyPresent)
    } else {
      n.value = Some(val);
      Ok(())
    }
  }

  pub fn insert(&mut self, key: usize, val: T) -> RTreeResult<()> {
    let leaf = self.ensure_leaf(key)?;
    Self::store(leaf, val)
  }

  pub fn lookup(&self, key: usize) -> Option<&T> {
    let node = self.leaf(key)?;
    let node_ref = unsafe { node.as_ref() };
    node_ref.value.as_ref()
  }

  pub fn lookup_mut(&mut self, key: usize) -> Option<&mut T> {
    let mut node = self.leaf(key)?;
    let node_mut = unsafe { node.as_mut() };
    node_mut.value.as_mut()
  }

  pub fn remove(&mut self, key: usize) -> Option<T> {
    let mut node = self.leaf(key)?;
    let node_mut = unsafe { node.as_mut() };
    let val = node_mut.value.take();
    self.prune(node);
    val
  }

  fn leaf(&self, key: usize) -> Option<NonNull<RNode<T, FANOUT>>> {
    let raw = self.root.load(Ordering::Acquire);
    let mut current = NonNull::new(raw)?;
    let levels = Self::levels();

    for level in 0..levels {
      let idx = Self::index_for(key, level);
      let next = unsafe { current.as_ref().child(idx)? };
      current = next;
    }

    Some(current)
  }

  fn ensure_leaf(&mut self, key: usize) -> RTreeResult<NonNull<RNode<T, FANOUT>>> {
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
    mut parent: NonNull<RNode<T, FANOUT>>,
    idx: usize,
  ) -> RTreeResult<NonNull<RNode<T, FANOUT>>> {
    if let Some(child) = unsafe { parent.as_ref().child(idx) } {
      return Ok(child);
    }

    let mut new_child = self.new_node(None)?;
    unsafe {
      new_child
        .as_mut()
        .parent
        .store(parent.as_ptr(), Ordering::Release);
      parent.as_mut().set_child(idx, new_child);
    }

    Ok(new_child)
  }

  fn prune(&mut self, mut node: NonNull<RNode<T, FANOUT>>) {
    loop {
      let should_remove = {
        let n = unsafe { node.as_ref() };
        n.value.is_none()
          && n
            .children
            .iter()
            .all(|child| child.load(Ordering::Acquire).is_null())
      };

      if !should_remove {
        break;
      }

      let parent_raw = unsafe { node.as_ref() }.parent.load(Ordering::Acquire);

      NonNull::new(parent_raw).map_or_else(
        || {
          self.root.store(core::ptr::null_mut(), Ordering::Release);
        },
        |mut parent_node_ptr| {
          let parent_node = unsafe { parent_node_ptr.as_mut() };
          let _ = parent_node.clear_slot(node);
          node = parent_node_ptr;
        },
      );

      if parent_raw.is_null() {
        break;
      }
    }
  }
}

unsafe impl<T: Send, const FANOUT: usize> Send for RTree<T, FANOUT> {}
unsafe impl<T: Sync, const FANOUT: usize> Sync for RTree<T, FANOUT> {}

#[cfg(test)]
mod tests;
