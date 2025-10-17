use core::ptr::NonNull;

pub trait Giveup {
  type Failure: Default;

  fn giveup(self) -> Result<Self, Self::Failure>
  where
    Self: Sized;
}

pub struct UnsafeStore<T> {
  ptr: NonNull<T>,
  len: usize,
}

impl<T> From<&[T]> for UnsafeStore<T> {
  fn from(slice: &[T]) -> Self {
    Self {
      ptr: NonNull::new(slice.as_ptr() as *mut T).unwrap(),
      len: slice.len(),
    }
  }
}

impl<T> UnsafeStore<T> {
  pub fn as_slice(&self) -> &[T] {
    unsafe { core::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
  }

  pub fn as_mut_slice(&mut self) -> &mut [T] {
    unsafe { core::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
  }
}
