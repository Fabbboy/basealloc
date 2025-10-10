#![cfg_attr(not(test), no_std)]

use core::{
  marker::PhantomData,
  ptr::NonNull,
};

use getset::{
  Getters,
  MutGetters,
};

pub trait HasLink {
  fn link(&self) -> &Link<Self>
  where
    Self: Sized;
  fn link_mut(&mut self) -> &mut Link<Self>
  where
    Self: Sized;
}

#[derive(Debug, Getters, MutGetters)]
pub struct Link<T>
where
  T: HasLink,
{
  #[getset(get = "pub", get_mut = "pub")]
  next: Option<NonNull<T>>,
  #[getset(get = "pub", get_mut = "pub")]
  prev: Option<NonNull<T>>,
}

pub struct List {}

impl List {
  fn to_non_null<T>(item: &mut T) -> NonNull<T>
  where
    T: HasLink,
  {
    NonNull::from(&mut *item)
  }

  pub fn insert_before<T>(item: &mut T, at: &mut T)
  where
    T: HasLink,
  {
    let at_ptr = Self::to_non_null(at);
    let item_ptr = Self::to_non_null(item);

    let item_link = item.link_mut();
    let at_link = at.link_mut();

    item_link.next = Some(at_ptr);
    item_link.prev = at_link.prev;

    if let Some(mut prev) = at_link.prev {
      unsafe { prev.as_mut().link_mut().next = Some(item_ptr) };
    }

    at_link.prev = Some(item_ptr);
  }

  pub fn insert_after<T>(item: &mut T, at: &mut T)
  where
    T: HasLink,
  {
    let at_ptr = Self::to_non_null(at);
    let item_ptr = Self::to_non_null(item);

    let item_link = item.link_mut();
    let at_link = at.link_mut();

    item_link.prev = Some(at_ptr);
    item_link.next = at_link.next;

    if let Some(mut next) = at_link.next {
      unsafe { next.as_mut().link_mut().prev = Some(item_ptr) };
    }

    at_link.next = Some(item_ptr);
  }

  pub fn remove<T>(item: &mut T)
  where
    T: HasLink,
  {
    let item_link = item.link_mut();

    if let Some(mut prev) = item_link.prev {
      unsafe { prev.as_mut().link_mut().next = item_link.next };
    }

    if let Some(mut next) = item_link.next {
      unsafe { next.as_mut().link_mut().prev = item_link.prev };
    }

    item_link.next = None;
    item_link.prev = None;
  }
}

pub struct ListIter<'list, T>
where
  T: HasLink + 'list,
{
  next: Option<NonNull<T>>,
  marker: PhantomData<&'list T>,
}

impl<'list, T> ListIter<'list, T>
where
  T: HasLink + 'list,
{
  pub fn new(start: Option<NonNull<T>>) -> Self {
    Self {
      next: start,
      marker: PhantomData,
    }
  }
}

impl<'list, T> From<&'list T> for ListIter<'list, T>
where
  T: HasLink + 'list,
{
  fn from(start: &'list T) -> Self {
    Self::new(Some(NonNull::from(start)))
  }
}

impl<'list, T> Iterator for ListIter<'list, T>
where
  T: HasLink + 'list,
{
  type Item = &'list mut T;

  fn next(&mut self) -> Option<Self::Item> {
    if let Some(mut current) = self.next {
      let current_ref = unsafe { current.as_mut() };
      self.next = current_ref.link().next;
      Some(current_ref)
    } else {
      None
    }
  }
}

pub struct ListDrainer<'list, T>
where
  T: HasLink + 'list,
{
  next: Option<NonNull<T>>,
  marker: PhantomData<&'list T>,
}

impl<'list, T> ListDrainer<'list, T>
where
  T: HasLink + 'list,
{
  pub fn new(start: Option<NonNull<T>>) -> Self {
    Self {
      next: start,
      marker: PhantomData,
    }
  }
}

impl<'list, T> From<&'list T> for ListDrainer<'list, T>
where
  T: HasLink + 'list,
{
  fn from(start: &'list T) -> Self {
    Self::new(Some(NonNull::from(start)))
  }
}

impl<'list, T> Iterator for ListDrainer<'list, T>
where
  T: HasLink + 'list,
{
  type Item = &'list mut T;

  fn next(&mut self) -> Option<Self::Item> {
    if let Some(mut current) = self.next {
      let current_ref = unsafe { current.as_mut() };
      self.next = current_ref.link().next;
      List::remove(current_ref);
      Some(current_ref)
    } else {
      None
    }
  }
}

impl<T> Drop for Link<T>
where
  T: HasLink,
{
  fn drop(&mut self) {
    self.next = None;
    self.prev = None;
  }
}

#[cfg(test)]
mod tests;
