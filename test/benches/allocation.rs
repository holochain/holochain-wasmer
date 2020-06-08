use criterion::BenchmarkId;
use criterion::Throughput;
use criterion::{criterion_group, criterion_main, Criterion};
use holochain_serialized_bytes::prelude::*;
use holochain_wasmer_common::allocation::allocate;
use holochain_wasmer_common::allocation::deallocate;
use holochain_wasmer_common::AllocationPtr;
use rand::prelude::*;

pub const EMPTY_WASM: &[u8] = include_bytes!(concat!(
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

/// allocate and deallocate some bytes
/// there are several approaches commonly referenced to work with wasm memory, e.g.
/// - directly using pointers to the wasm memory from the host
/// - mapping over wasm memory cells per-byte
/// - using the WasmPtr abstraction
/// the higher level strategies provide stronger security guarantees but it's worth benchmarking
/// any implementation to ensure the checks and balances don't slow things down
pub fn allocate_deallocate(c: &mut Criterion) {
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

pub fn sb_round_trip(c: &mut Criterion) {
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

pub fn wasm_instance(c: &mut Criterion) {
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

    let mut fs = vec![
        "string_input_ignored_empty_ret",
        "string_input_args_empty_ret",
        "string_input_args_echo_ret",
        "string_serialize_large",
        "string_ret_large"
    ];
    fs.shuffle(&mut thread_rng());

    for f in fs {
        // let n = 1_000_000;
        for n in vec![0, 1, 1_000, 1_000_000] {
            group.throughput(Throughput::Bytes(n as _));
            group.sample_size(10);
            let mut instance = holochain_wasmer_host::instantiate::instantiate(
                &vec![1],
                NOOP_WASM,
                &test::import::memory_only(),
            )
            .unwrap();
            let input = test_common::StringType::from(".".repeat(n));
            group.bench_with_input(BenchmarkId::new(&format!("noop {}", f), n), &n, |b, _n| {
                b.iter_batched(
                    || input.clone(),
                    |i| {
                        let _: test_common::StringType =
                            holochain_wasmer_host::guest::call(&mut instance, f, i).unwrap();
                    },
                    criterion::BatchSize::LargeInput,
                );
            });
        }
    }

    // for n in vec![0, 1, 1_000, 1_000_000] {
    //     group.throughput(Throughput::Bytes(n as _));
    //     group.sample_size(10);
    //     group.bench_with_input(BenchmarkId::new("test_process_string", n), &n, |b, n| {
    //         let mut instance = holochain_wasmer_host::instantiate::instantiate(
    //             &vec![2],
    //             TEST_WASM,
    //             &test::import::import_object(),
    //         )
    //         .unwrap();
    //         let input = test_common::StringType::from(".".repeat(*n));
    //
    //         b.iter_batched(
    //             || input.clone(),
    //             |i| {
    //                 let _: test_common::StringType =
    //                     holochain_wasmer_host::guest::call(&mut instance, "process_string", i)
    //                         .unwrap();
    //             },
    //             criterion::BatchSize::PerIteration,
    //         );
    //     });
    // }

    group.finish();
}


// #[derive(serde::Serialize, serde::Deserialize, SerializedBytes)]
// struct GenericBytesNewType(Vec<u8>);
#[derive(serde::Serialize, serde::Deserialize, SerializedBytes)]
struct SpecializedBytesNewType(#[serde(with = "serde_bytes")] Vec<u8>);
pub fn round_trip_bytes(c: &mut Criterion) {
    let mut group = c.benchmark_group("round_trip_bytes");

    macro_rules! do_it {
        ( $newtype:tt ) => {
            for n in vec![0, 1, 1_000, 1_000_000] {
                group.throughput(Throughput::Bytes(n as _));
                group.sample_size(10);
                group.bench_with_input(BenchmarkId::new(stringify!($newtype), n), &n, |b, &n| {
                    b.iter_batched(
                        || vec![0_u8; n],
                        |s| {
                            <$newtype>::try_from(SerializedBytes::try_from($newtype(s)).unwrap())
                                .unwrap();
                        },
                        criterion::BatchSize::PerIteration,
                    );
                });
            }
        };
    };

    // do_it!(GenericBytesNewType);
    do_it!(SpecializedBytesNewType);

    group.finish();
}

criterion_group!(
    benches,
    // allocate_deallocate,
    // sb_round_trip,
    // wasm_instance,
    wasm_call,
    // round_trip_bytes
);

criterion_main!(benches);
