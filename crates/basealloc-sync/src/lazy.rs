use core::{
  cell::UnsafeCell,
  mem::ManuallyDrop,
  ops::{
    Deref,
    DerefMut,
  },
};

use spin::Once;

struct Data<T, F> {
  value: ManuallyDrop<Option<T>>,
  f: ManuallyDrop<F>,
}

pub struct LazyLock<T, F = fn() -> T> {
  once: Once,
  data: UnsafeCell<Data<T, F>>,
}

impl<T, F> LazyLock<T, F>
where
  F: FnOnce() -> T,
{
  pub const fn new(f: F) -> Self {
    Self {
      once: Once::new(),
      data: UnsafeCell::new(Data {
        f: ManuallyDrop::new(f),
        value: ManuallyDrop::new(None),
      }),
    }
  }

  pub fn force(this: &LazyLock<T, F>) -> &T {
    this.once.call_once(|| {
      let data = unsafe { &mut *this.data.get() };
      let f = unsafe { ManuallyDrop::take(&mut data.f) };
      data.value = ManuallyDrop::new(Some(f()));
    });

    let data = unsafe { &*this.data.get() };
    data.value.as_ref().unwrap()
  }

  pub fn force_mut(this: &mut LazyLock<T, F>) -> &mut T {
    this.once.call_once(|| {
      let data = unsafe { &mut *this.data.get() };
      let f = unsafe { ManuallyDrop::take(&mut data.f) };
      data.value = ManuallyDrop::new(Some(f()));
    });

    let data = unsafe { &mut *this.data.get() };
    data.value.as_mut().unwrap()
  }
}

impl<T, F> Deref for LazyLock<T, F>
where
  F: FnOnce() -> T,
{
  type Target = T;
  fn deref(&self) -> &Self::Target {
    Self::force(self)
  }
}

impl<T, F> DerefMut for LazyLock<T, F>
where
  F: FnOnce() -> T,
{
  fn deref_mut(&mut self) -> &mut Self::Target {
    Self::force_mut(self)
  }
}

unsafe impl<T, F> Sync for LazyLock<T, F>
where
  T: Sync,
  F: Send + FnOnce() -> T,
{
}
unsafe impl<T, F> Send for LazyLock<T, F>
where
  T: Send,
  F: Send + FnOnce() -> T,
{
}
