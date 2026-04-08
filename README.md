# holochain-wasmer

## How to use the Nix development flake

This repository provides a Nix flake in `./flake.nix`. It has a devShell that provides required libraries and tools for 
developing on this project. You can access the dev shell with:

```shell 
nix develop
```

The version of Rust is controlled by `./rust-toolchain.toml` which will apply to users of Cargo outside the Nix 
environment, and equally is loaded by the flake so it applies to users of the dev shell.

To update the flake dependencies, run the following command:

```bash
nix flake update
```

The Nix packages version should be checked occasionally and updated. There should be a new one roughly every 6 months.
Prefer to use the latest stable Nix packages version.

## Why?

Doing certain high level things in WASM is still pretty hard to get right, even
with Rust.

Luckily Rust provides us with enough tools to abstract what we need at the
compiler level. Most details are not visible downstream to hApp devs and can be
achieved via 'zero cost abstraction'.

That said, there _are_ some limitations by design that we enforce here to allow
the whole WASM stack to be simple to maintain and understand.

The Rust compiler makes many things sane for us in WASM but there are a few
notable things that are left up to us:

- Define a clear interface between the "host" and the "guest" within WASM limits
- Manage a shared memory across the host/guest with different Rust allocators
- Inject additional runtime context on the host that the guest cannot provide
- Performance optimisations

## What

This repository consists of 3 main library crates:

- `holochain_wasmer_common`: host/guest agnostic and shared functionality
- `holochain_wasmer_guest`: essential macros for WASM guests
- `holochain_wasmer_host`: infrastructure to manage a WASM guest

There is also a `test-crates` directory containing analogous crates implementing the
above libraries for the purpose of testing and simple working examples.

- `test-crates/common`: data structures shared by the host and guest
- `test-crates/wasms`: multiple sample Rust WASM projects to be used in tests
- `test-crates/tests`: tests and benchmarks, with a custom build script to build the WASM projects

The main dependencies are:

- [wasmer](https://wasmer.io/): one of the best wasm implementations for Rust
- [holochain_serialization](https://github.com/holochain/holochain-serialization): our crates to normalize serialization at the byte level

## How to use

There are several pieces involved in running wasm under Holochain. The detailed
API documentation lives on docs.rs and in the rustdoc of each crate; this
README sticks to the conceptual model and points at the right surface for each
question.

- The host (Holochain core, or any embedder) loads and instantiates wasm
  modules. The primary entry point on the host side is
  [`holochain_wasmer_host::module::ModuleCache`][hwh-modcache], which caches
  compiled modules in memory and (optionally) on the filesystem so the same
  wasm doesn't get recompiled on every call.
- The host invokes guest functions with
  [`holochain_wasmer_host::guest::call`][hwh-call], which handles the
  serialization and pointer dance across the host/guest boundary.
- The host exposes "imported functions" to the guest as a wasmer
  [`Imports`][wasmer-imports] object. Wasmer's
  [`imports_function`][wasmer-imports-example] example is a good reference for
  the wasmer-side mechanics; this crate's host-side test code in
  [`test-crates/tests/src/import.rs`][hwh-imports] shows how that's wired up
  against `holochain_wasmer_host` in practice.
- The guest side (typically a Holochain zome) is documented as crate-level
  rustdoc in the [`holochain_wasmer_guest`][hwg] crate, including all the
  macros and helpers a guest needs to receive arguments, call host functions
  and return values. Most zome authors don't write to this layer directly —
  the [Holochain HDK](https://docs.rs/hdk) hides it.

### Worked examples

The end-to-end test setup in this repository is the most up-to-date reference
for how all the pieces fit together. CI exercises every code path on every PR,
so unlike a snippet in this README the examples there cannot quietly rot:

- [`test-crates/tests/src/import.rs`][hwh-imports] — host-side imports and the
  host functions a test wasm can call back into.
- [`test-crates/tests/src/test.rs`][hwh-test] — host-side test harness driving
  guest functions via `guest::call`.
- [`test-crates/tests/src/wasms.rs`][hwh-wasms] — module construction and the
  `ModuleCache` setup the tests use.
- [`test-crates/wasms/wasm_core/src/wasm.rs`][hwh-core-wasm] — a guest wasm
  that exercises every macro in `holochain_wasmer_guest`.

[hwh-modcache]: https://docs.rs/holochain_wasmer_host/latest/holochain_wasmer_host/module/struct.ModuleCache.html
[hwh-call]: https://docs.rs/holochain_wasmer_host/latest/holochain_wasmer_host/guest/fn.call.html
[hwg]: https://docs.rs/holochain_wasmer_guest
[wasmer-imports]: https://docs.rs/wasmer/latest/wasmer/struct.Imports.html
[wasmer-imports-example]: https://github.com/wasmerio/wasmer/blob/main/examples/imports_function.rs
[hwh-imports]: ./test-crates/tests/src/import.rs
[hwh-test]: ./test-crates/tests/src/test.rs
[hwh-wasms]: ./test-crates/tests/src/wasms.rs
[hwh-core-wasm]: ./test-crates/wasms/wasm_core/src/wasm.rs

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

This is handled via `host::guest::call()` on the host side and the `host_args`
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

The host side reads bytes straight from the shared memory based on a `GuestPtr`
the guest passes to the host, using the host crate's internal byte-reading
helpers wrapped by each imported function.

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
