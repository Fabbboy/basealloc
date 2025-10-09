use super::*;
use crate::prim::page_size;

#[test]
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn test_posix_memory_alloc_dealloc() {
  let size = page_size();
  
  unsafe {
    let memory = GLOBAL_SYSTEM.alloc(size, SysOption::ReadWrite);
    assert!(memory.is_ok(), "Should allocate memory on POSIX systems");
    
    let slice = memory.unwrap();
    assert_eq!(slice.len(), size, "Allocated size should match requested size");
    
    slice[0] = 42;
    slice[size - 1] = 24;
    assert_eq!(slice[0], 42, "Should be able to write to allocated memory");
    assert_eq!(slice[size - 1], 24, "Should be able to write to end of allocated memory");
    
    let result = GLOBAL_SYSTEM.dealloc(slice);
    assert!(result.is_ok(), "Should deallocate memory successfully");
  }
}

#[test]
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn test_posix_memory_reserve_modify() {
  let size = page_size();
  
  unsafe {
    let memory = GLOBAL_SYSTEM.alloc(size, SysOption::Reserve);
    assert!(memory.is_ok(), "Should reserve memory on POSIX systems");
    
    let slice = memory.unwrap();
    
    let result = GLOBAL_SYSTEM.modify(slice, SysOption::ReadWrite);
    assert!(result.is_ok(), "Should modify memory protection");
    
    slice[0] = 42;
    assert_eq!(slice[0], 42, "Should be able to write after modifying protection");
    
    let result = GLOBAL_SYSTEM.dealloc(slice);
    assert!(result.is_ok(), "Should deallocate reserved memory");
  }
}

#[test]
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn test_posix_memory_reclaim() {
  let size = page_size() * 2;
  
  unsafe {
    let memory = GLOBAL_SYSTEM.alloc(size, SysOption::ReadWrite);
    assert!(memory.is_ok(), "Should allocate memory");
    
    let slice = memory.unwrap();
    slice.fill(42);
    
    let result = GLOBAL_SYSTEM.modify(slice, SysOption::Reclaim);
    assert!(result.is_ok(), "Should reclaim memory");
    
    let result = GLOBAL_SYSTEM.dealloc(slice);
    assert!(result.is_ok(), "Should deallocate reclaimed memory");
  }
}

#[test]
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn test_posix_invalid_size_alignment() {
  unsafe {
    let result = GLOBAL_SYSTEM.alloc(123, SysOption::ReadWrite);
    assert!(result.is_err(), "Should fail with non-page-aligned size");
    
    if let Err(error) = result {
      assert!(matches!(error, SysError::InvalidArgument), "Should return InvalidArgument");
    }
  }
}

#[test]
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn test_unsupported_system_alloc() {
  let size = 4096;
  
  unsafe {
    let result = GLOBAL_SYSTEM.alloc(size, SysOption::ReadWrite);
    assert!(result.is_err(), "Should fail on unsupported systems");
    
    if let Err(error) = result {
      assert!(matches!(error, SysError::Unsupported), "Should return Unsupported");
    }
  }
}

#[test]
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn test_unsupported_system_modify() {
  let dummy_slice = &[0u8; 4096];
  
  unsafe {
    let result = GLOBAL_SYSTEM.modify(dummy_slice, SysOption::ReadWrite);
    assert!(result.is_err(), "Should fail on unsupported systems");
    
    if let Err(error) = result {
      assert!(matches!(error, SysError::Unsupported), "Should return Unsupported");
    }
  }
}

#[test]
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn test_unsupported_system_dealloc() {
  let dummy_slice = &[0u8; 4096];
  
  unsafe {
    let result = GLOBAL_SYSTEM.dealloc(dummy_slice);
    assert!(result.is_err(), "Should fail on unsupported systems");
    
    if let Err(error) = result {
      assert!(matches!(error, SysError::Unsupported), "Should return Unsupported");
    }
  }
}