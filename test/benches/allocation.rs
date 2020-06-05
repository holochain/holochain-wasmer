use criterion::{criterion_group, criterion_main, Criterion};
use holochain_wasmer_common::allocation::allocate;
use holochain_wasmer_common::allocation::deallocate;
use holochain_wasmer_common::AllocationPtr;
use holochain_serialized_bytes::prelude::*;
use criterion::Throughput;
use criterion::BenchmarkId;

// simply allocate and deallocate some bytes
fn allocate_deallocate(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocate_deallocate");
    for n in vec![
        // 1 byte
        1,
        // 1 kb
        1_000,
        // 1 mb
        1_000_000,
        // 1 gb
        1_000_000_000,
    ] {
        group.throughput(Throughput::Bytes(n as _));
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                deallocate(allocate(n), n);
            });
        });
    }
    group.finish();
}

fn sb_round_trip(c: &mut Criterion) {
    let mut group = c.benchmark_group("sb_round_trip");

    for n in vec![
        // 1 byte
        1,
        // 1 kb
        1_000,
        // 1 mb
        1_000_000,
        // 1 gb
        1_000_000_000,
    ] {
        group.throughput(Throughput::Bytes(n as _));
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter_batched(|| {
                SerializedBytes::from(UnsafeBytes::from(vec![0_u8; n]))
            },
            |sb| {
                SerializedBytes::from(AllocationPtr::from(sb));
            },
            criterion::BatchSize::PerIteration
        );
        });
    }
    group.finish();
}

criterion_group!(benches, allocate_deallocate, sb_round_trip);

criterion_main!(benches);
