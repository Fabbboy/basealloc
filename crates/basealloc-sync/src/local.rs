use core::marker::PhantomData;

use basealloc_fixed::bump::Bump;
use spin::Mutex;

use crate::lazy::LazyLock;

static TLS_BUMP: Mutex<Bump> = Mutex::new(Bump::new(1024 * 16));

pub struct ThreadLocal<T, F = fn() -> T> {
  key: LazyLock<libc::pthread_key_t>,
  init: F,
  _marker: PhantomData<T>,
}

unsafe extern "C" fn tls_detor<T>(ptr: *mut libc::c_void) {
  if ptr.is_null() {
    return;
  }

  unsafe { core::ptr::drop_in_place(ptr as *mut T) };
}

fn obtain_key<T>() -> libc::pthread_key_t {
  let mut key: libc::pthread_key_t = 0;
  let ret = unsafe { libc::pthread_key_create(&mut key, Some(tls_detor::<T>)) };
  if ret != 0 {
    panic!("Failed to create pthread key: {}", ret);
  }
  key
}

impl<T, F> ThreadLocal<T, F>
where
  F: Fn() -> T,
{
  pub const fn new(init: F) -> Self {
    Self {
      key: LazyLock::new(|| obtain_key::<T>()),
      init,
      _marker: PhantomData,
    }
  }

  fn get_or_init(&self) -> *mut T {
    let key = *self.key;
    let ptr = unsafe { libc::pthread_getspecific(key) } as *mut T;
    if !ptr.is_null() {
      return ptr;
    }

    let uninit = TLS_BUMP
      .lock()
      .create::<T>()
      .unwrap_or_else(|_| panic!("ThreadLocal bump allocation failed")) as *mut T;

    unsafe { uninit.write((self.init)()) };
    let ret = unsafe { libc::pthread_setspecific(key, uninit.cast()) };
    if ret != 0 {
      panic!("pthread_setspecific failed: {}", ret);
    }
    uninit
  }

  pub fn with<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
    let ptr = self.get_or_init();
    f(unsafe { &mut *ptr })
  }
}

impl<T, F> Drop for ThreadLocal<T, F> {
  fn drop(&mut self) {
    let _ = unsafe { libc::pthread_key_delete(*self.key) };
  }
}

unsafe impl<T, F> Send for ThreadLocal<T, F> {}
unsafe impl<T, F> Sync for ThreadLocal<T, F> {}
