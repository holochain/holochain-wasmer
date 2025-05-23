# Changelog
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.100] - 2025-05-16

- Updated to wasmer v6.
- **BREAKING CHANGE** Functions that are no longer used by holochain have been removed: `build_ios_module` and `get_ios_module_from_file`. Their only use was deprecated in holochain 0.5 and has been removed in 0.6.

## [0.0.99] - 2025-01-16

- Modify the `ModuleCache::new` constructor to no longer take a `ModuleBuilder` parameter.
- Add a `ModuleCache::new_with_builder` constructor which does take a `ModuleBuilder` parameter.

## [0.0.98] - 2025-01-15

### Changed
- Removed separate cargo workspaces for `crates/guest` and `test` and separate cargo projects for test wasms. Now all crates are members of a single cargo workspace.
- Re-enable CI testing on windows with feature `wasmer_wamr`.
- Removes the in-memory serialized module cache which was redundant. Now there is just a `ModuleCache` which stores modules in-memory as well as optionally persisting modules by serializing and saving them to the filesystem.

### Added
- Add CI job "check" which passes if all other jobs pass.
- When the `ModuleCache` fails to deserialize a Module that was perisisted to the filesystem, the persisted file is deleted and the original wasm is added to the cache again.

## [0.0.97] - 2024-12-18

### Changed
- Bumped wasmer version to 5.x
- **BREAKING CHANGE** The `wasmer_sys` feature has been renamed to `wasmer_sys_dev`
- The error variant `WasmErrorInner::Compile` has been renamed to `WasmErrorInner::ModuleBuild` to clarify that the error is related to constructing a wasmer `Module`. Only with the feature flags `wasmer_sys_dev` or `wasmer_sys_prod`, is this when wasm compilation occurs. On the feature flag `wasmer_wamr`, wasms are interpreted and thus no compilation occurs.

### Added
- A new feature flag, `wasmer_sys_prod` which enables the Wasmer LLVM compiler. The default, with the `wasmer_sys_dev` feature
  is the Cranelift compiler. The Cranelift compiler is fast, and recommended for development, but the LLVM compiler is supposed
  to be faster and more optimized for production. In testing so far, the compile step is slower with LLVM but the runtime is
  faster. More testing is needed yet to confirm the difference.
- A new public function `build_module`, which builds a wasmer Module directly, bypassing the `ModuleCache`. It is only implemented for the feature flag `wasmer_wamr`. On the feature flags `wasmer_sys_dev` and `wasmer_sys_prod` it will panic as unimplemented. This enforces the use of the `ModuleCache` when wasmer is used in a compiled mode, and allows bypassing the cache when wasmer is used in interpreter mode as caching is not relevant.

## [0.0.96] - 2024-08-28

### Added
Two new mutually-exclusive feature flags `wasmer_sys` and `wasmer_wamr` for toggling different wasm runtime engines:
- `wasmer_sys` sets wasmer to use the cranelift compiler
- `wasmer_wamr` sets wasmer to use the wasm-micro-runtime in interpreter mode

### Changed
- Use the full path to `WasmError` within the `wasm_error!` macro, so that the consumer does not need to manually import `WasmError`.

## [0.0.95] - 2024-08-28

### Changed
- Bumped holochain_serialized_bytes version
- Bumped wasmer version

## [0.0.94] - 2024-05-21

### Changed
- Fixed memory deallocation for rust 1.78
- Bump Criterion version

## [0.0.93] - 2024-04-24

### Changed
- **BREAKING CHANGE:** Holochain serialization updated to v0.0.54 which in turn contains a breaking change in how the conductor API serializes enums.

## [0.0.92] - 2024-01-16

### Added
- Deserialized module cache `DeserializedModuleCache` was reinstated.
- An abstraction for caching (serialized & deserialized modules) called `ModuleCache` was added.
- All logic related to modules and wasmer caching from `holochain` has been moved to the host crate. Consequently functions for wasmer development under iOS need to be imported from there.

### Removed
- **BREAKING CHANGE:** Instance cache in host crate has been removed in favor of a deserialized module cache `DeserializedModuleCache`.

## [0.0.90]

- Bump wasmer to 4.2.4

## [0.0.87]

- support file cache for serialized modules
- host function metering support

## [0.0.86]

- Support wasmer 4.x

## [0.0.73] - 2021-07-20

### Added

### Changed

- Bumped holochain_serialized_bytes version

### Deprecated

### Removed

### Fixed

### Security

## [0.0.72] - 2021-07-02

### Added

- Support for 3 level serialized -> deserialized -> instance caching with PLRU

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.71] - 2021-06-23

### Added

- `HostShortCircuit` variant for `WasmError`
- moved a lot of memory handling to the `WasmerEnv` handling
- added a simple `MODULE_CACHE` as a status

### Changed

- Uses wasmer 1+
- Uses latest holonix
- Externs follow (ptr, len) -> ptrlen as (u32, u32) -> u64
- all guest functions are `#[inline(always)]`

### Deprecated

### Removed

### Fixed

- [PR#66](https://github.com/holochain/holochain-wasmer/pull/66) - workaround a memory leak in (our usage of) wasmer

### Security

## [0.0.67] - 2021-02-21

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.66] - 2021-02-09

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.65] - 2021-02-02

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.64] - 2021-01-20

### Added

### Changed

- Changed `SerializedBytes` to `holochain_serialized_bytes::encode()` globally

### Deprecated

### Removed

- Removed the `Cargo.lock` file

### Fixed

### Security

## [0.0.54] - 2021-01-06

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.53] - 2021-01-06

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.53] - 2020-12-22

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.52] - 2020-12-17

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.51] - 2020-12-16

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.50] - 2020-11-19

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.49] - 2020-11-19

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.48] - 2020-11-16

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.47] - 2020-10-29

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.46] - 2020-09-18

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.45] - 2020-09-18

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.45] - 2020-08-28

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.44] - 2020-08-28

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.45] - 2020-08-26

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.42] - 2020-08-23

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.41] - 2020-08-17

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.40] - 2020-08-06

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.39] - 2020-08-05

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.38] - 2020-08-05

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.37] - 2020-07-31

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.36] - 2020-07-31

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.36] - 2020-07-13

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.35] - 2020-07-13

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.35] - 2020-07-07

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.34] - 2020-07-03

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.33] - 2020-06-19

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.32] - 2020-06-17

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.31] - 2020-05-29

### Added

- added holochain_externs!() macro

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.30] - 2020-05-10

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.29] - 2020-05-02

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.28] - 2020-05-02

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.27] - 2020-05-02

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.26] - 2020-05-02

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.25] - 2020-05-02

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.24] - 2020-04-04

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.22] - 2020-04-01

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.21] - 2020-03-31

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.20] - 2020-03-27

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.19] - 2020-03-21

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.18] - 2020-03-09

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.17] - 2020-03-08

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.15] - 2020-03-02

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.14] - 2020-02-20

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.13] - 2020-02-17

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.12] - 2020-02-15

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.11] - 2020-02-15

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.10] - 2020-02-13

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.9] - 2020-02-10

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.7] - 2020-02-10

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.6] - 2020-02-10

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.5] - 2020-02-10

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.4] - 2020-02-10

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security
