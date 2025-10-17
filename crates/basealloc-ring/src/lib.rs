#![cfg_attr(not(test), no_std)]

use core::sync::atomic::{
  AtomicUsize,
  Ordering,
};

#[derive(Debug, PartialEq, Eq)]
pub enum RingError<T> {
  Full(T),
}

pub struct Ring {
  head: AtomicUsize,
  tail: AtomicUsize,
  len: AtomicUsize,
}

impl Ring {
  pub const fn new() -> Self {
    Self {
      head: AtomicUsize::new(0),
      tail: AtomicUsize::new(0),
      len: AtomicUsize::new(0),
    }
  }

  pub fn len(&self) -> usize {
    self.len.load(Ordering::Relaxed)
  }

  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  pub fn is_full<T>(&self, buf: &[T]) -> bool {
    self.len() == buf.len()
  }

  fn next_idx(current: usize, capacity: usize) -> usize {
    (current + 1) % capacity
  }

  pub fn push<T>(&self, buf: &mut [T], val: T) -> Result<(), RingError<T>> {
    if self.is_full(buf) {
      return Err(RingError::Full(val));
    }

    let head = self.head.load(Ordering::Relaxed);
    buf[head] = val;
    self
      .head
      .store(Self::next_idx(head, buf.len()), Ordering::Relaxed);
    self.len.fetch_add(1, Ordering::Relaxed);

    Ok(())
  }

  pub fn pop<'a, T>(&self, buf: &'a [T]) -> Option<&'a T> {
    if self.is_empty() {
      return None;
    }

    let tail = self.tail.load(Ordering::Relaxed);
    let val = &buf[tail];
    self
      .tail
      .store(Self::next_idx(tail, buf.len()), Ordering::Relaxed);
    self.len.fetch_sub(1, Ordering::Relaxed);

    Some(val)
  }
}

#[cfg(test)]
mod tests;
