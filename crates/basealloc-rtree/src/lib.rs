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
  fn load_child(&self, idx: usize) -> Option<NonNull<RNode<T, FANOUT>>> {
    let raw = self.children[idx].load(Ordering::Acquire);
    NonNull::new(raw)
  }

  #[inline(always)]
  fn cas_child(
    &self,
    idx: usize,
    expected: *mut RNode<T, FANOUT>,
    new: NonNull<RNode<T, FANOUT>>,
  ) -> bool {
    self.children[idx]
      .compare_exchange_weak(expected, new.as_ptr(), Ordering::AcqRel, Ordering::Relaxed)
      .is_ok()
  }

  fn clear_child(&mut self, target: NonNull<RNode<T, FANOUT>>) -> bool {
    for child in self.children.iter() {
      loop {
        let current_ptr = child.load(Ordering::Acquire);
        if current_ptr != target.as_ptr() {
          break;
        }
        if child
          .compare_exchange_weak(
            current_ptr,
            core::ptr::null_mut(),
            Ordering::AcqRel,
            Ordering::Relaxed,
          )
          .is_ok()
        {
          return true;
        }
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
    let current = self.root.load(Ordering::Acquire);
    if let Some(root) = NonNull::new(current) {
      return Ok(root);
    }

    let new_root = self.new_node(None)?;

    match self.root.compare_exchange_weak(
      core::ptr::null_mut(),
      new_root.as_ptr(),
      Ordering::AcqRel,
      Ordering::Relaxed,
    ) {
      Ok(_) => Ok(new_root),
      Err(existing) => Ok(unsafe { NonNull::new_unchecked(existing) }),
    }
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
    let root_ptr = self.root.load(Ordering::Acquire);
    let mut current = NonNull::new(root_ptr)?;

    let levels = Self::levels();
    for level in 0..levels {
      let idx = Self::index_for(key, level);
      current = unsafe { current.as_ref().load_child(idx)? };
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
    parent: NonNull<RNode<T, FANOUT>>,
    idx: usize,
  ) -> RTreeResult<NonNull<RNode<T, FANOUT>>> {
    loop {
      let current = unsafe { parent.as_ref() }.children[idx].load(Ordering::Acquire);
      if let Some(child) = NonNull::new(current) {
        return Ok(child);
      }

      let mut new_child = self.new_node(None)?;
      unsafe {
        new_child
          .as_mut()
          .parent
          .store(parent.as_ptr(), Ordering::Release);
      }

      if unsafe { parent.as_ref().cas_child(idx, current, new_child) } {
        return Ok(new_child);
      }
    }
  }

  fn prune(&mut self, mut node: NonNull<RNode<T, FANOUT>>) {
    loop {
      let should_remove = self.should_remove_node(node);
      if !should_remove {
        break;
      }

      let parent_ptr = unsafe { node.as_ref() }.parent.load(Ordering::Acquire);
      if parent_ptr.is_null() {
        self.root.store(core::ptr::null_mut(), Ordering::Release);
        break;
      } else {
        let mut parent_node = unsafe { NonNull::new_unchecked(parent_ptr) };
        let parent_node_mut = unsafe { parent_node.as_mut() };
        let _ = parent_node_mut.clear_child(node);
        node = parent_node;
      }
    }
  }

  fn should_remove_node(&self, node: NonNull<RNode<T, FANOUT>>) -> bool {
    let n = unsafe { node.as_ref() };
    n.value.is_none()
      && n
        .children
        .iter()
        .all(|child| child.load(Ordering::Acquire).is_null())
  }
}

unsafe impl<T: Send, const FANOUT: usize> Send for RTree<T, FANOUT> {}
unsafe impl<T: Sync, const FANOUT: usize> Sync for RTree<T, FANOUT> {}

#[cfg(test)]
mod tests;
