use super::*;
use core::alloc::Layout;

#[test]
fn alloc_respects_alignment_and_size() {
  let mut buffer = [0u8; 32];
  let base = buffer.as_ptr() as usize;
  let mut bump = Fixed::new(&buffer);

  let layout_4 = Layout::from_size_align(4, 4).unwrap();
  let block_4 = match bump.allocate(&mut buffer, layout_4) {
    Ok(block) => block,
    Err(_) => panic!("4-byte allocation should succeed"),
  };
  assert_eq!(block_4.len(), 4);
  assert_eq!(block_4.as_ptr() as usize - base, 0);
  assert_eq!((block_4.as_ptr() as usize) % 4, 0);

  let layout_8 = Layout::from_size_align(5, 8).unwrap();
  let block_8 = match bump.allocate(&mut buffer, layout_8) {
    Ok(block) => block,
    Err(_) => panic!("5-byte aligned-to-8 allocation should succeed"),
  };
  assert_eq!(
    block_8.len(),
    8,
    "allocation should round size up to alignment"
  );
  assert_eq!(
    block_8.as_ptr() as usize - base,
    8,
    "second block should start at the next 8-byte boundary"
  );
  assert_eq!(
    (block_8.as_ptr() as usize) % 8,
    0,
    "allocation should respect requested alignment"
  );
}

#[test]
fn alloc_reports_oom_and_preserves_state() {
  let mut buffer = [0u8; 16];
  let mut bump = Fixed::new(&buffer);

  let first = Layout::from_size_align(8, 8).unwrap();
  if bump.allocate(&mut buffer, first).is_err() {
    panic!("initial allocation should succeed");
  }

  let oversized = Layout::from_size_align(16, 8).unwrap();
  assert!(
    matches!(bump.allocate(&mut buffer, oversized), Err(FixedError::OOM)),
    "allocation exceeding remaining capacity should signal OOM"
  );

  let later = Layout::from_size_align(4, 8).unwrap();
  let block = match bump.allocate(&mut buffer, later) {
    Ok(block) => block,
    Err(_) => panic!("allocator state should remain usable after OOM"),
  };
  assert_eq!(block.len(), 8);
  assert_eq!((block.as_ptr() as usize) % 8, 0);
}

#[test]
fn create_returns_zeroed_value() {
  #[repr(C)]
  struct Sample {
    a: u32,
    b: u64,
  }

  let mut buffer = [0u8; core::mem::size_of::<Sample>() * 2];
  let mut bump = Fixed::new(&mut buffer);

  let sample = match bump.create::<Sample>(&mut buffer) {
    Ok(value) => value,
    Err(_) => panic!("create should allocate space for Sample"),
  };

  let sample_ref = unsafe { &*sample };

  assert_eq!(sample_ref.a, 0);
  assert_eq!(sample_ref.b, 0);
  assert_eq!(
    (sample as *const Sample as usize) % core::mem::align_of::<Sample>(),
    0
  );
}
