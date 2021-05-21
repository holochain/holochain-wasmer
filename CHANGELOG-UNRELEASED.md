# Changelog
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

{{ version-heading }}

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

### Security

