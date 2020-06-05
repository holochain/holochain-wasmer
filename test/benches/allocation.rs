use criterion::{criterion_group, criterion_main, Criterion};
use holochain_wasmer_common::allocation::allocate;
use holochain_wasmer_common::allocation::deallocate;
use holochain_wasmer_common::AllocationPtr;
use holochain_serialized_bytes::prelude::*;
use criterion::Throughput;
use criterion::BenchmarkId;

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

fn wasm_instance(c: &mut Criterion) {
    let mut group = c.benchmark_group("wasm_instance");

    for (name, wasm) in vec![
        ("empty", EMPTY_WASM),
        ("noop", NOOP_WASM),
        ("test", TEST_WASM),
    ] {
        group.bench_with_input(BenchmarkId::new("wasm_instance", name), &wasm, |b, &wasm| {
            b.iter(|| {
                holochain_wasmer_host::instantiate::instantiate(
                    &vec![0],
                    wasm,
                    &wasmer_runtime::imports!(),
                ).unwrap();
            });
        });
    }

    group.finish();
}

fn wasm_call(c: &mut Criterion) {
    let mut group = c.benchmark_group("wasm_call");

    for (name, wasm, f) in vec![
        ("noop", NOOP_WASM, "a"),
    ] {
        group.bench_with_input(BenchmarkId::new("wasm_call", name), &wasm, |b, &wasm| {
            let mut instance = holochain_wasmer_host::instantiate::instantiate(
                &vec![0],
                wasm,
                &wasmer_runtime::imports!(),
            ).unwrap();

            b.iter(|| {
                let _: () = holochain_wasmer_host::guest::call(
                    &mut instance,
                    f,
                    (),
                ).unwrap();
            });
        });
    }
    group.finish();
}

criterion_group!(benches, allocate_deallocate, sb_round_trip, wasm_instance, wasm_call);

criterion_main!(benches);
