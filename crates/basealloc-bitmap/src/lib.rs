#![cfg_attr(not(test), no_std)]

use core::sync::atomic::{
  AtomicUsize,
  Ordering,
};

#[cfg(test)]
pub mod tests;

#[derive(Debug)]
pub enum BitmapError {
  InsufficientSize { have: usize, need: usize },
  OutOfBounds { index: usize, size: usize },
}

pub type BitmapWord = AtomicUsize;

const USIZE_BITS: usize = usize::BITS as usize;

#[derive(Debug)]
pub struct Bitmap<'slice> {
  store: &'slice [BitmapWord],
  bits: usize,
  used: AtomicUsize,
}

impl<'slice> Bitmap<'slice> {
  #[inline(always)]
  pub const fn words(fields: usize) -> usize {
    fields.div_ceil(USIZE_BITS)
  }

  #[inline(always)]
  pub const fn bytes(fields: usize) -> usize {
    Self::words(fields) * core::mem::size_of::<BitmapWord>()
  }

  #[inline(always)]
  pub const fn available(&self) -> usize {
    self.store.len() * USIZE_BITS
  }

  #[inline(always)]
  pub const fn store(&self) -> &[BitmapWord] {
    self.store
  }

  #[inline(always)]
  pub const fn bits(&self) -> usize {
    self.bits
  }

  const fn position(&self, index: usize) -> Result<(usize, usize), BitmapError> {
    if index >= self.bits {
      return Err(BitmapError::OutOfBounds {
        index,
        size: self.bits,
      });
    }
    let word_index = index / USIZE_BITS;
    let bit_index = index % USIZE_BITS;
    Ok((word_index, bit_index))
  }

  pub fn zero(store: &'slice [BitmapWord], bits: usize) -> Result<Self, BitmapError> {
    let available = store.len() * USIZE_BITS;
    if bits > available {
      return Err(BitmapError::InsufficientSize {
        have: available,
        need: bits,
      });
    }

    let bitmap = Self {
      store,
      bits,
      used: AtomicUsize::new(0),
    };
    bitmap.clear_all();
    Ok(bitmap)
  }

  pub fn one(store: &'slice [BitmapWord], bits: usize) -> Result<Self, BitmapError> {
    let available = store.len() * USIZE_BITS;
    if bits > available {
      return Err(BitmapError::InsufficientSize {
        have: available,
        need: bits,
      });
    }

    let bitmap = Self {
      store,
      bits,
      used: AtomicUsize::new(0),
    };
    bitmap.set_all();
    Ok(bitmap)
  }

  pub const fn check(&self, fields: usize) -> Result<(), BitmapError> {
    let total_bits = self.store.len() * USIZE_BITS;
    if fields > total_bits {
      return Err(BitmapError::InsufficientSize {
        have: total_bits,
        need: fields,
      });
    }
    Ok(())
  }

  #[inline]
  pub fn set(&self, index: usize) -> Result<(), BitmapError> {
    let (word_index, bit_index) = self.position(index)?;
    let mask = 1usize << bit_index;
    self.store[word_index].fetch_or(mask, Ordering::Relaxed);
    self.used.fetch_add(1, Ordering::Relaxed);
    Ok(())
  }

  #[inline]
  pub fn clear(&self, index: usize) -> Result<(), BitmapError> {
    let (word_index, bit_index) = self.position(index)?;
    let mask = !(1usize << bit_index);
    self.store[word_index].fetch_and(mask, Ordering::Relaxed);
    self.used.fetch_sub(1, Ordering::Relaxed);
    Ok(())
  }

  #[inline]
  pub fn get(&self, index: usize) -> Result<bool, BitmapError> {
    let (word_index, bit_index) = self.position(index)?;
    let value = self.store[word_index].load(Ordering::Relaxed);
    Ok((value & (1usize << bit_index)) != 0)
  }

  pub fn clear_all(&self) {
    for word in self.store.iter() {
      word.store(0, Ordering::Relaxed);
    }
    self.used.store(0, Ordering::Relaxed);
  }

  pub fn set_all(&self) {
    let full_words = self.bits / USIZE_BITS;

    for word in self.store[..full_words].iter() {
      word.store(usize::MAX, Ordering::Relaxed);
    }

    let remaining_bits = self.bits % USIZE_BITS;
    if remaining_bits > 0 && full_words < self.store.len() {
      let mask = usize::MAX >> (USIZE_BITS - remaining_bits);
      self.store[full_words].store(mask, Ordering::Relaxed);
    }
    self.used.store(self.bits, Ordering::Relaxed);
  }

  pub fn find_fs(&self) -> Option<usize> {
    for (word_index, word) in self.store.iter().enumerate() {
      let value = word.load(Ordering::Relaxed);
      if value != 0 {
        let bit_offset = value.trailing_zeros() as usize;
        let global_index = word_index * USIZE_BITS + bit_offset;
        if global_index < self.bits {
          return Some(global_index);
        }
      }
    }
    None
  }

  pub fn find_fc(&self) -> Option<usize> {
    for (word_index, word) in self.store.iter().enumerate() {
      let value = word.load(Ordering::Relaxed);
      let inverted = value ^ usize::MAX;
      if inverted != 0 {
        let bit_offset = inverted.trailing_zeros() as usize;
        let global_index = word_index * USIZE_BITS + bit_offset;
        if global_index < self.bits {
          return Some(global_index);
        }
      }
    }
    None
  }

  #[inline]
  pub fn is_clear(&self) -> bool {
    self.used.load(Ordering::Relaxed) == 0
  }

  #[inline]
  pub fn one_clear(&self) -> bool {
    self.used.load(Ordering::Relaxed) < self.bits
  }
}
