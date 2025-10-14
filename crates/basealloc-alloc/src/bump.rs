use core::{
  alloc::Layout,
  cmp,
  mem::ManuallyDrop,
  ptr::{
    NonNull,
    drop_in_place,
  },
};

use basealloc_fixed::{
  Fixed,
  FixedError,
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

#[derive(Debug)]
pub enum ChunkError {
  ExtentError(ExtentError),
  PrimError(PrimError),
  FixedError(FixedError),
  Overflow, // fucking keep!!!
}

pub type ChunkResult<T> = Result<T, ChunkError>;

pub struct Chunk {
  link: Link<Self>,
  fixed: Fixed,
  extent: ManuallyDrop<Extent>,
}

impl Chunk {
  const SELF_LAYOUT: Layout = Layout::new::<Self>();

  fn data_offset() -> ChunkResult<usize> {
    let size = Self::SELF_LAYOUT.size();
    let align = Self::SELF_LAYOUT.align();
    let aligned = align_up(size, align).ok_or(ChunkError::PrimError(PrimError::Overflow))?;
    page_align(aligned).map_err(ChunkError::PrimError)
  }

  pub fn new(size: usize) -> ChunkResult<NonNull<Self>> {
    let mut extent = Extent::new(size, SysOption::Commit).map_err(ChunkError::ExtentError)?;
    let mut fixed = Fixed::new(extent.as_mut());
    let chunk_ptr = fixed
      .create::<Self>(extent.as_mut())
      .map_err(ChunkError::FixedError)?;

    let chunk = unsafe { &mut *chunk_ptr };
    chunk.link = Link::default();
    chunk.fixed = fixed;
    chunk.extent = ManuallyDrop::new(extent);

    Ok(unsafe { NonNull::new_unchecked(chunk_ptr) })
  }

  pub fn create<T>(&mut self) -> ChunkResult<&mut T> {
    let slot = self.allocate(Layout::new::<T>())?;
    let ptr = slot.as_mut_ptr() as *mut T;
    unsafe { ptr.write(core::mem::zeroed()) };
    Ok(unsafe { &mut *ptr })
  }

  pub fn allocate(&mut self, layout: Layout) -> ChunkResult<&mut [u8]> {
    let extent_slice = self.extent.as_mut();
    self
      .fixed
      .allocate(extent_slice, layout)
      .map_err(ChunkError::FixedError)
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

  pub fn create<T>(&mut self) -> BumpResult<&mut T> {
    let layout = Layout::new::<T>();
    let bytes = self.allocate(layout)?;
    let ptr = bytes.as_mut_ptr() as *mut T;
    unsafe { ptr.write(core::mem::zeroed()) };
    Ok(unsafe { &mut *ptr })
  }

  pub fn allocate(&mut self, layout: Layout) -> BumpResult<&mut [u8]> {
    if let Some(mut tail) = self.tail {
      unsafe {
        match tail.as_mut().allocate(layout) {
          Ok(slice) => return Ok(slice),
          Err(err) => return Err(err),
        }
      }
    }

    let header = Chunk::data_offset()?;
    let aligned = align_up(layout.size(), layout.align()).ok_or(ChunkError::Overflow)?;
    let required = header.checked_add(aligned).ok_or(ChunkError::Overflow)?;
    let chunk_size = cmp::max(self.chunk_size, required);
    let chunk_size = page_align(chunk_size).map_err(ChunkError::PrimError)?;

    let mut new_chunk = Chunk::new(chunk_size)?;

    if let Some(mut tail) = self.tail {
      unsafe { List::insert_after(new_chunk.as_mut(), tail.as_mut()) };
    } else {
      self.head = Some(new_chunk);
    }

    self.tail = Some(new_chunk);

    unsafe { new_chunk.as_mut().allocate(layout) }
  }
}

impl Drop for Bump {
  fn drop(&mut self) {
    let mut current = self.head;
    while let Some(mut ptr) = current {
      unsafe {
        let chunk = ptr.as_mut();
        current = *chunk.link().next();
        //drop_in_place(chunk);
      }
    }
    self.head = None;
    self.tail = None;
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::config::CHUNK_SIZE;

  #[test]
  fn chunk_allocate() {
    let mut chunk = Chunk::new(CHUNK_SIZE).unwrap();
    {
      let layout = Layout::from_size_align(64, 8).unwrap();
      let slice = unsafe { chunk.as_mut().allocate(layout).unwrap() };
      assert_eq!(slice.len(), 64);
      assert_eq!(slice.as_ptr() as usize % 8, 0);
    }
  }

  #[test]
  fn chunk_out_of_memory() {
    let mut chunk = Chunk::new(CHUNK_SIZE).unwrap();
    let oversized = Layout::from_size_align(CHUNK_SIZE, 1).unwrap();
    let result = unsafe { chunk.as_mut().allocate(oversized) };
    assert!(matches!(
      result,
      Err(ChunkError::FixedError(FixedError::OOM))
    ));
  }

  #[test]
  fn bump_new() {
    let bump = Bump::new(CHUNK_SIZE);
    assert_eq!(bump.chunk_size, CHUNK_SIZE);
  }

  #[test]
  fn bump_allocate() {
    let mut bump = Bump::new(CHUNK_SIZE);
    {
      let layout = Layout::from_size_align(64, 8).unwrap();
      let slice = bump.allocate(layout).unwrap();
      assert_eq!(slice.len(), 64);
      assert_eq!(slice.as_ptr() as usize % 8, 0);
    }
  }

  #[test]
  fn bump_multiple_chunks() {
    let mut bump = Bump::new(CHUNK_SIZE);
    let half = Layout::from_size_align(CHUNK_SIZE / 2, 1).unwrap();
    let _slice1 = bump.allocate(half).unwrap();
    {
      let slice2 = bump.allocate(half).unwrap();
      assert_eq!(slice2.len(), CHUNK_SIZE / 2);
    }
  }

  #[test]
  fn bump_drop() {
    let bump = Bump::new(CHUNK_SIZE);
    drop(bump);
  }
}
