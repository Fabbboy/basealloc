#![cfg_attr(not(test), no_std)]
use core::{
  cmp::Ordering,
  ptr::NonNull,
  sync::atomic::AtomicPtr,
};

use basealloc_fixed::bump::Bump;
use getset::{
  Getters,
  MutGetters,
};
use spin::RwLock;

enum Color {
  Red,
  Black,
}

#[derive(Getters, MutGetters)]
struct Node<T> {
  #[getset(get = "pub", set = "pub")]
  value: RwLock<T>,
  #[getset(get = "pub", set = "pub")]
  parent: AtomicPtr<T>,
  #[getset(get = "pub", set = "pub")]
  left: AtomicPtr<T>,
  #[getset(get = "pub", set = "pub")]
  right: AtomicPtr<T>,
  #[getset(get = "pub", set = "pub")]
  color: RwLock<Color>,
}

pub struct RBTree<T, F = fn(&T, &T) -> Ordering> {
  root: AtomicPtr<Node<T>>,
  bump: Bump,
  cmp: F,
}

impl<T, F> RBTree<T, F>
where
  F: Fn(&T, &T) -> Ordering,
{
  pub const fn new(chunk_size: usize, cmp: F) -> Self {
    Self {
      root: AtomicPtr::new(core::ptr::null_mut()),
      bump: Bump::new(chunk_size),
      cmp,
    }
  }
}
