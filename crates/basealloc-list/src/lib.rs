#![cfg_attr(not(test), no_std)]

use core::{
  marker::PhantomData,
  ptr::{
    NonNull,
    drop_in_place,
  },
  sync::atomic::{AtomicPtr, Ordering},
};


pub trait HasLink {
  fn link(&self) -> &Link<Self>
  where
    Self: Sized;
  fn link_mut(&mut self) -> &mut Link<Self>
  where
    Self: Sized;
}

#[derive(Debug)]
pub struct Link<T>
where
  T: HasLink,
{
  next: AtomicPtr<T>,
  prev: AtomicPtr<T>,
}

impl<T> Link<T>
where
  T: HasLink,
{
  pub fn next(&self) -> Option<NonNull<T>> {
    NonNull::new(self.next.load(Ordering::Acquire))
  }
  
  pub fn prev(&self) -> Option<NonNull<T>> {
    NonNull::new(self.prev.load(Ordering::Acquire))
  }
  
  pub fn set_next(&self, ptr: Option<NonNull<T>>) {
    let raw = ptr.map_or(core::ptr::null_mut(), |p| p.as_ptr());
    self.next.store(raw, Ordering::Release);
  }
  
  pub fn set_prev(&self, ptr: Option<NonNull<T>>) {
    let raw = ptr.map_or(core::ptr::null_mut(), |p| p.as_ptr());
    self.prev.store(raw, Ordering::Release);
  }
}

impl<T> Default for Link<T>
where
  T: HasLink,
{
  fn default() -> Self {
    Self {
      next: AtomicPtr::new(core::ptr::null_mut()),
      prev: AtomicPtr::new(core::ptr::null_mut()),
    }
  }
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

    let item_link = item.link();
    let at_link = at.link();

    item_link.set_next(Some(at_ptr));
    
    let prev_ptr = at_link.prev();
    item_link.set_prev(prev_ptr);
    
    prev_ptr.map(|prev| unsafe {
      prev.as_ref().link().set_next(Some(item_ptr))
    });
    
    at_link.set_prev(Some(item_ptr));
  }

  pub fn insert_after<T>(item: &mut T, at: &mut T)
  where
    T: HasLink,
  {
    let at_ptr = Self::to_non_null(at);
    let item_ptr = Self::to_non_null(item);

    let item_link = item.link();
    let at_link = at.link();

    item_link.set_prev(Some(at_ptr));
    
    let next_ptr = at_link.next();
    item_link.set_next(next_ptr);
    
    next_ptr.map(|next| unsafe {
      next.as_ref().link().set_prev(Some(item_ptr))
    });
    
    at_link.set_next(Some(item_ptr));
  }

  pub fn remove<T>(item: &mut T)
  where
    T: HasLink,
  {
    let item_link = item.link();

    let prev_ptr = item_link.prev();
    let next_ptr = item_link.next();

    prev_ptr.map(|prev| unsafe {
      prev.as_ref().link().set_next(next_ptr)
    });

    next_ptr.map(|next| unsafe {
      next.as_ref().link().set_prev(prev_ptr)
    });

    item_link.set_next(None);
    item_link.set_prev(None);
  }

  pub fn drain<'list, T>(start: &'list mut T) -> ListDrainer<'list, T>
  where
    T: HasLink + 'list,
  {
    ListDrainer::from(start)
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
    let current = self.next.take()?;
    let current_ref = unsafe { current.as_ref() };
    self.next = current_ref.link().next();
    Some(unsafe { &mut *(current.as_ptr()) })
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

impl<'list, T> From<&'list mut T> for ListDrainer<'list, T>
where
  T: HasLink + 'list,
{
  fn from(start: &'list mut T) -> Self {
    Self::new(Some(NonNull::from(start)))
  }
}

impl<'list, T> Iterator for ListDrainer<'list, T>
where
  T: HasLink + 'list,
{
  type Item = &'list mut T;

  fn next(&mut self) -> Option<Self::Item> {
    let current = self.next.take()?;
    let current_ref = unsafe { &mut *current.as_ptr() };
    self.next = current_ref.link().next();
    List::remove(current_ref);
    Some(current_ref)
  }
}

impl<'list, T> Drop for ListDrainer<'list, T>
where
  T: HasLink + 'list,
{
  fn drop(&mut self) {
    while let Some(current) = self.next.take() {
      unsafe {
        self.next = current.as_ref().link().next();
        drop_in_place(current.as_ptr());
      }
    }
  }
}

impl<T> Drop for Link<T>
where
  T: HasLink,
{
  fn drop(&mut self) {
    self.set_next(None);
    self.set_prev(None);
  }
}

#[cfg(test)]
mod tests;

pub mod prelude {
  pub use super::{
    HasLink,
    Link,
    List,
    ListDrainer,
    ListIter,
  };
}
