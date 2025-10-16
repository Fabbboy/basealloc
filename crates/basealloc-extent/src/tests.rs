use super::*;
use basealloc_sys::prelude::*;

#[test]
fn test_extent_new() {
  let ps = page_size();
  let extent = Extent::new(ps, SysOption::Commit);
  assert!(extent.is_ok());
  let extent = extent.unwrap();
  assert_eq!(extent.as_ref().len(), ps);
}

#[test]
fn test_extent_zero_size() {
  let extent = Extent::new(0, SysOption::Commit);
  assert!(extent.is_ok());
  let extent = extent.unwrap();
  assert_eq!(extent.as_ref().len(), 0);
}

#[test]
fn test_extent_as_ref() {
  let ps = page_size();
  let extent = Extent::new(ps, SysOption::Commit).unwrap();
  let slice = extent.as_ref();
  assert_eq!(slice.len(), ps);
}

#[test]
fn test_extent_as_mut() {
  let ps = page_size();
  let mut extent = Extent::new(ps, SysOption::Commit).unwrap();
  let slice = extent.as_mut();
  assert_eq!(slice.len(), ps);
  slice[0] = 42;
  assert_eq!(slice[0], 42);
}

#[test]
fn test_extent_check_valid() {
  let ps = page_size();
  let extent = Extent::new(ps, SysOption::Commit).unwrap();
  assert!(extent.check(0..ps / 2).is_ok());
  assert!(extent.check(0..ps).is_ok());
  assert!(extent.check(100..100).is_ok());
}

#[test]
fn test_extent_check_invalid() {
  let ps = page_size();
  let extent = Extent::new(ps, SysOption::Commit).unwrap();
  assert!(matches!(
    extent.check(0..ps + 1),
    Err(ExtentError::OutOfBounds)
  ));
  assert!(matches!(
    extent.check(100..50),
    Err(ExtentError::OutOfBounds)
  ));
  assert!(matches!(
    extent.check(ps + 1..ps + 2),
    Err(ExtentError::OutOfBounds)
  ));
}

#[test]
fn test_extent_drop() {
  let ps = page_size();
  let extent = Extent::new(ps, SysOption::Commit).unwrap();
  drop(extent);
}

#[test]
fn test_extent_giveup() {
  let ps = page_size();
  let extent = Extent::new(ps, SysOption::Commit).unwrap();
  let len = extent.as_ref().len();
  let extent2 = extent.giveup();
  assert!(extent2.is_ok());
  assert_eq!(extent2.unwrap().as_ref().len(), len);
}
