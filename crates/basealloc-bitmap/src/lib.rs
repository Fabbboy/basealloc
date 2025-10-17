#![cfg_attr(not(test), no_std)]

use core::{
  ptr::NonNull,
  sync::atomic::{
    AtomicUsize,
    Ordering,
  },
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
pub struct BmStore {
  ptr: NonNull<BitmapWord>,
  len: usize,
}

impl BmStore {
  #[inline(always)]
  pub fn as_slice(&self) -> &[BitmapWord] {
    unsafe { core::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
  }
}

impl From<&[BitmapWord]> for BmStore {
  fn from(slice: &[BitmapWord]) -> Self {
    Self {
      ptr: NonNull::new(slice.as_ptr() as *mut BitmapWord).unwrap(),
      len: slice.len(),
    }
  }
}

unsafe impl Send for BmStore {}
unsafe impl Sync for BmStore {}

#[derive(Debug)]
pub struct Bitmap {
  store: BmStore,
  bits: usize,
  used: AtomicUsize,
}

impl Bitmap {
  #[inline(always)]
  pub const fn words(fields: usize) -> usize {
    fields.div_ceil(USIZE_BITS)
  }

  #[inline(always)]
  pub const fn bytes(fields: usize) -> usize {
    Self::words(fields) * core::mem::size_of::<BitmapWord>()
  }

  #[inline(always)]
  pub fn available(&self) -> usize {
    self.store.as_slice().len() * USIZE_BITS
  }

  #[inline(always)]
  pub fn store(&self) -> &[BitmapWord] {
    self.store.as_slice()
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

  pub fn zero(store: &[BitmapWord], bits: usize) -> Result<Self, BitmapError> {
    let available = store.len() * USIZE_BITS;
    if bits > available {
      return Err(BitmapError::InsufficientSize {
        have: available,
        need: bits,
      });
    }

    let bitmap = Self {
      store: BmStore::from(store),
      bits,
      used: AtomicUsize::new(0),
    };
    bitmap.clear_all();
    Ok(bitmap)
  }

  pub fn one(store: &[BitmapWord], bits: usize) -> Result<Self, BitmapError> {
    let available = store.len() * USIZE_BITS;
    if bits > available {
      return Err(BitmapError::InsufficientSize {
        have: available,
        need: bits,
      });
    }

    let bitmap = Self {
      store: BmStore::from(store),
      bits,
      used: AtomicUsize::new(0),
    };
    bitmap.set_all();
    Ok(bitmap)
  }

  pub fn check(&self, fields: usize) -> Result<(), BitmapError> {
    let total_bits = self.store.as_slice().len() * USIZE_BITS;
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
    let store = self.store.as_slice();
    store[word_index].fetch_or(mask, Ordering::Relaxed);
    self.used.fetch_add(1, Ordering::Relaxed);
    Ok(())
  }

  #[inline]
  pub fn clear(&self, index: usize) -> Result<(), BitmapError> {
    let (word_index, bit_index) = self.position(index)?;
    let mask = !(1usize << bit_index);
    let store = self.store.as_slice();
    store[word_index].fetch_and(mask, Ordering::Relaxed);
    self.used.fetch_sub(1, Ordering::Relaxed);
    Ok(())
  }

  #[inline]
  pub fn get(&self, index: usize) -> Result<bool, BitmapError> {
    let (word_index, bit_index) = self.position(index)?;
    let store = self.store.as_slice();
    let value = store[word_index].load(Ordering::Relaxed);
    Ok((value & (1usize << bit_index)) != 0)
  }

  pub fn clear_all(&self) {
    let store = self.store.as_slice();
    for word in store.iter() {
      word.store(0, Ordering::Relaxed);
    }
    self.used.store(0, Ordering::Relaxed);
  }

  pub fn set_all(&self) {
    let store = self.store.as_slice();
    let full_words = self.bits / USIZE_BITS;

    for word in store[..full_words].iter() {
      word.store(usize::MAX, Ordering::Relaxed);
    }

    let remaining_bits = self.bits % USIZE_BITS;
    if remaining_bits > 0 && full_words < store.len() {
      let mask = usize::MAX >> (USIZE_BITS - remaining_bits);
      store[full_words].store(mask, Ordering::Relaxed);
    }
    self.used.store(self.bits, Ordering::Relaxed);
  }

  pub fn find_fs(&self) -> Option<usize> {
    let store = self.store.as_slice();
    for (word_index, word) in store.iter().enumerate() {
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
    let store = self.store.as_slice();
    for (word_index, word) in store.iter().enumerate() {
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
