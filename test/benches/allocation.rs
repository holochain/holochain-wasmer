use criterion::BenchmarkId;
use criterion::Throughput;
use criterion::{criterion_group, criterion_main, Criterion};
use holochain_serialized_bytes::prelude::*;
use holochain_wasmer_common::allocation::allocate;
use holochain_wasmer_common::allocation::deallocate;
use holochain_wasmer_common::AllocationPtr;

const EMPTY_WASM: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/wasm32-unknown-unknown/release/test_wasm_empty.wasm"
));

const NOOP_WASM: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/wasm32-unknown-unknown/release/test_wasm_noop.wasm"
));

const TEST_WASM: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/wasm32-unknown-unknown/release/test_wasm.wasm"
));

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
            b.iter_batched(
                || SerializedBytes::from(UnsafeBytes::from(vec![0_u8; n])),
                |sb| {
                    SerializedBytes::from(AllocationPtr::from(sb));
                },
                criterion::BatchSize::PerIteration,
            );
        });
    }
    group.finish();
}

fn wasm_instance(c: &mut Criterion) {
    let mut group = c.benchmark_group("wasm_instance");

    for (name, wasm) in vec![
        ("empty", EMPTY_WASM),
        ("noop", NOOP_WASM),
        ("test", TEST_WASM),
    ] {
        group.bench_with_input(
            BenchmarkId::new("wasm_instance", name),
            &wasm,
            |b, &wasm| {
                b.iter(|| {
                    holochain_wasmer_host::instantiate::instantiate(
                        &vec![0],
                        wasm,
                        &wasmer_runtime::imports!(),
                    )
                    .unwrap();
                });
            },
        );
    }

    group.finish();
}

fn wasm_call(c: &mut Criterion) {
    let mut group = c.benchmark_group("wasm_call");

    for f in vec!["a", "b", "c"] {
        for n in vec![0, 1, 1_000, 1_000_000] {
            group.throughput(Throughput::Bytes(n as _));
            group.sample_size(10);
            group.bench_with_input(BenchmarkId::new(&format!("noop {}", f), n), &n, |b, n| {
                let mut instance = holochain_wasmer_host::instantiate::instantiate(
                    &vec![1],
                    NOOP_WASM,
                    &test::import::memory_only(),
                )
                .unwrap();
                let input = test_common::StringType::from(".".repeat(*n));

                b.iter_batched(
                    || input.clone(),
                    |i| {
                        let _: test_common::StringType =
                            holochain_wasmer_host::guest::call(&mut instance, f, i).unwrap();
                    },
                    criterion::BatchSize::PerIteration,
                );
            });
        }
    }

    for n in vec![0, 1, 1_000, 1_000_000] {
        group.throughput(Throughput::Bytes(n as _));
        group.sample_size(10);
        group.bench_with_input(BenchmarkId::new("test_process_string", n), &n, |b, n| {
            let mut instance = holochain_wasmer_host::instantiate::instantiate(
                &vec![2],
                TEST_WASM,
                &test::import::import_object(),
            )
            .unwrap();
            let input = test_common::StringType::from(".".repeat(*n));

            b.iter_batched(
                || input.clone(),
                |i| {
                    let _: test_common::StringType =
                        holochain_wasmer_host::guest::call(&mut instance, "process_string", i)
                            .unwrap();
                },
                criterion::BatchSize::PerIteration,
            );
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    allocate_deallocate,
    sb_round_trip,
    wasm_instance,
    wasm_call
);

criterion_main!(benches);
