# holochain-wasmer

## How to update holonix

This repository uses the `rustDev` shell from [Holonix](https://github.com/holochain/holochain/blob/develop/nix/modules/devShells.nix).

To update Holonix, run the following command:

```bash
nix flake update
````

## Why?

Doing certain high level things in wasm is still pretty hard to get right, even
with Rust.

Luckily Rust provides us with enough tools to abstract what we need at the
compiler level. Most details are not visible downstream to happ devs and can be
achieved via. 'zero cost abstraction'.

That said, there _are_ some limitations by design that we enforce here to allow
the whole wasm stack to be simple to maintain and understand.

The Rust compiler makes many things sane for us in wasm but there are a few
notable things that are left up to us:

- Define a clear interface between the "host" and the "guest" within wasm limits
- Manage a shared memory across the host/guest with different Rust allocators
- Inject additional runtime context on the host that the guest cannot provide
- Performance optimisations

## What

This repository consists of 3 main library crates:

- `holochain_wasmer_common`: host/guest agnostic and shared functionality
- `holochain_wasmer_guest`: essential macros for wasm guests
- `holochain_wasmer_host`: infrastructure to manage a wasm guest

There is also a `test` directory containing analogous crates implementing the
above libraries for the purpose of testing and simple working examples.

- `test/common`: data structures shared by the host and guest
- `test/src`: a wasm host containing test functions
- `test/wasm`: a guest wasm containing test functions

The main dependencies are:

- [wasmer](https://wasmer.io/): one of the best wasm implementations for Rust
- [holochain_serialization](https://github.com/holochain/holochain-serialization): our crates to normalize serialization at the byte level
- [holonix](https://github.com/holochain/holonix): specifically the Rust version management and wasm tooling

## How to use

There are several places we need to implement things:

- Holochain core needs to [act as a wasm host](https://docs.wasmer.io/integrations/rust/examples/hello-world) to build modules and instances to run wasm functions
- Holochain core [needs to provide](https://docs.wasmer.io/integrations/rust/examples/host-functions) 'imported functions' as an `ImportObject`
- Holochain HDK needs to use the `holochain_wasmer_guest` functions to wrap externs in something ergonomic for happ developers
- Happ developers need to be broadly aware of how to send cleanly serializable inputs and work with serde

## Fuzzing

You can fuzz this repository as:

```
docker run --rm --env FUZZ_TARGET="<some fuzz target>" -it holochain/fuzzbox:holochain-wasmer
```

You may need to pull the tag before fuzzing to get the latest code as it is built
on CI against main.

For more information on fuzzbox see https://github.com/holochain/fuzzbox.

### Holochain core

#### Being a good wasm host

It is a multi-step process to get from rust code to running a wasm function.

0. Rust guest .rs files are compiled to .wasm files
1. Rust wasmer host compiles the .wasm files to a native 'module'
2. The module is instantiated to an 'instance' with imported functions, linear memory and whatever else wasmer needs to call functions

The first step needs to be handled by happ developers using relevant tooling.

Holochain core will be passed a .wasm file and needs to build running instances.

Basic performance testing showed that the default wasmer handling of a ~40mb .wasm
file takes 1-2 seconds to compile into a module.

Wasmer has a native cache trait and can serialize modules into something that loads much faster.

Loading a serialized module from an NVMe disk and instantiating it still takes about `500ms`.

The default file system cache is about 2-4x faster than cold compiling a module
but is still too slow to be hitting on every function call.

Pulling a module from a lazy static and instantiating it takes about `50ns` which is
very reasonable overhead for the host to build a completely fresh instance.

Calling a function with `holochain_wasmer_host::guest::call()` takes several `us`
for small input/output values and some `ms` for ~1mb of input/output data.

To see benchmarks on your system run `nix-shell --run ./bench.sh`.

With low overhead like this, core is relatively free to decide when it wants to
re-instantiate an already-in-memory module.

Re-instantiating modules has several potential benefits:

- Fresh linear memory every time means simpler wasm logic and less memory footprint (because wasm memory _pages_ can never be freed)
- Core can provide fresh references (e.g. `Arc::clone`) to its internals and fresh closures on each function call
- Potentially simpler core code to simply create a new instance each call vs. trying to manage shared/global/long running things

Note though, that cache key generation for modules in memory (e.g. multiple DNAs in
 memory) has performance implications too.

The default wasmer handling hashes the wasm bytes passed to it to create a key to
lookup a module for.

Hashing a 40mb wasm file with the wasmer algorithm takes about `15ms` which is not
huge but is a bit high to be doing every function call.

Given that we already hash DNAs, it makes sense that we pass in the DNA hash, or
something similar, and use this as the cache key per function call, which takes only
a few nanoseconds.

To handle all this use `host::instantiate::instantiate()` which is a wrapper around
the default wasmer instantiate.

__Always use the instantiate function as it adds a guard against a badly behaved
guest wasm forcing the host to leak memory.__

It takes an additional argument `cache_key_bytes: &[u8]` which can either be the
raw wasm or something precalculated like the DNA hash.

Internally the module will be compiled once and stored in a lazy static and then
every new instance will re-use the module.

The full `instantiate` signature is:

- `cache_key_bytes: &[u8]`: the key for the in-memory module cache
- `wasm: &[u8]`: the raw bytes of the wasm to compile into a module (can be the same as the cache key)
- `wasm_imports: &ImportObject`: a standard wasmer `ImportObject`

It is expected that the `instantiate` function here will evolve alongside the
core persistence implementation so that e.g. lmdb could be used as a cache backend.

See `test_instance` for an example of getting an instance:

```rust
fn test_instance() -> Instance {
    let wasm = load_wasm();
    instantiate(&wasm, &wasm, &import_object()).expect("build test instance")
}
```

And `native_test` to show how to call a function with structure input/output:

```rust
#[test]
fn native_test() {
    let some_inner = "foo";
    let some_struct = SomeStruct::new(some_inner.into());

    let result: SomeStruct =
        guest::call(&mut test_instance(), "native_type", some_struct.clone())
            .expect("native type handling");

    assert_eq!(some_struct, result,);
}
```

All the magic happens in `host::guest:call()`, just make sure to tell Rust the
return type in the `let result: SomeStruct = ...` expression.

#### Building an `ImportObject`

The [wasmer docs](https://docs.wasmer.io/integrations/rust/examples/host-functions) generally provide a good overview.

See the [PR i opened against the old system](https://github.com/holochain/holochain-rust/pull/2079/files#diff-f066dcca6e836742cae1afd74341a72aR169) for working examples and macros.

Also see the tests in this repository for a minimal example.

However, there are a few 'gotchas' to be aware of.

##### Don't plan to call back into the same instance

The function signature of an `ImportObject` function includes a wasmer context `&mut Ctx`
as the first argument but it does not provide access to the current instance.

This means that an imported function can build new instances for the same module
if there is an `Arc` or similar available in the closure, but these new instances
would have their own memory and closures.

One potential (untested) workaround for this could be to init some constant inside
the wasm guest that is a key for a global registry of active instances on the host
but i'd generally avoid something complex like this that would involve global state,
mutexes probably, cleanup, etc.

Better to design core such that imported functions are 'self contained' on the
host side and don't need to call back into the guest.

For example, we would NOT be able to write validation callbacks in the wasm guest
that read from global memory that was previously written to by the guest. The
validation callback would be running in a separate wasm instance with isolated
memory from the original wasm that called the host function that triggered the
callback.

I'd argue that this is A Good Thing for us anyway, as guest callbacks should be
pure functions of their arguments, and isolating their memory is an effective
way to limit the potential for accidental state creeping into callbacks.

Note this is just about sharing the same wasm guest, it doesn't stop us from keeping
a consistent persistence cursor/transaction open across all the related wasm guest
calls, it just means they can't share the internal instance state.


##### Always need fresh references in closures

The functions in an `ImportObject` MUST be an `Fn`, e.g. not an `FnOnce` or `FnMut`.

I found the easiest way to achieve this without fighting lifetimes or global scope
is to do the following:

- Some struct exists that can be passed around that can access wasm bytes
- This struct `impl` some instantiate method
- The instantiate method builds an `ImportObject` internally
- The instantiate method does `Arc::clone()` to `self` (the struct that can access wasm bytes)
- All the functions inside the closures that 'do work' also recieve newly cloned `Arc`s on each call

As long as we are cloning fresh `Arc` values on each instantiate and each function
call, we get to keep `Fn` which makes wasmer happy without us worrying about lifetimes.

Having an `Arc` to `self` which has access to wasm bytes allows us to create new
modules/instances inside imported function closures, which will probably be needed
e.g. for nice callback handling.

##### Use the host crate

It's really easy to make a mistake in the data handling (see below) and end up
with memory leaks or serialization mistakes, missing tracing or whatever else.

Use the `holochain_wasmer_host` crate to do as much heavy lifting as possible.

The `test_process_struct` shows a good minimal example of an import function:

```rust
pub fn test_process_struct(
    env: &Env,
    guest_ptr: usize,
    len: usize,
) -> Result<DoubleUSize, wasmer::RuntimeError> {
    let mut some_struct: SomeStruct = env.consume_bytes_from_guest(guest_ptr, len)?;
    some_struct.process();
    env.move_data_to_guest(Ok::<SomeStruct, WasmError>(some_struct))
}
```

It shows how to retrieve the input struct from the guest:

```rust
    let mut some_struct: SomeStruct = env.consume_bytes_from_guest(guest_ptr, len)?;
```

And how to build a return value that wasmer understands and the guest can read:

```rust
    env.move_data_to_guest(Ok::<SomeStruct, WasmError>(some_struct))
```

### Being a good wasm guest

Ideally we want the HDK to hide as much of this as possible.

The experience of building a happ should be as close to building a native Rust
binary as possible.

That said, there are some details that won't be able to be hidden completely.

Devs will need to learn how to use the HDK macros and make sure their data
cleanly serializes and deserializes as messagepack between any external
interface. For example it is the happ dev's responsiblity to make sure a number
serialized by JavaScript deserializes e.g. to a `u32` Rust input to an extern.

Specifically:

- use the same version of `holochain_serialized_bytes` as HDK/core
- define functions that can be exposed to a wasm host (e.g. using HDK macros)
- implement `serde` correctly using the same version as core
- Serialize inputs in a compatible format

Generally we want the HDK/tooling to hide/smooth at least the following details:

- Keeping a small .wasm file (e.g. optimisation tooling)
- All memory management
- Implementing sane wrappers around imported holochain functions to be ergonomic
- Needing to interact directly with the 'outer' `Result`
- Lots of other things...

At a high level there isn't much that a guest needs to do:

- Define externs that will be provided by the imported host functions
- Write extern functions that the host will call
- Use `host_args` to receive the input arguments from the host
- Use `host_call` to call a host function
- Use `return_ptr` to return a value to the host
- Use `return_err_ptr` to return an error to the host
- Use `try_ptr!` to emulate a `?` in a function that returns to the host

The tests wasm includes examples for all of these.

There is more documentation for this in the HDK itself.

#### Define externs that will be overwritten by the imported host functions

There are two sets of externs to define:

- The 'internal' externs used to make memory work
- The externs that represent callable functions on the host

The HDK absolutely should handle all of this for the happ developer as the
memory externs are mandatory and the callable functions are all set by holochain
core.

To do this manually:

- Use the `host_externs!(foo, bar, baz, ...)` macro to list all the importable host functions.
- Use the `memory_externs!()` macro to define the minimal memory logic needed by core

#### Write extern functions that the host will call

The HDK makes this mostly invisible to the happ developer.

All functions that the host can call must look like this to be compatible with our setup:

```rust
#[no_mangle]
pub extern "C" fn foo(guest_ptr: u32, len: u32) -> u64 {

}
```

This tells the rust compiler to make `foo` available in the final .wasm file as
something that can be called by the host as `"foo"`.

As the host is dealing with strings rather than functions, we implemented a 'hook' style callback system into the HDK.

E.g. the guest could implement `validate_MY_THING` and the host can call
`"validate_MY_THING"` if the function exists in the wasm module or just `validate`
if the less specfic version exists.

This means that structured data for input/output and `Result` style return
values (and therefore also `?`) need to be handled through serialization and
direct manipulation of bytes.

#### Use `host_args` to receive input from the host

This is easy, `host_args` takes `u32` pointer and length and tries to inject it into `SomeType`:

```rust
#[no_mangle]
pub extern "C" fn foo(remote_ptr: usize, len: usize) -> DoubleUSize {
 let bar: SomeType = match host_args(remote_ptr, len) {
  Ok(v) => v,
  Err(guest_ptr) => return guest_ptr,
 }
}
```

The `host_args` function _returns an `Err(GuestPtrLen)` if it errors_.

Notably it errors if deserialization fails.

If it errors the guest MUST __immediately__ return the `GuestPtrLen` to the host.

The guest MUST call `host_args` before attempting to call any host functions or
the guest memory will likely be corrupted and unrecoverable.

The host memory will not be permanently effected if it correctly implements
guest handling, so a malicious guest cannot damage the host in this way.

#### Use `host_call` to call host functions

This works a bit different to `host_args` as it _returns a native Rust `Result`_.

This allows it to be used anywhere in a wasm (e.g. even outside of an extern).

Pass the extern defined in `host_externs` along with anything serializable.
The types must be provided by the guest.

```rust
host_externs!(__some_host_function);

fn foo() -> Result<SomeStruct, WasmError> {
 let input = String::from("bar");

 // host_call returns the `Result` as per the host function return value
 // it also respects `?` (see test wasm for examples)
 // it knows to pull the return from the host back into a String based on
 // the Ok arm of the Result.
 // Note there is an 'outer' `Result` that needs to propagate back to the host
 // as a `GuestPtr` if there is an `Err`, the guest should unwind in that case.
 let output = host_call::<&String, HostFunctionOutput>(__some_host_function, &input)?;
}
```

In a guest extern you will likely want to wrap the `host_call` in a `try_ptr!` (see below):

```rust
host_externs!(__some_host_function);

extern "C" fn foo(_: usize, _: usize) -> DoubleUSize {
 let input = String::from("bar");

 // note the try_ptr! wrapper to be compatible with GuestPtr return value
 let output = try_ptr!(
  host_call::<&String, HostFunctionOutput>(__some_host_function, &input),
  "failed to call __some_host_function"
 );
}
```

#### Return any Err(GuestPtr) values immediately

Inside an extern we must return a `GuestPtrLen`.

The host is expecting a serialized `Result` (see below) whether we succeed or
fail. This is the 'outer' `Result` that needs to communicate to the host whether
or not the guest needed to stop and unwind due to a problem with the host/guest
interface itself. For example, if the host passes data that cannot be
deserialized by the guest, the guest needs to immediately stop and return these
bad bytes back to the host as a pointer to an `Err`.

This is true regardless of how deeply nested or complicated logic is within an
extern.

Any failure to interact with the host must immediately unwind with an `Err`.

Returning an `Ok(_)` tells the host that the guest managed to execute the extern
completely without any issues with the host.

The contents of the outer result can be anything that serializes, including
other results.

`Ok(Ok(_))` implies the guest succeeded to complete _and_ whatever called the
extern is expecting a `Result` for whatever domain specific logic was executed.

## Background information

I won't attempt a comprehensive guide to wasm here, it's a huge topic.

There are some key things to understand or this crate won't make sense.

### WASM data types are very limited

WASM only has 4 data types: `i32`, `i64`, `f32` and `f64`.

This represents integers and floats.

Integers are '[sign agnostic](https://rsms.me/wasm-intro#sign-agnostic)' which
can be awkward in Rust, that only has explicitly signed/unsigned primitives.
This basically means that integers are just chunks of binary data that allow
contextual math operations. For example, nothing in wasm prevents us from
performing signed and unsigned math operations on the same number. The number
itself is not signed, it's just that certain math requires the developer to
adopt consistent _conventions_ in order to write correct code. This is a poor
fit for the Rust mentality that demands _proofs_ at the compiler level, not mere
conventions.

By contrast, Rust doesn't even let us represent `i64` and `u64` in the same part
of our codebase, we must always be completely unambiguous about which type every
value is. Moving between `i64` and `u64` requires explicit error handling every
time.

Wasm floats show some [non-deterministic behaviour](https://webassembly.org/docs/nondeterminism/) in the case of `NaN` values.
The cranelift compiler can be configured to canonicalize `NaN` values and it is
strongly recommended to enable this. Non-determinism is very bad in the context
a p2p network because it means we cannot differentiate clearly between honest
and dishonest actors based on individual pieces of data. At best we can apply
statistical heuristics across many data points that are costly and can be gamed
or avoided by attackers.

Wasm has no strings, sequences, structs or any other collection or complex type.

It is clear that to get from the world of the compile time Rust types to runtime
WASM binary data, we will need a clear mapping and abstraction from raw integers
to complex data for both arguments to functions and return values, in both
directions across the host/guest boundary.

### WASM memory/function calls are very limited & Rust supports custom allocators

WASM only supports a single, shared linear memory between host and guest.

WASM itself has no high level memory handling and nothing like garbage
collection.

The host can read and write bytes directly to the guest's memory at any time,
including while the guest is executing its own code (in a multi-threaded context).

The host has no access to any logic or abstractions inside the guest, other than
to call explicitly exposed functions with integer arguments and return values.
For example, the host cannot interact directly with the guest's data structures
or allocator, or locks around things in shared memory.

The guest has no direct access to the host's memory or functions. The host must
'import' whitelisted functions into the guest at the moment the wasm is
instantiated. The guest can call these imported functions with wasm data types
(i.e. integers or floats) and receive a single integer/float back from the host.

There is no support for `Result` style function calls across the host/guest
boundary, although the host has limited support for `Result` return values
within the context of wasmer instance closures (i.e. the host can pass an error
back to itself and panic the guest).

The only way that the guest can access host memory is if it calls an imported
host function that in turn copies bytes directly into the guest's shared memory
somewhere, and then this function returns to the guest a pointer to where the
data was copied.

When the guest calls the host, it is not possible for the host to call back into
the guest (although the host can create a new, separate wasm instance and call
that). The host must wait for guest calls to complete before calling again and
the guest must wait for the host to complete before it can continue.

WASM has a hard limit in the spec of 4GB total memory, with 64kb pages.
Some WASM implementations, notably in some web browsers, limit this further.
Pages can be added at initialization or dynamically at runtime, but cannot be
removed, so a long-running wasm can be expected to eat a lot of memory if a
large amount of data crosses into the guest even momentarily.

Rust helps the situation a lot by providing a strong memory management model
enforced by the compiler but also allows for the host and the guest to have
different allocation models.

Even 'simple' primitives like `String` are not safe to round trip through their
'raw parts' (e.g. length, capacity and pointer for `String`) if the allocator
is different at source and destination.

It's not even clear whether a mismatch in Rust compiler versions constitutes
a different allocator for the purposes of avoiding memory corruption.

From [the Rust docs](https://doc.rust-lang.org/std/string/struct.String.html#method.from_raw_parts):

> The memory at ptr needs to have been previously allocated by the same allocator the standard library uses, with a required alignment of exactly 1.
> ...
> Violating these may cause problems like corrupting the allocator's internal data structures.

There is no ability in wasm to setup separate memories for the guest/host usage.
The only way to separate memory as 'ours' and 'yours' in Rust in the wasm guest
would be to do something like create a crazy global lazy static vector wrapped
in a mutex and fake a new linear memory inside the wasm (which then the host
would need some way to safely interact with).

It is clear that we need a byte-level protocol between host and guest, that also
respects the limited types (see above), to reliably share data across the host
and guest boundary.

### How we move data between the host and guest

All the following assumes:

- We have some canonical serialization for our data, as per `encode` and `decode` in `holochain_serialized_bytes`
- We have a running host and guest
- There is some crate containing all shared rust data types common to both the host and the guest

The fundamental constraint in both direction is that the _sender_ of data knows
the _length_ of the data and the _recipient_ can allocate and generate a pointer
to where the data should be copied to.

As we will be executing untrusted, potentially malicious, wasm code as a guest
we also have to require:

- The guest can never force the host to leak data beyond the lifetime of the guest
- The guest can never force the host to hand it back data outside the guest's own memory
- The guest can never force the host to write the guest's data outside the guest's own memory
- If the guest leaks or corrupts memory the leak/corruption is sandboxed to it's own memory

There are 4 basic scenarios that require data negotiation:

- Input data from the host to the guest
- Input data from the guest to the host
- Output data from the host to the guest
- Output data from the guest to the host

To handle all of these in each direction without allowing the guest to request
data on the host at a specific pointer on the host system, or overcomplicating
the protocol, we have a ptr/length u32 input and merged (bit shifted) u64 bit
output.

The merged outputs of guest functions as a u64 are to workaround the need for the
nightly Rust compiler to use the `extern "wasm"` syntax. The stable `extern "C"`
doesn't support multi value outputs, even though wasm does, so it's better to
return a single 64 bit value and treat it as 2x 32 bit values on the host.

#### How data round trips the host and the guest (details)

The host moves data into the guest when it is calling a guest function or
returning data from an imported function.

##### Host calling guest

When the host is calling into the guest it first asks the guest to provide a
pointer to freshly allocated memory, then copies length prefixed bytes straight
to this location. The host then calls the desired function on the guest passing
the pointer and length of input data as arguments for the call.

This is handled via. `host::guest::call()` on the host side and the `host_args`
macro on the guest side.

- The host moves serialized `SomeDataType` on the host using the host allocator
- The host calculates the `u32` length of the serialized data
- The host asks the guest to `__hc__allocate_1` the length
- The guest (inside `__hc__allocate_1`) allocates length bytes and returns a `GuestPtr` to the host
- The host checks that the `GuestPtr` + len bytes fits in the guest's memory bounds
- The host writes the data into the guest memory
- The host calls the function it wants to call in the guest, passing in the `GuestPtr` and `Len`
- The guest receives the `GuestPtr` and `Len` and passes both to the `host_args!` macro
- The guest deserializes `length` bytes from `guest_ptr` into whatever input type it was expecting
- The deserialization process takes ownership of the bytes inside the guest so rust will handle cleanup from here

##### Guest returning to host

On the guest this is handled by the `return_ptr`, `return_err_ptr` and
`try_ptr!` functions.

All these functions work in broadly the same way, by pushing serialized data
across the host/guest boundary, including an error representing problems doing
the same.

The `host::guest::call()` function knows what to do with the outer `Result`, the
host only needs to line up `SomeDataType` of the guest inner return value with
the `host::guest::call()` return value.

- The guest calls one of the `return_ptr` style functions with something `Serialize`
- Internal to `return_ptr` et. al. a `Result` is built out of serializable data or serializes an error
- The `Result` bytes are leaked into the guest
- The guest returns a `GuestPtrLen` to the host referencing the bytes
- The host copies the bytes from `GuestPtrLen` and deserializes the `Result`
- The host calls `__hc__deallocate_1` so that the guest can cleanup the leaked data
- The host deserializes the inner value if it makes sense to

##### Guest calling host

On the guest side this is the first half of the `host_call` macro.

The host side uses the `host::guest::from_guest_ptr()` function that reads bytes
straight from the shared memory based on a `GuestPtr` the guest passes to the
host.

- The guest moves serialized `SomeDataType` into memory
- The guest leaks the serialized data to get a `GuestPtr` and `Len`
- The guest calls the host function with the `GuestPtr` and `Len`
- The host reads, deallocates and deserializes `SomeDataType` from the guest's memory
- Note: due to a limitation in wasmer it is not possible for the host to call
  back into the guest during an imported function call, so at this point the
  input is still leaked on the guest

##### Host returning data from imported function to guest

On the guest side, this is the second half of the `host_call` function.

This is handled by each imported function on the host side.

The expectation is that holochain core implements sensible macros to normalize
this alongside the HDK and internal workflow implementations.

- The host function does whatever it does as native rust
- The host function final value is `SomeDataType`
- The host serializes the return value to a `Vec<u8>`
- The host requests that the guest allocates byte for the length of the serialized value
- The host clones the bytes to the pointer returned by the guest
- The host returns the pointer and length of the serialized data to the guest as a u64 merged value
- The guest then calls `consume_bytes` with the split u64 into 2x u32 values which reads and deserializes the value

###### Mitigating leaked data by the guest

In general guests can allocate and leak as much memory as they want if they are
buggy or malicious. This is up to 4GB per instance.

Hosts are strongly recommended NOT to cache instances arbitrarily and set some
maximum memory usage on the instance cache.
