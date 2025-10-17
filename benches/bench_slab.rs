use std::hint::black_box;
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

use basealloc_alloc::{
  classes::{QUANTUM, class_for, class_at, pages_for, SlabSize},
  slab::Slab,
  static_::CHUNK_SIZE,
};
use basealloc_fixed::bump::Bump;

fn bench_slab_allocate(c: &mut Criterion) {
  let mut group = c.benchmark_group("slab_allocate");

  for size in [QUANTUM, QUANTUM * 4, QUANTUM * 16] {
    group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &sz| {
      let mut bump = Bump::new(CHUNK_SIZE);
      let class_idx = class_for(sz).unwrap();
      let class = class_at(class_idx);
      let SlabSize(slab_size) = pages_for(class_idx);
      let mut slab_ptr = Slab::new(&mut bump, class, slab_size).unwrap();
      let slab = unsafe { slab_ptr.as_mut() };

      b.iter(|| {
        let ptr = slab.allocate();
        black_box(&ptr);
        if let Ok(p) = ptr {
          let _ = slab.deallocate(p);
        }
      });
    });
  }

  group.finish();
}

fn bench_slab_allocate_deallocate(c: &mut Criterion) {
  let mut group = c.benchmark_group("slab_alloc_dealloc");

  for size in [QUANTUM, QUANTUM * 4, QUANTUM * 16] {
    group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &sz| {
      let mut bump = Bump::new(CHUNK_SIZE);
      let class_idx = class_for(sz).unwrap();
      let class = class_at(class_idx);
      let SlabSize(slab_size) = pages_for(class_idx);
      let mut slab_ptr = Slab::new(&mut bump, class, slab_size).unwrap();
      let slab = unsafe { slab_ptr.as_mut() };

      b.iter(|| {
        let ptr = slab.allocate().unwrap();
        black_box(ptr);
        slab.deallocate(black_box(ptr)).unwrap();
      });
    });
  }

  group.finish();
}

fn bench_slab_reuse(c: &mut Criterion) {
  let mut bump = Bump::new(CHUNK_SIZE);
  let class_idx = class_for(QUANTUM).unwrap();
  let class = class_at(class_idx);
  let SlabSize(slab_size) = pages_for(class_idx);
  let mut slab_ptr = Slab::new(&mut bump, class, slab_size).unwrap();
  let slab = unsafe { slab_ptr.as_mut() };

  c.bench_function("slab_reuse_same_slot", |b| {
    let ptr = slab.allocate().unwrap();
    slab.deallocate(ptr).unwrap();

    b.iter(|| {
      let p = slab.allocate().unwrap();
      black_box(p);
      slab.deallocate(p).unwrap();
    });
  });
}

fn bench_slab_interleaved(c: &mut Criterion) {
  c.bench_function("slab_interleaved_pattern", |b| {
    let mut bump = Bump::new(CHUNK_SIZE * 10);
    let class_idx = class_for(QUANTUM * 8).unwrap();
    let class = class_at(class_idx);
    let SlabSize(slab_size) = pages_for(class_idx);
    let mut slab_ptr = Slab::new(&mut bump, class, slab_size).unwrap();
    let slab = unsafe { slab_ptr.as_mut() };

    b.iter(|| {
      let p1 = slab.allocate().unwrap();
      let p2 = slab.allocate().unwrap();
      let p3 = slab.allocate().unwrap();
      black_box((p1, p2, p3));
      slab.deallocate(p2).unwrap();
      let p4 = slab.allocate().unwrap();
      black_box(p4);
      slab.deallocate(p1).unwrap();
      slab.deallocate(p3).unwrap();
      slab.deallocate(p4).unwrap();
    });
  });
}

criterion_group!(
  benches,
  bench_slab_allocate,
  bench_slab_allocate_deallocate,
  bench_slab_reuse,
  bench_slab_interleaved
);
criterion_main!(benches);
