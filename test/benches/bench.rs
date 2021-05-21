use criterion::BenchmarkId;
use criterion::Throughput;
use criterion::{criterion_group, criterion_main, Criterion};
use holochain_wasmer_host::prelude::*;
use rand::prelude::*;
use std::sync::Arc;
use test::wasms::TestWasm;

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
                wasm.module();
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
        // let store = wasmer::Store::new(&JIT::new(Singlepass::new()).engine());
        // let serialized_module = wasmer::Module::from_binary(&store, wasm.bytes())
        //     .unwrap()
        //     .serialize()
        //     .unwrap();

        group.bench_function(BenchmarkId::new("wasm_instance", wasm.name()), |b| {
            b.iter(|| {
                // wasm.instance()
                // let store = wasmer::Store::new(&JIT::new(Singlepass::new()).engine());
                // // let module = wasmer::Module::from_binary(
                // //     &store,
                // //     wasm.bytes()
                // // ).unwrap();
                // let module =
                //     unsafe { wasmer::Module::deserialize(&store, &serialized_module).unwrap() };
                let module = wasm.module();
                // // let module = Arc::clone(&module);
                // // let module = module.clone();
                let env = Env::default();
                let import_object: wasmer::ImportObject = imports! {
                    "env" => {
                        "__import_data" => Function::new_native_with_env(
                            module.store(),
                            env.clone(),
                            holochain_wasmer_host::import::__import_data
                        ),
                        "__test_process_string" => Function::new_native_with_env(
                            module.store(),
                            env.clone(),
                            test::test_process_string
                        ),
                        "__test_process_struct" => Function::new_native_with_env(
                            module.store(),
                            env.clone(),
                            test::test_process_struct
                        ),
                        "__debug" => Function::new_native_with_env(
                            module.store(),
                            env.clone(),
                            test::debug
                        ),
                        "__pages" => Function::new_native_with_env(
                            module.store(),
                            env.clone(),
                            test::pages
                        ),
                    },
                };
                let _ = wasmer::Instance::new(&module, &import_object).unwrap();
            });
        });
    }

    group.finish();
}

/// call a function with an argument of size n
pub fn wasm_call(c: &mut Criterion) {
    let mut group = c.benchmark_group("wasm_call");

    let instance = TestWasm::Io.instance();

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
                                let _drop: test_common::$t = holochain_wasmer_host::guest::call(
                                    Arc::clone(&instance),
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

    let instance = TestWasm::Io.instance();

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
                                    Arc::clone(&instance),
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
    }

    bench_n!( vec![ "bytes_serialize_n", "bytes_ret_n", ]; test_common::BytesType; );
    bench_n!( vec![ "string_serialize_n", "string_ret_n", ]; test_common::StringType; );

    group.finish();
}

/// basic bench for the basic tests
pub fn test_process_string(c: &mut Criterion) {
    let mut group = c.benchmark_group("test_process_string");

    let instance = TestWasm::Test.instance();

    for n in vec![0, 1, 1_000, 1_000_000] {
        group.throughput(Throughput::Bytes(n as _));
        group.sample_size(10);
        let input = test_common::StringType::from(".".repeat(n));
        group.bench_with_input(BenchmarkId::new("test_process_string", n), &n, |b, _| {
            b.iter(|| {
                let _: test_common::StringType = holochain_wasmer_host::guest::call(
                    Arc::clone(&instance),
                    "process_string",
                    &input,
                )
                .unwrap();
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    wasm_module,
    wasm_instance,
    wasm_call,
    wasm_call_n,
    test_process_string,
);

criterion_main!(benches);
