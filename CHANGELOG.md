# Changelog
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- Refactor: Instance cache in host crate has been removed in favor of a deserialized module cache `DeserializedModuleCache`. An abstraction for caching (serialized & deserialized modules) called `ModuleCache` was added.
- Refactor: All logic related to modules and wasmer caching from `holochain` has been moved to the host crate. Consequently functions for wasmer development under iOS need to be imported from there.

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
