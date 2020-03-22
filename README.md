# holochain-wasmer

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
- [holonix](https://github.com/holochain/holonix): specifically the nightly Rust version management and wasm tooling

## Background information

I won't attempt a comprehensive guide to wasm here, it's a huge topic.

There are some key things to understand or this crate won't make sense.

### WASM data types are very limited

WASM only has 4 data types: `i32`, `i64`, `f32` and `f64`.

This represents integers and floats.

Integers are '[sign agnostic](https://rsms.me/wasm-intro#sign-agnostic)' which can be awkward in Rust, that only has signed
primitives. This basically means that integers are just chunks of binary data
that allow contextual math operations. For example, nothing in wasm prevents
us from performing signed and unsigned math operations on the same number. The
number itself is not signed, it's just that certain math requires the developer
to adopt consistent _conventions_ in order to write correct code.

By contrast, Rust doesn't even let us represent `i64` and `u64` in the same part
of our codebase, we must always be completely unambiguous about which type every
value is.

Wasm floats show some [non-deterministic behaviour](https://webassembly.org/docs/nondeterminism/) in the case of `NaN` values.

Non-determinism is very scary in the context of building a p2p network because
it means we cannot differentiate clearly between honest and dishonest actors
based on individual pieces of data. At best we can apply statistical heuristics
across many data points that are costly and can be gamed or avoided by attackers.

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
different allocation models. Notably, [custom allocators](https://doc.rust-lang.org/1.15.1/book/custom-allocators.html) are common
in WASM, such as [wee_alloc](https://github.com/rustwasm/wee_alloc) designed to
keep the overall footprint small.

Even 'simple' primitives like `String` are not safe to round trip through their
'raw parts' (e.g. length, capacity and pointer for `String`) if the allocator
is different at source and destination.

It's not even clear whether a mismatch in 'nightly' compiler versions constitutes
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

- We have some canonical `SerializedBytes` for our data, as per `holochain_serialized_bytes`
- We have a running host and guest
- There is some crate containing all shared rust data types common to both the host and the guest

The fundamental constraint in both direction is that the _sender_ of data knows
the _length_ of the data and the _recipient_ is managing the memory that data
will be copied into.

Here is a simplified explanation of how we navigate all the limitations above:

- The sender converts `SomeDataType` into `SerializedBytes` of length `len`
- The sender requests that an allocation of length `len` be made
- The receiver returns a pointer `ptr` where `len` has been safely allocated (won't be automatically dropped and re-used by Rust)
- The sender copies `SerializedBytes` to the receiver, notifies the receiver
- The receiver does an `unsafe` read of `SerializedBytes` out of `ptr` and converts it to `SomeDataType`
- The receiver notifies the sender that it has a native type with ownership of the bytes
- The sender drops its copy of the data to free up memory

Note though, that the above is _not possible in wasm_ exactly as written as the
guest has no ability to copy bytes to the host.

What we have instead is a multi-tiered set of data structures that move closer
to something that can be sent/received in a wasm function in one direction
(i.e. a wasm `i64`) but are also moving further away from the actual bytes and
data structures.

`SomeDataType` > `SerializedBytes` > `Allocation` > `AllocationPtr` > `RemotePtr`

`SomeDataType` represents any shared rust data type in the crate common to both
the host and the guest, that has a canonical `SerializedBytes` round-trip.

`SerializedBytes` comes from the `holochain_serialized_bytes` crate and is a
zero cost wrapper around a `Vec<u8>` that we can copy byte-by-byte in a
deterministic way.

`Allocation` is a `[u64; 2]` fixed-length slice as `[offset, length]`. This is
a slice rather than e.g. some kind of encoding scheme in a single `u64` as we
need the `offset` to be a `u64` to support 64 bit _hosts_. We need something of
a known, fixed length to move between host/guest in a 'preflight' to co-ordinate
the copying of bytes of arbitrary length.

`AllocationPtr` is a new type that wraps a single `u64` which is a pointer to
an `Allocation`. It exists so that we can implement `From` round-trips through
`SerializedBytes` such that the compiler can give us guide rails around calling
`mem::ManuallyDrop` and `unsafe { Vec::from_raw_parts() }` symmetrically.

Moving `From<SerializedBytes>` to `AllocationPtr` leaves both the bytes and an
`Allocation` leaked in memory to be read and manually dropped later.

Moving `From<AllocationPtr>` to `SerializedBytes` drops both the `AllocationPtr`
and intermediate `Allocation` and gives ownership of the bytes to `SerializedBytes`.

`AllocationPtr` does NOT implement `Clone` _by design_ so that functions are
forced to take ownership, including `From`. This means the compiler helps us
achieve 1:1 round-trips everywhere and not read or write to the wrong place.

`RemotePtr` is just a type alias to `u64`. It represents something that can be
passed across the wasm host/guest boundary either as function inputs or outputs.
As a Rust primitive we have little ability to extend or control its behaviour
(e.g. forcing it to not be copied/cloned/reused) so we hide it behind
`allocation_ptr.as_remote_ptr()` and `AllocationPtr::from_remote_ptr()` methods
to try and make it as clear as possible when we move to it.

#### How data round trips the host and the guest (details)

The host moves data into the guest when it is calling a guest function or
returning data from an imported function.

##### Host calling guest

This is handled via. `host::guest::call()` on the host side and the `host_args!`
macro on the guest side.

- The host moves `SomeDataType` into `SerializedBytes` on the host using the host allocator
- The host moves `SerializedBytes` into `AllocationPtr`, leaving the bytes and an `Allocation` leaked on the host
- The host passes a `RemotePtr` to a guest function
- The guest receives the `RemotePtr` from the host and calls the `host_args!` macro
- Internally the guest calls `guest::map_bytes()` with the `RemotePtr`
- `map_bytes` creates a fake `Allocation` on the guest and passes the `AllocationPtr` for it back to the host
- The host writes the host `Allocation` (including the host ptr/len) directly over the fake guest `Allocation`
- The guest allocates `len` bytes based on the host's allocation and now has a guest ptr
- The guest asks the host to copy `len` bytes from the host's ptr to the guest ptr
- The host drops its copy of the bytes and the `Allocation` that were leaked, after writing them to the guest
- The guest can now build a meaningful `Allocation` and `AllocationPtr` from the `len` and guest ptr
- The guest uses the populated `AllocationPtr` to build `SerializedBytes`
- The guest attempts to build `SomeDataType` from the `SerializedBytes`

##### Guest returning to host

On the guest this is handled by the `ret!`, `ret_err!` and `try_result!` macros.

There is a shared `WasmResult` enum that can either be `WasmResult::Ok<SerializedBytes>`
or `WasmResult::Err<WasmError>` where `WasmError` is another enum including some
basic variants.

All these macros work in broadly the same way, by wrapping some data in an enum
to fake a rust native `Result`.

The `host::guest::call()` function knows what to do with the `WasmResult`, the
host only needs to line up `SomeDataType` of the guest inner return value with
the `host::guest::call()` return value.

- The guest calls one of the `ret!` style macros with `SomeDataType`
- Internal to `ret!` et. al. a `WasmResult` is built out of (maybe) `SerializedBytes` or just an error, leaving data leaked on the guest
- The macro returns a `RemotePtr` to the host
- The host reads `Allocation` and `SerializedBytes` directly from the host shared memory (see below)
- The host calls the `__deallocate_return_value` function inside the guest with the same guest `RemotePtr`
- The guest frees the data leaked earlier
- The host does a `try_into()` from the _inner_ `SerializedBytes` _if_ the return was a `WasmResult::Ok<SerializedBytes>`

##### Guest calling host

On the guest side this is the first half of the `host_call!` macro.

The host side uses the `host::guest::from_guest_ptr()` function that reads bytes
straight from the shared memory based on a `RemotePtr` the guest passes to the
host.

- The guest moves `SomeDataType` into `SerializedBytes`
- The guest moves `SerializedBytes` into an `AllocationPtr` with leaked bytes and `Allocation` on the guest
- The guest calls the host with `RemotePtr` from its `AllocationPtr`
- The host reads `Allocation` directly from the guest memory using `RemotePtr`, it can do this because `Allocation` is of known length (2x `u64`)
- The host reads the bytes directly from the guest memory using the offset/length it read from the guest `Allocation`
- The host builds `SerializedBytes` from the bytes it read from the guest
- The host builds `SomeDataType` from `SerializedBytes` and treats it as the input to the imported host function
- Note: due to a limitation in wasmer it is not possible for the host to call back into the host during an imported function call, so at this point bytes and `Allocation` are still leaked on the guest with no way to free them
- After the host call returns, the `host_call!` macro frees the previously leaked bytes and `Allocation`

##### Host returning data from imported function to guest

On the guest side, this is the second half of the `host_call!` macro.

This is handled by each imported function on the host side.

The expectation is that holochain core implements sensible macros to normalize
this alongside the HDK and internal workflow implementations.

- The host function does whatever it does as native rust
- The host function return value is `SomeDataType`
- The host converts the return value to `SerializedBytes`
- The host converts the `SerializedBytes` to an `AllocationPtr` with leaked bytes and `Allocation`
- The host returns a `RemotePtr` to the guest
- The guest, as part of the `host_call!` macro, calls `map_bytes`
- `map_bytes` functions as explained above, freeing the bytes and `Allocation` from the host and giving `SerializedBytes` to the guest
- The guest converts `SerializedBytes` into `SomeDataType` and does a `try_into()` straight out of `host_call!`
