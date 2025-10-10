use core::{
  mem::ManuallyDrop,
  ptr::NonNull,
};

use basealloc_list::{
  HasLink,
  Link,
  List,
};
use basealloc_sys::{
  extent::{
    Extent,
    ExtentError,
  },
  math::align_up,
  prim::{
    PrimError,
    page_align,
  },
  system::SysOption,
};
use spin::Mutex;

use crate::config::CHUNK_SIZE;

pub static GLOBAL_BUMP: Mutex<Bump> = Mutex::new(Bump::new(CHUNK_SIZE));

#[derive(Debug)]
pub enum ChunkError {
  ExtentError(ExtentError),
  PrimError(PrimError),
  OutOfMemory,
  Overflow,
}

pub type ChunkResult<T> = Result<T, ChunkError>;

pub struct Chunk {
  link: Link<Self>,
  used: usize,
  extent: ManuallyDrop<Extent>,
}

impl Chunk {
  const SELF_SIZED: usize = core::mem::size_of::<Self>();
  const SELF_ALIGNED: usize = core::mem::align_of::<Self>();

  pub fn new(size: usize) -> ChunkResult<NonNull<Self>> {
    let mut extent = Extent::new(size, SysOption::Commit).map_err(ChunkError::ExtentError)?;
    let needed = align_up(Self::SELF_SIZED, Self::SELF_ALIGNED).ok_or(ChunkError::Overflow)?;
    let ps_aligned = page_align(needed).map_err(ChunkError::PrimError)?;
    if ps_aligned > size {
      return Err(ChunkError::OutOfMemory);
    }

    let used = ps_aligned;

    let ptr = extent.as_mut().as_mut_ptr() as *mut Self;
    unsafe {
      ptr.write(Self {
        extent: ManuallyDrop::new(extent),
        used,
        link: Link::default(),
      });
    }

    Ok(unsafe { NonNull::new_unchecked(ptr) })
  }

  fn as_ref(&self) -> &[u8] {
    self.extent.as_ref()
  }

  fn as_mut(&mut self) -> &mut [u8] {
    self.extent.as_mut()
  }

  pub fn allocate(&mut self, size: usize, align: usize) -> ChunkResult<&mut [u8]> {
    let start = align_up(self.used, align).ok_or(ChunkError::Overflow)?;
    let end = start.checked_add(size).ok_or(ChunkError::Overflow)?;

    if end > self.as_ref().len() {
      return Err(ChunkError::OutOfMemory);
    }

    self.used = end;
    let slice = &mut self.as_mut()[start..end];

    Ok(slice)
  }
}

impl Drop for Chunk {
  fn drop(&mut self) {
    unsafe {
      ManuallyDrop::drop(&mut self.extent);
    }
  }
}

impl HasLink for Chunk {
  fn link(&self) -> &Link<Self> {
    &self.link
  }

  fn link_mut(&mut self) -> &mut Link<Self> {
    &mut self.link
  }
}

pub type BumpError = ChunkError;
pub type BumpResult<T> = Result<T, BumpError>;

pub struct Bump {
  head: Option<NonNull<Chunk>>,
  tail: Option<NonNull<Chunk>>,
  chunk_size: usize,
}

impl Bump {
  pub const fn new(chunk_size: usize) -> Self {
    Self {
      head: None,
      tail: None,
      chunk_size,
    }
  }

  pub fn allocate(&mut self, size: usize, align: usize) -> BumpResult<&mut [u8]> {
    if let Some(mut tail) = self.tail {
      unsafe {
        if let Ok(slice) = tail.as_mut().allocate(size, align) {
          return Ok(slice);
        }
      }
    }

    let aligned_size = align_up(size, align).ok_or(ChunkError::Overflow)?;
    let chunk_size = core::cmp::max(self.chunk_size, aligned_size);
    let mut new_chunk = Chunk::new(chunk_size)?;

    let slice = unsafe { new_chunk.as_mut().allocate(size, align)? };

    if let Some(mut tail) = self.tail {
      unsafe { List::insert_after(new_chunk.as_mut(), tail.as_mut()) };
    } else {
      self.head = Some(new_chunk);
    }

    self.tail = Some(new_chunk);

    Ok(slice)
  }
}

impl Drop for Bump {
  fn drop(&mut self) {
    if let Some(mut head) = self.head {
      let _ = unsafe { List::drain(head.as_mut()) };
    }
  }
}

// SAFETY: Bump contains raw pointers, but they are only accessed through the mutex,
// ensuring thread safety. The chunks are heap-allocated and their ownership is
// managed by the Bump allocator itself.
unsafe impl Send for Bump {}
unsafe impl Sync for Bump {}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::config::CHUNK_SIZE;

  #[test]
  fn chunk_allocate() {
    let mut chunk = Chunk::new(CHUNK_SIZE).unwrap();
    let slice = unsafe { chunk.as_mut().allocate(64, 8).unwrap() };
    assert_eq!(slice.len(), 64);
    assert_eq!(slice.as_ptr() as usize % 8, 0);
  }

  #[test]
  fn chunk_out_of_memory() {
    let mut chunk = Chunk::new(CHUNK_SIZE).unwrap();
    let remaining = unsafe { chunk.as_ref().as_ref().len() - chunk.as_ref().used };
    let result = unsafe { chunk.as_mut().allocate(remaining + 1, 1) };
    assert!(matches!(result, Err(ChunkError::OutOfMemory)));
  }

  #[test]
  fn bump_new() {
    let bump = Bump::new(CHUNK_SIZE);
    assert_eq!(bump.chunk_size, CHUNK_SIZE);
  }

  #[test]
  fn bump_allocate() {
    let mut bump = Bump::new(CHUNK_SIZE);
    let slice = bump.allocate(64, 8).unwrap();
    assert_eq!(slice.len(), 64);
    assert_eq!(slice.as_ptr() as usize % 8, 0);
  }

  #[test]
  fn bump_multiple_chunks() {
    let mut bump = Bump::new(CHUNK_SIZE);
    let _slice1 = bump.allocate(CHUNK_SIZE / 2, 1).unwrap();
    let slice2 = bump.allocate(CHUNK_SIZE / 2, 1).unwrap();
    assert_eq!(slice2.len(), CHUNK_SIZE / 2);
  }

  #[test]
  fn bump_drop() {
    let bump = Bump::new(CHUNK_SIZE);
    drop(bump);
  }
}
