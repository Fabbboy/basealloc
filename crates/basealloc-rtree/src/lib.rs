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
  children: [Option<AtomicPtr<RNode<T, FANOUT>>>; FANOUT],
  parent: Option<AtomicPtr<RNode<T, FANOUT>>>,
}

impl<T, const FANOUT: usize> RNode<T, FANOUT> {
  pub fn new(value: Option<T>) -> Self {
    Self {
      value,
      children: core::array::from_fn(|_| None),
      parent: None,
    }
  }

  #[inline(always)]
  fn child(&self, idx: usize) -> Option<NonNull<RNode<T, FANOUT>>> {
    let atomic = self.children[idx].as_ref()?;
    let raw = atomic.load(Ordering::Acquire);
    // SAFETY: Child pointers are always initialised with a non-null address.
    Some(unsafe { NonNull::new_unchecked(raw) })
  }

  #[inline(always)]
  fn set_child(&mut self, idx: usize, node: NonNull<RNode<T, FANOUT>>) {
    if let Some(child) = self.children[idx].as_ref() {
      child.store(node.as_ptr(), Ordering::Release);
    } else {
      self.children[idx] = Some(AtomicPtr::new(node.as_ptr()));
    }
  }

  fn clear_slot(&mut self, target: NonNull<RNode<T, FANOUT>>) -> bool {
    for child in self.children.iter_mut() {
      match child {
        Some(ptr) if ptr.load(Ordering::Acquire) == target.as_ptr() => {
          *child = None;
          return true;
        }
        _ => {}
      }
    }
    false
  }
}

pub struct RTree<T, const FANOUT: usize> {
  bump: Bump,
  root: Option<AtomicPtr<RNode<T, FANOUT>>>,
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

  const fn levels() -> usize {
    (va_size() + Self::BPL - 1) / Self::BPL
  }

  fn new_node(&mut self, value: Option<T>) -> RTreeResult<NonNull<RNode<T, FANOUT>>> {
    let node = self
      .bump
      .create::<RNode<T, FANOUT>>()
      .map_err(RTreeError::Bump)?;

    let tmp = RNode::new(value);
    unsafe { node.write(tmp) };
    // SAFETY: `Bump::create` never returns a null pointer on success.
    Ok(unsafe { NonNull::new_unchecked(node) })
  }

  #[inline(always)]
  const fn index_for(key: usize, level: usize) -> usize {
    let shift = (Self::levels() - 1 - level) * Self::BPL;
    (key >> shift) & Self::MASK
  }

  fn ensure_root(&mut self) -> RTreeResult<NonNull<RNode<T, FANOUT>>> {
    if let Some(existing) = self.root.as_ref() {
      let raw = existing.load(Ordering::Acquire);
      // SAFETY: Root is only stored as a valid, non-null pointer.
      return Ok(unsafe { NonNull::new_unchecked(raw) });
    }

    let new_root = self.new_node(None)?;
    self.root = Some(AtomicPtr::new(new_root.as_ptr()));
    Ok(new_root)
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

  pub fn insert(&mut self, key: usize, val: Option<T>) -> RTreeResult<()> {
    let Some(value) = val else {
      let _ = self.remove(key);
      return Ok(());
    };

    let leaf = self.ensure_leaf(key)?;
    Self::store(leaf, value)
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
    let root_atomic = self.root.as_ref()?;
    let raw = root_atomic.load(Ordering::Acquire);
    // SAFETY: Root pointers are only stored as valid, non-null addresses.
    let mut current = unsafe { NonNull::new_unchecked(raw) };
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
      new_child.as_mut().parent = Some(AtomicPtr::new(parent.as_ptr()));
      parent.as_mut().set_child(idx, new_child);
    }

    Ok(new_child)
  }

  fn prune(&mut self, mut node: NonNull<RNode<T, FANOUT>>) {
    loop {
      let should_remove = {
        let n = unsafe { node.as_ref() };
        n.value.is_none() && n.children.iter().all(|child| child.is_none())
      };

      if !should_remove {
        break;
      }

      let parent_atomic = unsafe { node.as_ref() }.parent.as_ref();

      match parent_atomic {
        Some(parent_ptr) => {
          let raw = parent_ptr.load(Ordering::Acquire);
          // SAFETY: Parent pointers are only stored as valid, non-null addresses.
          let mut parent_node_ptr = unsafe { NonNull::new_unchecked(raw) };
          let parent_node = unsafe { parent_node_ptr.as_mut() };
          let _ = parent_node.clear_slot(node);
          node = parent_node_ptr;
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
