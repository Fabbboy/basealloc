#![cfg_attr(not(test), no_std)]
use core::{
  cmp::Ordering,
  ptr::NonNull,
};

use basealloc_fixed::bump::Bump;
use getset::{
  Getters,
  MutGetters,
};

pub trait HasNode {
  fn node(&self) -> &RBNode<Self>
  where
    Self: Sized;
  fn node_mut(&mut self) -> &mut RBNode<Self>
  where
    Self: Sized;
}

pub enum Color {
  Red,
  Black,
}

#[derive(Getters, MutGetters)]
pub struct RBNode<T>
where
  T: HasNode,
{
  #[getset(get = "pub", set = "pub")]
  parent: Option<NonNull<T>>,
  #[getset(get = "pub", set = "pub")]
  left: Option<NonNull<T>>,
  #[getset(get = "pub", set = "pub")]
  right: Option<NonNull<T>>,
  #[getset(get = "pub", set = "pub")]
  color: Color,
}

impl<T> Default for RBNode<T>
where
  T: HasNode,
{
  fn default() -> Self {
    Self {
      parent: None,
      left: None,
      right: None,
      color: Color::Red,
    }
  }
}

pub struct RBTree<T, F>
where
  T: HasNode,
  F: Fn(&T, &T) -> Ordering,
{
  root: Option<NonNull<T>>,
  bump: Bump,
  cmp: F,
}

impl<T, F> RBTree<T, F>
where
  T: HasNode,
  F: Fn(&T, &T) -> Ordering,
{
  pub const fn new(chunk_size: usize, cmp: F) -> Self {
    Self {
      root: None,
      bump: Bump::new(chunk_size),
      cmp,
    }
  }
}
