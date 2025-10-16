use criterion::{criterion_group, criterion_main, Criterion};
use core::hint::black_box;
use basealloc_list::prelude::*;

#[derive(Debug)]
struct BenchNode {
  value: usize,
  link: Link<Self>,
}

impl BenchNode {
  fn new(value: usize) -> Self {
    Self {
      value,
      link: Link::default(),
    }
  }
}

impl HasLink for BenchNode {
  fn link(&self) -> &Link<Self> {
    &self.link
  }

  fn link_mut(&mut self) -> &mut Link<Self> {
    &mut self.link
  }
}

fn bench_list_operations(c: &mut Criterion) {
  c.bench_function("list_insert_after", |b| {
    b.iter(|| {
      let mut nodes: Vec<BenchNode> = (0..100).map(BenchNode::new).collect();
      
      for i in 1..nodes.len() {
        let (left, right) = nodes.split_at_mut(i);
        List::insert_after(&mut right[0], &mut left[0]);
      }
      
      black_box(&nodes);
    });
  });

  c.bench_function("list_remove", |b| {
    b.iter_batched(
      || {
        let mut nodes: Vec<BenchNode> = (0..100).map(BenchNode::new).collect();
        for i in 1..nodes.len() {
          let (left, right) = nodes.split_at_mut(i);
          List::insert_after(&mut right[0], &mut left[i-1]);
        }
        nodes
      },
      |mut nodes| {
        for i in 1..nodes.len() {
          List::remove(&mut nodes[i]);
        }
        black_box(&nodes);
      },
      criterion::BatchSize::SmallInput,
    );
  });

  c.bench_function("list_iteration", |b| {
    b.iter_batched(
      || {
        let mut nodes: Vec<BenchNode> = (0..100).map(BenchNode::new).collect();
        for i in 1..nodes.len() {
          let (left, right) = nodes.split_at_mut(i);
          List::insert_after(&mut right[0], &mut left[i-1]);
        }
        nodes
      },
      |nodes| {
        let iter = ListIter::from(&nodes[0]);
        let sum: usize = iter.map(|node| node.value).sum();
        black_box(sum);
      },
      criterion::BatchSize::SmallInput,
    );
  });
}

criterion_group!(list_benches, bench_list_operations);
criterion_main!(list_benches);