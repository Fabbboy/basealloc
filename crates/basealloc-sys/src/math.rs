use core::ptr::NonNull;

pub const fn is_aligned(value: usize, align: usize) -> Option<bool> {
  if !align.is_power_of_two() {
    return None;
  }
  Some((value & (align - 1)) == 0)
}

pub const fn align_up(value: usize, align: usize) -> Option<usize> {
  if !align.is_power_of_two() {
    return None;
  }

  let mask = align - 1;
  if let Some(sum) = value.checked_add(mask) {
    return Some(sum & !mask);
  }

  None
}

pub const fn align_down(value: usize, align: usize) -> Option<usize> {
  if !align.is_power_of_two() {
    return None;
  }

  Some(value & !(align - 1))
}

pub fn align_ptr<T>(ptr: NonNull<T>, align: usize) -> Option<NonNull<T>> {
  let addr = ptr.as_ptr() as usize;
  let aligned_addr = align_up(addr, align)?;
  NonNull::new(aligned_addr as *mut T)
}

pub fn align_mut_ptr<T>(ptr: NonNull<T>, align: usize) -> Option<NonNull<T>> {
  let addr = ptr.as_ptr() as usize;
  let aligned_addr = align_up(addr, align)?;
  NonNull::new(aligned_addr as *mut T)
}

