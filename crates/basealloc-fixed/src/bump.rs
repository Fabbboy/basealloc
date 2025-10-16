use core::{
  alloc::Layout,
  cmp,
  mem::ManuallyDrop,
  ptr::{
    NonNull,
    drop_in_place,
  },
};

use crate::fixed::{
  Fixed,
  FixedError,
};

use basealloc_extent::{Extent, ExtentError};
use basealloc_list::{
  HasLink,
  Link,
  List,
};
use basealloc_sys::{
 
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
  link: ManuallyDrop<Link<Self>>,
  fixed: ManuallyDrop<Fixed>,
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
    chunk.link = ManuallyDrop::new(Link::default());
    chunk.fixed = ManuallyDrop::new(fixed);
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
      ManuallyDrop::drop(&mut self.fixed);
      ManuallyDrop::drop(&mut self.link);

      // SAFETY: MUST BE DROPPED LAST
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

  fn obtain_chunk(&self, layout: Layout) -> BumpResult<NonNull<Chunk>> {
    let header = Chunk::data_offset()?;
    let required = header
      .checked_add(layout.size())
      .ok_or(ChunkError::Overflow)?;
    let chunk_size = cmp::max(self.chunk_size, required);
    let chunk_size = page_align(chunk_size).map_err(ChunkError::PrimError)?;
    Chunk::new(chunk_size)
  }

  pub fn create<T>(&mut self) -> BumpResult<*mut T> {
    let layout = Layout::new::<T>();
    let bytes = self.allocate(layout)?;
    let ptr = bytes.as_mut_ptr() as *mut T;
    unsafe { ptr.write(core::mem::zeroed()) };
    Ok(ptr)
  }

  pub fn allocate(&mut self, layout: Layout) -> BumpResult<&mut [u8]> {
    if let Some(mut tail) = self.tail {
      if let Ok(slice) = unsafe { tail.as_mut().allocate(layout) } {
        return Ok(slice);
      }
    }

    let mut new_chunk = self.obtain_chunk(layout)?;

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
    let as_ref = |mut ptr: NonNull<Chunk>| unsafe { ptr.as_mut() };

    while let Some(ptr) = current {
      let chunk_ref = as_ref(ptr);
      List::remove(chunk_ref);
      current = *chunk_ref.link().next();
      unsafe { drop_in_place(chunk_ref) };
    }
  }
}
