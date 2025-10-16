#![cfg_attr(not(test), no_std)]
use core::{
  cmp::Ordering,
  ptr::NonNull,
};

use getset::{
  Getters,
  MutGetters,
};

pub trait HasNode {}

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

pub struct RBTree<T, F>
where
  T: HasNode,
  F: Fn(&T, &T) -> Ordering,
{
  root: Option<NonNull<T>>,
  cmp: F,
}

impl<T, F> RBTree<T, F>
where
  T: HasNode,
  F: Fn(&T, &T) -> Ordering,
{
  pub const fn new(cmp: F) -> Self {
    Self { root: None, cmp }
  }
}
