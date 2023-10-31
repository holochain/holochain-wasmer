use criterion::BenchmarkId;
use criterion::Throughput;
use criterion::{criterion_group, criterion_main, Criterion};
use holochain_wasmer_host::prelude::*;
use rand::prelude::*;
use test::wasms::TestWasm;
use wasmer::AsStoreMut;

/// create a module
pub fn wasm_module(c: &mut Criterion) {
    let mut group = c.benchmark_group("wasm_module");

    for wasm in vec![
        TestWasm::Empty,
        TestWasm::Io,
        TestWasm::Test,
        TestWasm::Memory,
    ] {
        group.bench_function(BenchmarkId::new("wasm_module", wasm.name()), |b| {
            b.iter(|| {
                wasm.module(false);
            })
        });
    }

    group.finish()
}

/// create an instance
pub fn wasm_instance(c: &mut Criterion) {
    let mut group = c.benchmark_group("wasm_instance");

    for wasm in vec![
        TestWasm::Empty,
        TestWasm::Io,
        TestWasm::Test,
        TestWasm::Memory,
    ] {
        group.bench_function(BenchmarkId::new("wasm_instance", wasm.name()), |b| {
            b.iter(|| {
                let _drop = wasm.unmetered_instance();
            });
        });
    }

    group.finish();
}

/// call a function with an argument of size n
pub fn wasm_call(c: &mut Criterion) {
    let mut group = c.benchmark_group("wasm_call");

    let instance_with_store = TestWasm::Io.unmetered_instance();

    macro_rules! bench_call {
        ( $fs:expr; $t:tt; $n:ident; $build:expr; ) => {
            let mut fs = $fs;
            // shuffle to avoid accidental hysteresis
            fs.shuffle(&mut thread_rng());

            for f in fs {
                for $n in vec![0, 1, 1_000, 1_000_000] {
                    group.throughput(Throughput::Bytes($n));
                    group.sample_size(10);

                    let input = test_common::$t::from($build);
                    group.bench_with_input(
                        BenchmarkId::new(&format!("io {}", f), $n),
                        &$n,
                        |b, _| {
                            b.iter(|| {
                                let instance = instance_with_store.instance.clone();
                                let mut store_lock = instance_with_store.store.lock();
                                let mut store_mut = store_lock.as_store_mut();
                                let _drop: test_common::$t = holochain_wasmer_host::guest::call(
                                    &mut store_mut,
                                    instance,
                                    f,
                                    &input,
                                )
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
        ".".repeat(n.try_into().unwrap());
    );

    bench_call!(
        vec![
            "bytes_input_ignored_empty_ret",
            "bytes_input_args_empty_ret",
            "bytes_input_args_echo_ret",
        ];
        BytesType;
        n;
        vec![0; n.try_into().unwrap()];
    );

    group.finish();
}

/// call a function that internally creates and returns a value of size n
pub fn wasm_call_n(c: &mut Criterion) {
    let mut group = c.benchmark_group("wasm_call_n");

    let instance_with_store = TestWasm::Io.instance();

    macro_rules! bench_n {
        ( $fs:expr; $t:ty; ) => {
            let mut fs = $fs;
            // randomise order to avoid hysteresis
            fs.shuffle(&mut thread_rng());

            for f in fs {
                for n in vec![0_u32, 1, 1_000, 1_000_000] {
                    group.throughput(Throughput::Bytes(n.try_into().unwrap()));
                    group.sample_size(10);

                    group.bench_with_input(
                        BenchmarkId::new(&format!("io {}", f), n),
                        &n,
                        |b, _| {
                            b.iter(|| {
                                let instance = instance_with_store.instance.clone();
                                let store = instance_with_store.store.clone();
                                {
                                    let mut store_lock = store.lock();
                                    let mut store_mut = store_lock.as_store_mut();
                                    let _: $t = holochain_wasmer_host::guest::call(
                                        &mut store_mut,
                                        instance,
                                        f,
                                        test_common::IntegerType::from(n),
                                    )
                                    .expect("failed deserialize");
                                }
                            });
                        },
                    );
                }
            }
        };
    }

    bench_n!( vec![ "bytes_serialize_n", "bytes_ret_n", ]; test_common::BytesType; );
    bench_n!( vec![ "string_serialize_n", "string_ret_n", ]; test_common::StringType; );

    group.finish();
}

/// basic bench for the basic tests
pub fn test_process_string(c: &mut Criterion) {
    let mut group = c.benchmark_group("test_process_string");

    let instance_with_store = TestWasm::Test.instance();

    for n in vec![0, 1, 1_000, 1_000_000] {
        group.throughput(Throughput::Bytes(n));
        group.sample_size(10);
        let input = test_common::StringType::from(".".repeat(n.try_into().unwrap()));
        group.bench_with_input(BenchmarkId::new("test_process_string", n), &n, |b, _| {
            b.iter(|| {
                let instance = instance_with_store.instance.clone();
                let store = instance_with_store.store.clone();
                {
                    let mut store_lock = store.lock();
                    let mut store_mut = store_lock.as_store_mut();
                    let _drop: test_common::StringType = holochain_wasmer_host::guest::call(
                        &mut store_mut,
                        instance,
                        "process_string",
                        &input,
                    )
                    .unwrap();
                }
            });
        });
    }

    group.finish();
}

pub fn test_instances(c: &mut Criterion) {
    let mut group = c.benchmark_group("test_instances");
    group.throughput(Throughput::Bytes(1_000));
    group.sample_size(100);
    let input = test_common::StringType::from(".".repeat(1000));
    group.bench_with_input(BenchmarkId::new("test_instances", 1000), &1000, |b, _| {
        b.iter(|| {
            let mut jhs = Vec::new();
            for _ in 0..25 {
                let input = input.clone();
                let instance_with_store = TestWasm::Test.instance();
                let instance = instance_with_store.instance.clone();
                let store = instance_with_store.store.clone();
                let jh = std::thread::spawn(move || {
                    let _: test_common::StringType = holochain_wasmer_host::guest::call(
                        &mut store.lock().as_store_mut(),
                        instance,
                        "process_string",
                        &input,
                    )
                    .unwrap();
                });
                jhs.push(jh);
            }
            for jh in jhs {
                jh.join().unwrap();
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    wasm_module,
    wasm_instance,
    wasm_call,
    wasm_call_n,
    test_process_string,
    test_instances,
);

criterion_main!(benches);
