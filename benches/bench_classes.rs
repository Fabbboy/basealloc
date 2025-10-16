use basealloc_alloc::classes::class_for;
use criterion::{
  BenchmarkId,
  Criterion,
  criterion_group,
  criterion_main,
};
use std::hint::black_box;

fn bench_class_for_tiny(c: &mut Criterion) {
  let mut group = c.benchmark_group("class_for_tiny");
  group.sample_size(50);

  for size in [16, 256, 1024] {
    group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &s| {
      b.iter(|| class_for(black_box(s)));
    });
  }

  group.finish();
}

fn bench_class_for_regular(c: &mut Criterion) {
  let mut group = c.benchmark_group("class_for_regular");
  group.sample_size(50);

  for size in [2048, 32768, 524288] {
    group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &s| {
      b.iter(|| class_for(black_box(s)));
    });
  }

  group.finish();
}

fn bench_class_for_mixed(c: &mut Criterion) {
  let mut group = c.benchmark_group("class_for_mixed");
  group.sample_size(50);

  let sizes: Vec<usize> = vec![17, 65, 256, 1023, 2049, 8193, 65537];
  group.bench_function("mixed", |b| {
    b.iter(|| {
      for &size in &sizes {
        black_box(class_for(black_box(size)));
      }
    });
  });

  group.finish();
}

criterion_group!(
  benches,
  bench_class_for_tiny,
  bench_class_for_regular,
  bench_class_for_mixed
);
criterion_main!(benches);
