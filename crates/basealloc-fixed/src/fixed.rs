use core::{
  alloc::Layout,
  sync::atomic::{
    AtomicUsize,
    Ordering,
  },
};

use basealloc_sys::math::align_up;

#[derive(Debug)]
pub enum FixedError {
  OutOfMemory,
  Invalid,
}

pub type FixedResult<T> = Result<T, FixedError>;

pub struct Fixed {
  max: usize,
  offset: AtomicUsize,
}

impl Fixed {
  pub fn new(slice: &[u8]) -> Self {
    let max = slice.len();
    Self {
      max,
      offset: AtomicUsize::new(0),
    }
  }

  fn has(&self, needed: usize) -> bool {
    let cur = self.offset.load(Ordering::Acquire);
    if self.max < needed {
      return false;
    }
    cur <= self.max - needed
  }

  fn required(&self, layout: Layout) -> FixedResult<usize> {
    Ok(layout.size())
  }

  fn start_offset(&self, slice_ptr: usize, current: usize, align: usize) -> FixedResult<usize> {
    let base_plus = slice_ptr.checked_add(current).ok_or(FixedError::Invalid)?;
    let aligned = align_up(base_plus, align).ok_or(FixedError::Invalid)?;
    let start = aligned.checked_sub(slice_ptr).ok_or(FixedError::Invalid)?;
    Ok(start)
  }

  fn end_range(&self, start: usize, required: usize) -> FixedResult<usize> {
    let end = start.checked_add(required).ok_or(FixedError::Invalid)?;
    Ok(end)
  }

  fn reserve(&self, slice_ptr: usize, required: usize, align: usize) -> FixedResult<usize> {
    loop {
      let current = self.offset.load(Ordering::Acquire);

      let start = self.start_offset(slice_ptr, current, align)?;
      let end = self.end_range(start, required)?;

      if end > self.max {
        return Err(FixedError::OutOfMemory);
      }

      match self
        .offset
        .compare_exchange(current, end, Ordering::AcqRel, Ordering::Acquire)
      {
        Ok(_) => return Ok(start),
        Err(_) => {
          continue;
        }
      }
    }
  }

  pub fn allocate<'slice>(
    &mut self,
    slice: &'slice mut [u8],
    layout: Layout,
  ) -> FixedResult<&'slice mut [u8]> {
    let required = self.required(layout)?;
    if !self.has(required) {
      return Err(FixedError::OutOfMemory);
    }

    let slice_ptr = slice.as_ptr() as usize;
    let start = self.reserve(slice_ptr, required, layout.align())?;
    let end = start.checked_add(required).ok_or(FixedError::Invalid)?;
    if end > slice.len() {
      return Err(FixedError::OutOfMemory);
    }

    Ok(&mut slice[start..end])
  }

  pub fn create<T>(&mut self, slice: &mut [u8]) -> FixedResult<*mut T> {
    let layout = Layout::new::<T>();
    let bytes = self.allocate(slice, layout)?;
    if bytes.len() < layout.size() {
      return Err(FixedError::Invalid);
    }

    Ok(bytes.as_mut_ptr() as *mut T)
  }
}

unsafe impl Send for Fixed {}
unsafe impl Sync for Fixed {}
