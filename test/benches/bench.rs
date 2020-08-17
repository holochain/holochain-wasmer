use criterion::BenchmarkId;
use criterion::Throughput;
use criterion::{criterion_group, criterion_main, Criterion};
use rand::prelude::*;
use test::wasms;

/// create an instance
pub fn wasm_instance(c: &mut Criterion) {
    let mut group = c.benchmark_group("wasm_instance");

    for (name, wasm) in vec![
        ("empty", wasms::EMPTY),
        ("io", wasms::IO),
        ("test", wasms::TEST),
    ] {
        group.bench_with_input(
            BenchmarkId::new("wasm_instance", name),
            &wasm,
            |b, &wasm| {
                b.iter(|| {
                    holochain_wasmer_host::instantiate::instantiate::<String>(
                        &vec![0],
                        wasm,
                        &wasmer_runtime::imports!(),
                        None,
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

    let mut instance = holochain_wasmer_host::instantiate::instantiate::<String>(
        &vec![1],
        wasms::IO,
        &test::import::memory_only(),
        None,
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
            "string_input_ignored_empty_ret",
            "string_input_args_empty_ret",
            "string_input_args_echo_ret",
        ];
        StringType;
        n;
        ".".repeat(n);
    );

    bench_call!(
        vec![
            "bytes_input_ignored_empty_ret",
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

    let mut instance = holochain_wasmer_host::instantiate::instantiate::<String>(
        &vec![1],
        wasms::IO,
        &test::import::memory_only(),
        None,
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

    let mut instance = holochain_wasmer_host::instantiate::instantiate::<String>(
        &vec![2],
        wasms::TEST,
        &test::import::import_object(),
        None,
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
    wasm_instance,
    wasm_call,
    wasm_call_n,
    test_process_string,
);

criterion_main!(benches);
