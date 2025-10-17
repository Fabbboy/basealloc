#![cfg_attr(not(test), no_std)]

use core::sync::atomic::{
  AtomicUsize,
  Ordering,
};

#[derive(Debug, PartialEq, Eq)]
pub enum RingError<T> {
  Full(T),
}

pub struct Ring<T, const N: usize>
where
  T: Copy,
{
  buf: [T; N],
  head: AtomicUsize,
  tail: AtomicUsize,
  len: AtomicUsize,
}

impl<T, const N: usize> Ring<T, N>
where
  T: Copy,
{
  pub const fn new(init: T) -> Self {
    Self {
      buf: [init; N],
      head: AtomicUsize::new(0),
      tail: AtomicUsize::new(0),
      len: AtomicUsize::new(0),
    }
  }

  pub const fn capacity(&self) -> usize {
    N
  }

  pub fn len(&self) -> usize {
    self.len.load(Ordering::Relaxed)
  }

  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  pub fn is_full(&self) -> bool {
    self.len() == N
  }

  fn next_idx(current: usize) -> usize {
    (current + 1) % N
  }

  pub fn push(&mut self, val: T) -> Result<(), RingError<T>> {
    if self.is_full() {
      return Err(RingError::Full(val));
    }

    let head = self.head.load(Ordering::Relaxed);
    self.buf[head] = val;
    self.head.store(Self::next_idx(head), Ordering::Relaxed);
    self.len.fetch_add(1, Ordering::Relaxed);

    Ok(())
  }

  pub fn pop(&mut self) -> Option<T> {
    if self.is_empty() {
      return None;
    }

    let tail = self.tail.load(Ordering::Relaxed);
    let val = self.buf[tail];
    self.tail.store(Self::next_idx(tail), Ordering::Relaxed);
    self.len.fetch_sub(1, Ordering::Relaxed);

    Some(val)
  }
}

#[cfg(test)]
mod tests;
