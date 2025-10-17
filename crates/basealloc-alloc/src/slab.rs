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
  BmStore,
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

use crate::classes::SizeClass;

pub enum SlabError {
  BumpError(BumpError),
  ExtentError(ExtentError),
  LayoutError(LayoutError),
  BitmapError(BitmapError),
}

pub type SlabResult<T> = Result<T, SlabError>;

pub struct Slab {
  class: SizeClass,
  extent: Extent,
  link: Link<Self>,
  bitmap: Bitmap,
  store: BmStore,
}

impl Slab {
  fn new_bitmap(bump: &mut Bump, regions: usize) -> SlabResult<(Bitmap, BmStore)> {
    let bm_needed = Bitmap::bytes(regions);

    let bm_layout = Layout::from_size_align(bm_needed, core::mem::align_of::<BitmapWord>())
      .map_err(SlabError::LayoutError)?;

    let bm_raw = bump.allocate(bm_layout).map_err(SlabError::BumpError)?;
    let bm_slice = unsafe {
      core::slice::from_raw_parts_mut(bm_raw.as_ptr() as *mut BitmapWord, Bitmap::words(regions))
    };

    let store = BmStore::from(&*bm_slice);
    let bitmap = Bitmap::zero(store.as_slice(), regions).map_err(SlabError::BitmapError)?;

    Ok((bitmap, store))
  }

  pub fn new(bump: &mut Bump, class: SizeClass, size: usize) -> SlabResult<NonNull<Slab>> {
    let slab = bump.create::<Slab>().map_err(SlabError::BumpError)?;

    let extent = Extent::new(size, SysOption::Reserve).map_err(SlabError::ExtentError)?;

    let region_size = class.0;
    let regions = size / region_size;
    let (bitmap, store) = Self::new_bitmap(bump, regions)?;

    let tmp = Self {
      class,
      extent,
      link: Link::default(),
      bitmap,
      store,
    };

    unsafe {
      slab.write(tmp);
    }
    Ok(unsafe { NonNull::new_unchecked(slab) })
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
