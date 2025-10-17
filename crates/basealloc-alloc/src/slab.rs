use core::{
  alloc::{
    Layout,
    LayoutError,
  },
  ptr::NonNull,
};

use basealloc_bitmap::{
  Bitmap,
  BitmapError,
  BitmapWord,
};
use basealloc_extent::{
  Extent,
  ExtentError,
};
use basealloc_fixed::bump::{
  Bump,
  BumpError,
};
use basealloc_list::{
  HasLink,
  Link,
};
use basealloc_sys::system::SysOption;
use getset::Getters;

use crate::classes::SizeClass;

#[derive(Debug)]
pub enum SlabError {
  BumpError(BumpError),
  ExtentError(ExtentError),
  LayoutError(LayoutError),
  BitmapError(BitmapError),
  OutOfMemory,
  InvalidPointer,
}

pub type SlabResult<T> = Result<T, SlabError>;

#[derive(Getters)]
pub struct Slab {
  class: SizeClass,
  extent: Extent,
  link: Link<Self>,
  bitmap: Bitmap,
  last: usize,
}

impl Slab {
  fn new_bitmap(bump: &mut Bump, regions: usize) -> SlabResult<Bitmap> {
    let bm_needed = Bitmap::bytes(regions);

    let bm_layout = Layout::from_size_align(bm_needed, core::mem::align_of::<BitmapWord>())
      .map_err(SlabError::LayoutError)?;

    let bm_raw = bump.allocate(bm_layout).map_err(SlabError::BumpError)?;
    let bm_slice = unsafe {
      core::slice::from_raw_parts_mut(bm_raw.as_ptr() as *mut BitmapWord, Bitmap::words(regions))
    };

    let bitmap = Bitmap::zero(bm_slice, regions).map_err(SlabError::BitmapError)?;

    Ok(bitmap)
  }

  pub fn new(bump: &mut Bump, class: SizeClass, size: usize) -> SlabResult<NonNull<Slab>> {
    let slab_ptr = bump.create::<Slab>().map_err(SlabError::BumpError)?;

    let extent = Extent::new(size, SysOption::Reserve).map_err(SlabError::ExtentError)?;

    let region_size = class.0;
    let regions = size / region_size;
    let bitmap = Self::new_bitmap(bump, regions)?;

    let tmp = Self {
      class,
      extent,
      link: Link::default(),
      bitmap,
      last: 0,
    };

    let slab = slab_ptr as *mut Slab;

    unsafe {
      core::ptr::write(slab, tmp);
    }

    Ok(unsafe { NonNull::new_unchecked(slab) })
  }

  fn update_last(&mut self, found: usize) {
    self.last = found % self.bitmap.bits();
  }

  fn ptr_at(&mut self, index: usize) -> NonNull<u8> {
    let offset = index * self.class.0;
    let eslice = self.extent.as_mut();
    let ptr = unsafe { eslice.as_mut_ptr().add(offset) };
    NonNull::new(ptr).unwrap()
  }

  fn has_ptr(&self, ptr: NonNull<u8>) -> bool {
    let base_ptr = self.extent.as_ref().as_ptr();
    let end_ptr = unsafe { base_ptr.add(self.extent.as_ref().len()) };
    let p = ptr.as_ptr() as *const u8;
    p >= base_ptr && p < end_ptr
  }

  fn index_for(&self, ptr: NonNull<u8>) -> Option<usize> {
    if !self.has_ptr(ptr) {
      return None;
    }

    let base_ptr = self.extent.as_ref().as_ptr() as *mut u8;
    let offset = unsafe { ptr.as_ptr().offset_from(base_ptr) as usize };
    Some(offset / self.class.0)
  }

  pub fn allocate(&mut self) -> SlabResult<NonNull<u8>> {
    let slot = self.bitmap.find_fc(Some(self.last));
    if let None = slot {
      return Err(SlabError::OutOfMemory);
    }

    let slot = slot.unwrap();
    self.bitmap.set(slot).map_err(SlabError::BitmapError)?;
    self.update_last(slot);
    Ok(self.ptr_at(slot))
  }

  pub fn deallocate(&mut self, ptr: NonNull<u8>) -> SlabResult<()> {
    if !self.has_ptr(ptr) {
      return Err(SlabError::InvalidPointer);
    }

    let index = self.index_for(ptr).unwrap();
    self.bitmap.clear(index).map_err(SlabError::BitmapError)?;
    self.update_last(index);
    Ok(())
  }
}

impl HasLink for Slab {
  fn link(&self) -> &Link<Self> {
    &self.link
  }

  fn link_mut(&mut self) -> &mut Link<Self> {
    &mut self.link
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    classes::{
      QUANTUM,
      SlabSize,
      class_at,
      class_for,
      pages_for,
    },
    static_::CHUNK_SIZE,
  };

  #[test]
  fn allocate_deallocate_reuse() {
    let mut bump = Bump::new(CHUNK_SIZE);
    let class_idx = class_for(QUANTUM).unwrap();
    let class = class_at(class_idx);
    let SlabSize(slab_size) = pages_for(class_idx);
    let mut slab_ptr = Slab::new(&mut bump, class, slab_size).expect("create slab");
    let slab = unsafe { slab_ptr.as_mut() };

    let p = slab.allocate().expect("alloc");
    assert!(slab.has_ptr(p));

    slab.deallocate(p).expect("dealloc");
    let p2 = slab.allocate().expect("alloc2");
    assert_eq!(p.as_ptr(), p2.as_ptr());
  }

  #[test]
  fn allocate_exhaustion_and_reuse() {
    let mut bump = Bump::new(CHUNK_SIZE);
    let class_idx = class_for(QUANTUM).unwrap();
    let class = class_at(class_idx);

    let SlabSize(slab_size) = pages_for(class_idx);
    let mut slab_ptr = Slab::new(&mut bump, class, slab_size).expect("create slab");
    let slab = unsafe { slab_ptr.as_mut() };

    let mut slots = Vec::new();
    let regions = slab_size / class.0;
    for _ in 0..regions {
      slots.push(slab.allocate().expect("alloc"));
    }

    match slab.allocate() {
      Err(SlabError::OutOfMemory) => {}
      other => panic!("expected OutOfMemory, got {:?}", other),
    }

    slab.deallocate(slots[regions - 1]).expect("dealloc");
    let p = slab.allocate().expect("alloc after free");
    assert!(slab.has_ptr(p));
  }
}
