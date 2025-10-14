use core::alloc::Layout;

use basealloc_sys::math::align_up;

#[derive(Debug)]
pub enum FixedError {
  OOM,
  Invalid,
}

pub type FixedResult<T> = Result<T, FixedError>;

pub struct Fixed {
  max: usize,
  offset: usize,
}

impl Fixed {
  pub fn new(slice: &[u8]) -> Self {
    let max = slice.len();
    Self { max, offset: 0 }
  }

  fn has(&self, needed: usize) -> bool {
    self.max - self.offset >= needed
  }

  fn required(&self, layout: Layout) -> FixedResult<usize> {
    let align = layout.align();
    let size = layout.size();
    align_up(size, align).ok_or(FixedError::Invalid)
  }

  pub fn allocate<'slice>(
    &mut self,
    slice: &'slice mut [u8],
    layout: Layout,
  ) -> FixedResult<&'slice mut [u8]> {
    let required = self.required(layout)?;
    if !self.has(required) {
      return Err(FixedError::OOM);
    }

    let start = align_up(slice.as_ptr() as usize + self.offset, layout.align())
      .ok_or(FixedError::Invalid)?
      - slice.as_ptr() as usize;
    let end = start.checked_add(required).ok_or(FixedError::Invalid)?;
    if end > slice.len() {
      return Err(FixedError::OOM);
    }

    self.offset = end;
    Ok(&mut slice[start..end])
  }

  pub fn create<'slice, T>(&mut self, slice: &'slice mut [u8]) -> FixedResult<*mut T> {
    let layout = Layout::new::<T>();
    let bytes = self.allocate(slice, layout)?;
    if bytes.len() < layout.size() {
      return Err(FixedError::Invalid);
    }

    Ok(bytes.as_mut_ptr() as *mut T)
  }
}