pub fn align_offset(addr: usize, align: usize) -> Option<usize> {
  if !align.is_power_of_two() {
    return None;
  }

  let aligned_addr = align_up(addr, align)?;
  Some(aligned_addr - addr)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_is_aligned() {
    assert_eq!(is_aligned(0, 1), Some(true));
    assert_eq!(is_aligned(0, 2), Some(true));
    assert_eq!(is_aligned(0, 4), Some(true));
    assert_eq!(is_aligned(0, 8), Some(true));

    assert_eq!(is_aligned(1, 1), Some(true));
    assert_eq!(is_aligned(1, 2), Some(false));
    assert_eq!(is_aligned(1, 4), Some(false));

    assert_eq!(is_aligned(2, 2), Some(true));
    assert_eq!(is_aligned(2, 4), Some(false));

    assert_eq!(is_aligned(4, 4), Some(true));
    assert_eq!(is_aligned(4, 8), Some(false));

    assert_eq!(is_aligned(8, 8), Some(true));

    assert_eq!(is_aligned(16, 16), Some(true));
    assert_eq!(is_aligned(15, 16), Some(false));
    assert_eq!(is_aligned(17, 16), Some(false));

    assert_eq!(is_aligned(100, 3), None);
    assert_eq!(is_aligned(100, 5), None);
    assert_eq!(is_aligned(100, 6), None);
  }

  #[test]
  fn test_align_up() {
    assert_eq!(align_up(0, 1), Some(0));
    assert_eq!(align_up(0, 2), Some(0));
    assert_eq!(align_up(0, 4), Some(0));
    assert_eq!(align_up(0, 8), Some(0));

    assert_eq!(align_up(1, 1), Some(1));
    assert_eq!(align_up(1, 2), Some(2));
    assert_eq!(align_up(1, 4), Some(4));
    assert_eq!(align_up(1, 8), Some(8));

    assert_eq!(align_up(2, 2), Some(2));
    assert_eq!(align_up(3, 2), Some(4));
    assert_eq!(align_up(2, 4), Some(4));
    assert_eq!(align_up(3, 4), Some(4));
    assert_eq!(align_up(4, 4), Some(4));
    assert_eq!(align_up(5, 4), Some(8));

    assert_eq!(align_up(7, 8), Some(8));
    assert_eq!(align_up(8, 8), Some(8));
    assert_eq!(align_up(9, 8), Some(16));

    assert_eq!(align_up(15, 16), Some(16));
    assert_eq!(align_up(16, 16), Some(16));
    assert_eq!(align_up(17, 16), Some(32));

    assert_eq!(align_up(100, 3), None);
    assert_eq!(align_up(100, 5), None);
    assert_eq!(align_up(100, 6), None);
  }

  #[test]
  fn test_align_ptr() {
    let data = [0u8; 32];
    let ptr = NonNull::new(data.as_ptr() as *mut u8).unwrap();

    assert!(align_ptr(ptr, 1).is_some());
    assert!(align_ptr(ptr, 2).is_some());
    assert!(align_ptr(ptr, 4).is_some());
    assert!(align_ptr(ptr, 8).is_some());

    assert!(align_ptr(ptr, 3).is_none());
    assert!(align_ptr(ptr, 5).is_none());

    let aligned = align_ptr(ptr, 8).unwrap();
    assert!(is_aligned(aligned.as_ptr() as usize, 8) == Some(true));
    assert!(aligned.as_ptr() >= ptr.as_ptr());
  }

  #[test]
  fn test_align_down() {
    assert_eq!(align_down(0, 8), Some(0));
    assert_eq!(align_down(7, 8), Some(0));
    assert_eq!(align_down(8, 8), Some(8));
    assert_eq!(align_down(15, 8), Some(8));
    assert_eq!(align_down(16, 8), Some(16));

    assert_eq!(align_down(123, 64), Some(64));
    assert_eq!(align_down(256, 64), Some(256));

    assert_eq!(align_down(100, 3), None);
  }

  #[test]
  fn test_align_mut_ptr() {
    let mut data = [0u8; 32];
    let ptr = NonNull::new(data.as_mut_ptr()).unwrap();

    assert!(align_mut_ptr(ptr, 1).is_some());
    assert!(align_mut_ptr(ptr, 2).is_some());
    assert!(align_mut_ptr(ptr, 4).is_some());
    assert!(align_mut_ptr(ptr, 8).is_some());

    assert!(align_mut_ptr(ptr, 3).is_none());
    assert!(align_mut_ptr(ptr, 5).is_none());

    let aligned = align_mut_ptr(ptr, 8).unwrap();
    assert!(is_aligned(aligned.as_ptr() as usize, 8) == Some(true));
    assert!(aligned.as_ptr() >= ptr.as_ptr());
  }

  #[test]
  fn test_alignment_edge_cases() {
    assert_eq!(align_up(usize::MAX - 6, 8), None);
    assert_eq!(align_up(usize::MAX, 8), None);

    let max_align = 1usize << (usize::BITS - 1);
    assert_eq!(is_aligned(0, max_align), Some(true));
    assert_eq!(is_aligned(1, max_align), Some(false));
    assert_eq!(align_up(0, max_align), Some(0));
    assert_eq!(align_up(1, max_align), Some(max_align));
    assert_eq!(align_up(max_align + 1, max_align), None);
  }

  #[test]
  fn test_align_offset() {
    assert_eq!(align_offset(0, 8), Some(0));
    assert_eq!(align_offset(1, 8), Some(7));
    assert_eq!(align_offset(7, 8), Some(1));
    assert_eq!(align_offset(8, 8), Some(0));
    assert_eq!(align_offset(9, 8), Some(7));

    assert_eq!(align_offset(0, 16), Some(0));
    assert_eq!(align_offset(1, 16), Some(15));
    assert_eq!(align_offset(15, 16), Some(1));
    assert_eq!(align_offset(16, 16), Some(0));

    assert_eq!(align_offset(100, 3), None);
    assert_eq!(align_offset(usize::MAX, 8), None);
  }

  fn place_struct_in_buffer<T>(buffer: &mut [u8]) -> Option<*mut T> {
    let align = core::mem::align_of::<T>();
    let size = core::mem::size_of::<T>();
    let buffer_start = buffer.as_ptr() as usize;

    let offset = align_offset(buffer_start, align)?;

    if offset + size <= buffer.len() {
      Some(unsafe { buffer.as_mut_ptr().add(offset) as *mut T })
    } else {
      None
    }
  }

  #[test]
  fn test_struct_alignment_in_buffer() {
    #[repr(C)]
    struct TestStruct {
      a: u64,
      b: u32,
    }

    let mut buffer = [0u8; 64];
    let struct_ptr =
      place_struct_in_buffer::<TestStruct>(&mut buffer).expect("Buffer should fit aligned struct");

    assert!((struct_ptr as usize) % core::mem::align_of::<TestStruct>() == 0);

    unsafe {
      core::ptr::write(struct_ptr, TestStruct { a: 0x123, b: 0x456 });
      let read = core::ptr::read(struct_ptr);
      assert_eq!(read.a, 0x123);
      assert_eq!(read.b, 0x456);
    }
  }

  #[test]
  fn test_aligned_struct_placement() {
    #[repr(C, align(16))]
    struct AlignedStruct([u8; 8]);

    let mut buffer = [0u8; 128];
    let ptr = place_struct_in_buffer::<AlignedStruct>(&mut buffer)
      .expect("Should place 16-byte aligned struct");

    assert_eq!((ptr as usize) % 16, 0);

    unsafe {
      core::ptr::write(ptr, AlignedStruct([0xAA; 8]));
      assert_eq!((*ptr).0[0], 0xAA);
    }
  }
}
