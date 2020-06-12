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

const IO_WASM: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/wasm32-unknown-unknown/release/test_wasm_io.wasm"
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

/// round trip serialized bytes through the AllocationPtr abstractions
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

/// create an instance
pub fn wasm_instance(c: &mut Criterion) {
    let mut group = c.benchmark_group("wasm_instance");

    for (name, wasm) in vec![("empty", EMPTY_WASM), ("io", IO_WASM), ("test", TEST_WASM)] {
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

/// call a function with an argument of size n
pub fn wasm_call(c: &mut Criterion) {
    let mut group = c.benchmark_group("wasm_call");

    let mut instance = holochain_wasmer_host::instantiate::instantiate(
        &vec![1],
        IO_WASM,
        &test::import::memory_only(),
    )
    .unwrap();

    macro_rules! bench_call {
        ( $fs:expr; $t:tt; $n:ident; $build:expr; ) => {
            let mut fs = $fs;
            // shuffle to avoid accidental hysteresis
            fs.shuffle(&mut thread_rng());

            for f in fs {
                for $n in vec![0, 1, 1_000, 1_000_000] {
                    group.throughput(Throughput::Bytes($n as _));
                    group.sample_size(10);

                    let input = test_common::$t::from($build);
                    group.bench_with_input(
                        BenchmarkId::new(&format!("io {}", f), $n),
                        &$n,
                        |b, _| {
                            b.iter(|| {
                                let _drop: test_common::$t =
                                    holochain_wasmer_host::guest::call(&mut instance, f, &input)
                                        .unwrap();
                            });
                        },
                    );
                }
            }
        };
    }

    bench_call!(
        vec![
            "string_input_args_empty_ret",
            "string_input_args_echo_ret",
        ];
        StringType;
        n;
        ".".repeat(n);
    );

    bench_call!(
        vec![
            "bytes_input_args_empty_ret",
            "bytes_input_args_echo_ret",
        ];
        BytesType;
        n;
        vec![0; n];
    );

    group.finish();
}

/// call a function that internally creates and returns a value of size n
pub fn wasm_call_n(c: &mut Criterion) {
    let mut group = c.benchmark_group("wasm_call_n");

    let mut instance = holochain_wasmer_host::instantiate::instantiate(
        &vec![1],
        IO_WASM,
        &test::import::memory_only(),
    )
    .unwrap();

    macro_rules! bench_n {
        ( $fs:expr; $t:ty; ) => {
            let mut fs = $fs;
            // randomise order to avoid hysteresis
            fs.shuffle(&mut thread_rng());

            for f in fs {
                for n in vec![0, 1, 1_000, 1_000_000] {
                    group.throughput(Throughput::Bytes(n as _));
                    group.sample_size(10);

                    group.bench_with_input(
                        BenchmarkId::new(&format!("io {}", f), n),
                        &n,
                        |b, _| {
                            b.iter(|| {
                                let _: $t = holochain_wasmer_host::guest::call(
                                    &mut instance,
                                    f,
                                    test_common::IntegerType::from(n),
                                )
                                .expect("failed deserialize");
                            });
                        },
                    );
                }
            }
        };
    };

    bench_n!( vec![ "bytes_serialize_n", "bytes_ret_n", ]; test_common::BytesType; );
    bench_n!( vec![ "string_serialize_n", "string_ret_n", ]; test_common::StringType; );

    group.finish();
}

/// basic bench for the basic tests
pub fn test_process_string(c: &mut Criterion) {
    let mut group = c.benchmark_group("test_process_string");

    let mut instance = holochain_wasmer_host::instantiate::instantiate(
        &vec![2],
        TEST_WASM,
        &test::import::import_object(),
    )
    .unwrap();

    for n in vec![0, 1, 1_000, 1_000_000] {
        group.throughput(Throughput::Bytes(n as _));
        group.sample_size(10);
        let input = test_common::StringType::from(".".repeat(n));
        group.bench_with_input(BenchmarkId::new("test_process_string", n), &n, |b, _| {
            b.iter(|| {
                let _: test_common::StringType =
                    holochain_wasmer_host::guest::call(&mut instance, "process_string", &input)
                        .unwrap();
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    allocate_deallocate,
    sb_round_trip,
    wasm_instance,
    wasm_call,
    wasm_call_n,
    test_process_string,
);

criterion_main!(benches);
