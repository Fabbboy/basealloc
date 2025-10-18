use basealloc::BaseAlloc;
use criterion::{
  Criterion,
  criterion_group,
  criterion_main,
};
use rand::rng;
use std::{
  collections::HashMap,
  hint::black_box,
};

#[global_allocator]
static GLOBAL: BaseAlloc = BaseAlloc {};

fn bench_vec_push(c: &mut Criterion) {
  c.bench_function("vec_push_1k", |b| {
    b.iter(|| {
      let mut v = Vec::with_capacity(1024);
      for i in 0..1024 {
        v.push(black_box(i));
      }
      black_box(v);
    });
  });
}

fn bench_vec_reserve(c: &mut Criterion) {
  c.bench_function("vec_reserve_1mb", |b| {
    b.iter(|| {
      let mut v = Vec::<u8>::new();
      v.reserve(1024 * 1024);
      black_box(v);
    });
  });
}

fn bench_box_alloc(c: &mut Criterion) {
  c.bench_function("box_alloc_1k", |b| {
    b.iter(|| {
      let bx = Box::new([0u8; 1024]);
      black_box(bx);
    });
  });
}

fn bench_hashmap_insert(c: &mut Criterion) {
  c.bench_function("hashmap_insert_1k", |b| {
    b.iter(|| {
      let mut map = HashMap::with_capacity(1024);
      for i in 0..1024 {
        map.insert(black_box(i), black_box(i * 2));
      }
      black_box(map);
    });
  });
}

fn bench_hashmap_lookup(c: &mut Criterion) {
  use rand::seq::SliceRandom;
  let mut keys: Vec<u64> = (0..10_000).collect();
  keys.shuffle(&mut rng());

  c.bench_function("hashmap_lookup_10k", |b| {
    let mut map = HashMap::new();
    for i in 0..10_000 {
      map.insert(i, i * 3);
    }
    b.iter(|| {
      for key in keys.iter() {
        black_box(map.get(key));
      }
    });
  });
}

criterion_group!(
  benches,
  bench_vec_push,
  bench_vec_reserve,
  bench_box_alloc,
  bench_hashmap_insert,
  bench_hashmap_lookup,
);
criterion_main!(benches);
