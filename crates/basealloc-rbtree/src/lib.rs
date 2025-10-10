use std::ptr::NonNull;

use getset::{
  Getters,
  MutGetters,
};

mod config {}

pub trait HasRb {}

#[derive(Debug)]
pub enum RbError {}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub enum Color {
  #[default]
  Red,
  Black,
}

#[derive(Debug, Getters, MutGetters)]
pub struct RbLink<T>
where
  T: HasRb,
{
  #[getset(get = "pub", get_mut = "pub")]
  parent: Option<NonNull<T>>,
  #[getset(get = "pub", get_mut = "pub")]
  left: Option<NonNull<T>>,
  #[getset(get = "pub", get_mut = "pub")]
  right: Option<NonNull<T>>,
  #[getset(get = "pub", get_mut = "pub")]
  color: Color,
}

impl<T> Default for RbLink<T>
where
  T: HasRb,
{
  fn default() -> Self {
    Self {
      parent: None,
      left: None,
      right: None,
      color: Color::default(),
    }
  }
}

pub struct RbTree<T>
where
  T: HasRb,
{
  root: Option<NonNull<T>>,
}
