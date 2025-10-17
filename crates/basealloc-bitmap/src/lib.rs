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

#[inline(always)]
const fn word_index(bit: usize) -> usize {
  bit / USIZE_BITS
}

#[inline(always)]
const fn bit_index(bit: usize) -> usize {
  bit % USIZE_BITS
}

#[inline(always)]
const fn bit_mask(bit: usize) -> usize {
  1usize << bit_index(bit)
}

#[inline(always)]
const fn mask_from(start_bit: usize) -> usize {
  usize::MAX << start_bit
}

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
    Ok((word_index(index), bit_index(index)))
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
    self.position(index)?;
    let store = self.store.as_slice();
    store[word_index(index)].fetch_or(bit_mask(index), Ordering::Relaxed);
    self.used.fetch_add(1, Ordering::Relaxed);
    Ok(())
  }

  #[inline]
  pub fn clear(&self, index: usize) -> Result<(), BitmapError> {
    self.position(index)?;
    let store = self.store.as_slice();
    store[word_index(index)].fetch_and(!bit_mask(index), Ordering::Relaxed);
    self.used.fetch_sub(1, Ordering::Relaxed);
    Ok(())
  }

  #[inline]
  pub fn get(&self, index: usize) -> Result<bool, BitmapError> {
    self.position(index)?;
    let store = self.store.as_slice();
    let value = store[word_index(index)].load(Ordering::Relaxed);
    Ok((value & bit_mask(index)) != 0)
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

  fn iter_range<F>(&self, from_word: usize, to_word: usize, start_mask: usize, end_mask: usize, transform: F) -> Option<usize>
  where
    F: Fn(usize) -> usize,
  {
    let store = self.store.as_slice();

    for idx in from_word..=to_word {
      if idx >= store.len() {
        break;
      }

      let value = transform(store[idx].load(Ordering::Relaxed));
      let mask = if idx == from_word {
        start_mask
      } else if idx == to_word {
        end_mask
      } else {
        usize::MAX
      };

      let masked = value & mask;
      if masked != 0 {
        let bit_offset = masked.trailing_zeros() as usize;
        let global_index = idx * USIZE_BITS + bit_offset;
        if global_index < self.bits {
          return Some(global_index);
        }
      }
    }
    None
  }

  pub fn find_fs(&self, start: Option<usize>) -> Option<usize> {
    let start_bit = start.unwrap_or(0);
    if start_bit >= self.bits {
      return None;
    }

    let start_word = word_index(start_bit);
    let start_offset = bit_index(start_bit);
    let last_word = word_index(self.bits.saturating_sub(1));

    if let Some(result) = self.iter_range(start_word, last_word, mask_from(start_offset), usize::MAX, |v| v) {
      return Some(result);
    }

    if start_bit > 0 && start_word > 0 {
      let end_mask = (1usize << start_offset) - 1;
      self.iter_range(0, start_word - 1, usize::MAX, end_mask, |v| v)
    } else {
      None
    }
  }

  pub fn find_fc(&self, start: Option<usize>) -> Option<usize> {
    let start_bit = start.unwrap_or(0);
    if start_bit >= self.bits {
      return None;
    }

    let start_word = word_index(start_bit);
    let start_offset = bit_index(start_bit);
    let last_word = word_index(self.bits.saturating_sub(1));

    if let Some(result) = self.iter_range(start_word, last_word, mask_from(start_offset), usize::MAX, |v| v ^ usize::MAX) {
      return Some(result);
    }

    if start_bit > 0 && start_word > 0 {
      let end_mask = (1usize << start_offset) - 1;
      self.iter_range(0, start_word - 1, usize::MAX, end_mask, |v| v ^ usize::MAX)
    } else {
      None
    }
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
