use criterion::{criterion_group, criterion_main, Criterion};
use core::hint::black_box;
use basealloc_rtree::RTree;

type BenchTree = RTree<usize, 16>;

fn bench_radix_operations(c: &mut Criterion) {
  c.bench_function("radix_insert", |b| {
    b.iter_batched(
      || BenchTree::new(4096),
      |mut tree| {
        for i in 0..100 {
          let _ = tree.insert(i, Some(i));
        }
        black_box(&tree);
      },
      criterion::BatchSize::SmallInput,
    );
  });

  c.bench_function("radix_lookup", |b| {
    b.iter_batched(
      || {
        let mut tree = BenchTree::new(4096);
        for i in 0..100 {
          let _ = tree.insert(i, Some(i * 2));
        }
        tree
      },
      |tree| {
        let mut sum = 0;
        for i in 0..100 {
          if let Some(&value) = tree.lookup(i) {
            sum += value;
          }
        }
        black_box(sum);
      },
      criterion::BatchSize::SmallInput,
    );
  });

  c.bench_function("radix_remove", |b| {
    b.iter_batched(
      || {
        let mut tree = BenchTree::new(4096);
        for i in 0..100 {
          let _ = tree.insert(i, Some(i));
        }
        tree
      },
      |mut tree| {
        let mut sum = 0;
        for i in 0..100 {
          if let Some(value) = tree.remove(i) {
            sum += value;
          }
        }
        black_box(sum);
      },
      criterion::BatchSize::SmallInput,
    );
  });
}

criterion_group!(radix_benches, bench_radix_operations);
criterion_main!(radix_benches);