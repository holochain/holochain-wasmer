# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## \[[0.0.103](https://github.com/holochain/holochain-wasmer/compare/v0.0.102...v0.0.103)\] - 2026-04-08

### Features

- \[**BREAKING**\] Rename feature flags to kebab-case by @synchwire in [#181](https://github.com/holochain/holochain-wasmer/pull/181)
  - The cargo feature flags in `holochain_wasmer_common` and `holochain_wasmer_host` were defined in snake_case (`wasmer_sys_cranelift`, `error_as_host`, etc). Cargo accepts both conventions but kebab-case is the documented Cargo recommendation and is what the rest of the crates published from this org use. The inconsistency was paper cuts every time someone copied a feature list between Cargo.toml files.
  - Rename, with no behavioural changes:
  - Error_as_host        -> error-as-host - debug_memory         -> debug-memory - wasmer_sys           -> wasmer-sys - wasmer_sys_cranelift -> wasmer-sys-cranelift - wasmer_sys_llvm      -> wasmer-sys-llvm - wasmer_wasmi         -> wasmer-wasmi
  - Touches the three `[features]` tables, every `cfg(feature = "...")` site in the host and common crates and the test harness, the user- facing doc comments that name the features, both GitHub workflows, and the per-backend test/bench scripts (which are also git-mv'd to the new names so the workflow matrix entries continue to resolve).
  - This is a breaking change to the public cargo feature graph. Downstream consumers must update their `Cargo.toml` feature lists accordingly. This lands alongside the other breaking changes already queued for the next release.
- \[**BREAKING**\] Make wasmer backend feature flags additive by @synchwire in [#180](https://github.com/holochain/holochain-wasmer/pull/180)
  - The cargo feature flags for the wasmer backends were defined as mutually exclusive, with a `compile_error!` in the host crate's lib.rs forbidding any combination of `wasmer_sys` and `wasmer_wasmi`. That violates Rust's additivity contract for cargo features: in a dependency graph where crate A transitively pulls in `holochain_wasmer_host/wasmer_sys_dev` and crate B pulls in `holochain_wasmer_host/wasmer_wasmi`, cargo unifies the feature set and the resulting build doesn't compile. Today the only consumer is holochain so this hasn't bitten anyone, but it's a real footgun for any future caller.
  - Verified empirically that wasmer 7.x supports enabling its own backends (`sys`, `cranelift`, `llvm`, `wasmi`) simultaneously: the upstream lib.rs only forbids combinations of the `*-default` umbrella features, which we don't use. So the constraint is purely on our side and can be removed.
  - This commit restructures the host crate so every backend feature can be enabled in the same build:
  - Drop the `compile_error!` mutex in lib.rs and replace it with two   weaker checks: "at least one backend must be enabled" and "if   wasmer_sys is enabled, at least one of its compiler sub-features   must be enabled". - Rename `wasmer_sys_dev` -> `wasmer_sys_cranelift` and   `wasmer_sys_prod` -> `wasmer_sys_llvm`. The dev/prod naming was   misleading — these are compiler choices, not deployment modes — and   the rename makes it possible to have both compilers enabled at once. - Make `wasmer_sys_cranelift` and `wasmer_sys_llvm` independent   cargo features. `wasmer_sys` is the umbrella that pulls in   `wasmer-middlewares` and `wasmer/sys`; the compiler sub-features   add their respective compiler crate. - Restructure the host module layout: `module/wasmer_sys.rs` ->   `module/sys.rs` exposed as `pub mod sys`, `module/wasmer_wasmi.rs`   -> `module/wasmi.rs` exposed as `pub mod wasmi`. Drop the   `pub use ...::*` glob re-exports that previously made the engine   factories appear at `module::make_engine` etc — those globs were   the symbol-collision mechanism that necessitated mutual exclusion.   The factories now live at fully-qualified   `module::sys::make_cranelift_engine` /   `module::sys::make_llvm_engine` / `module::wasmi::make_engine`. - Split the sys engine factory into per-compiler functions   (`make_cranelift_engine`, `make_llvm_engine`) so both compilers   can coexist in the same build. The metering and tunables wiring   is shared via small helpers `configure_compiler` and   `apply_tunables`. A `make_engine` shorthand picks Cranelift if   it's enabled and falls back to LLVM otherwise — convenient for the   test harness, equivalent to the old single-compiler shape. - `ModuleBuilder::new` now takes both `make_engine` and   `make_runtime_engine` as explicit `fn() -> Engine` parameters.   Previously it relied on a glob re-export to resolve a single   `make_runtime_engine` symbol; making the choice an explicit   parameter is what shifts the backend decision from compile time to   the call site. - `ModuleCache::new` gains the same two parameters and forwards them   to `ModuleBuilder::new`. The convenience constructor stays useful   for callers that don't need a custom builder, but no longer hides   the backend choice.
  - The test harness (`test-crates/tests/src/wasms.rs`) is updated so the `module()` and `instance()` methods are gated as "sys when sys is available, wasmi otherwise". With both backends enabled, sys takes priority because it's the metered backend and exercises more of the codepaths under test; the wasmi-only matrix leg covers wasmi's runtime behaviour. The cfg_attr ignore on `tests::short_circuit` is updated to fire only on the wasmi-only path, matching the upstream wasmer 7.1.0 wasm_trap_new bug (wasmerio/wasmer#6397) it works around.
  - CI changes: - The test-and-bench, test-windows and android-mobile matrices are   renamed to use the new feature names. - A new `test-all-backends` job is added to the test workflow that   builds and runs the host crate with every backend feature enabled   simultaneously. This is the CI guarantee that the additivity   property doesn't regress.
  - Scripts `{test,bench}-wasmer_sys_dev.sh` and `{test,bench}-wasmer_sys_prod.sh` are renamed to `{test,bench}-wasmer_sys_cranelift.sh` and `{test,bench}-wasmer_sys_llvm.sh` to match the feature rename, and the dispatcher scripts `test.sh` / `bench.sh` are updated.
  - This is a breaking change to the public cargo feature graph and the host crate's public module path. Consumers that today depend on `holochain_wasmer_host` with `features = ["wasmer_sys_dev"]` need to update to `features = ["wasmer_sys", "wasmer_sys_cranelift"]` (the equivalent of the old default), and any code that calls `module::make_engine()` or `module::build_module()` directly needs to qualify those calls with the appropriate backend submodule (`module::sys::make_cranelift_engine()` etc). This aligns with the upcoming holochain major release that's already absorbing other breaking changes.
- \[**BREAKING**\] Replace WasmError.file with module_path by @synchwire in [#179](https://github.com/holochain/holochain-wasmer/pull/179)
  - `WasmError` previously carried a `file: String` populated by the `file!()` macro at the call site of `wasm_error!`. `file!()` produces whatever path rustc has for the source file, which for path dependencies and registry crates is an absolute filesystem path baked in at compile time. The result was that wasm guest errors shipped to end users showed paths like `/home/matt/Projects/Holochain/.../src/lib.rs` or `/home/$USER/.cargo/registry/src/index.crates.io-.../src/lib.rs` — the build machine's directory layout, leaked across the wasm boundary.
  - Replace the field with `module_path: String`, populated from [`std::module_path!`]. The Rust module path of the call site is machine-independent, includes the crate name automatically, and works the same regardless of how the calling crate was pulled in (in- workspace, path dep, registry). The two macros that constructed `WasmError` (`wasm_error!` in the common crate and `wasm_host_error!` in the host crate) are updated in lockstep, the destructure in `crates/host/src/guest.rs` and the literal-construction assertions in `test-crates/tests/src/test.rs` follow.
  - While touching the type, also replace `WasmError`'s `Display` impl — which previously just delegated to `Debug` — with a deliberate `module::path:line: <inner>` format. This decouples Display from Debug, makes the rendered errors consistent with how panics and tracing spans report call sites, and gives consumers a stable user-facing format we can iterate on independently of the struct's debug representation.
  - This is a breaking change to the public `WasmError` API. The wire format of serialized errors changes too; old serialized payloads will not deserialize against the new struct. Both are intentional and align with an upcoming Holochain release that doesn't preserve any application data.
- *(host)* Add wasmer_wasmi backend (#168) by @synchwire
  - Adds the pure-Rust `wasmi` interpreter as a third holochain_wasmer_host backend alongside `wasmer_sys` and `wasmer_wamr`, exposed via the new `wasmer_wasmi` cargo feature. wasmi is built on top of wasmer's wasm-c-api binding and gives us an iOS-buildable interpreter (the wamr backend cannot link on iOS because the upstream WAMR iOS build only ships `iwasm.dylib` and not the `vmlib` wasmer expects).
  - The three backend features (`wasmer_sys`, `wasmer_wamr`, `wasmer_wasmi`) are mutually exclusive; lib.rs is updated to enforce that and to require exactly one. The wasmi module hands out a process-wide `OnceLock<Engine>` so every module, store and instance share the same function-type registry — wasmi 1.x panics with "encountered foreign entity in func type registry" if a module from one engine is instantiated against a store backed by another.
  - Also fans the iOS mobile workflow into a matrix that builds both `wasmer_wamr` (advisory, marked `continue-on-error`) and `wasmer_wasmi` so the new backend is exercised on aarch64-apple-ios in CI.

### Bug Fixes

- *(nix)* Pin LLVM_SYS_211_PREFIX to llvmPackages_21.llvm.dev by @synchwire in [#173](https://github.com/holochain/holochain-wasmer/pull/173)
  - The previous `which llvm-config` lookup was non-deterministic — `clang` is also in the dev shell and could shadow llvm-config in PATH. Set the env var directly from the LLVM 21 dev output so it always resolves to the right LLVM version regardless of PATH ordering.

### Miscellaneous Tasks

- Update flake to latest Nix packages by @ThetaSinner
- Drop test-fuzz machinery and convert fuzz targets to seeded tests by @synchwire in [#178](https://github.com/holochain/holochain-wasmer/pull/178)
  - The `test-fuzz` setup has been in limbo since 2023: the targets were written, the developer who set them up reported finding bugs at the time, and the harness has not actually been run productively since then. The most recent commit on it was Oct 2024 — "Fix fuzz scripts, although fuzzing doesn't just work". The targets themselves (round_trip_u32 / _u64 / _usize, round_trip_allocation, alloc_dealloc, process_string_fuzz) are property tests over small, deterministic helpers; reviving the harness as-is would not find new bugs because the bugs it caught in 2023 are baked in as fixes. The kinds of inputs where fuzzing would actually pay off (structured-but-invalid wasm, mutated serialized artifacts, adversarial wire-format messages) are not what the existing targets cover.
  - Rather than perpetuate a half-deleted harness, drop it entirely:
  - Delete `scripts/fuzz.sh`, `scripts/fuzz-wasmer_sys_dev.sh` and   `scripts/fuzz-wasmer_sys_prod.sh`. - Drop the `test-fuzz` workspace dependency, the `fuzzing` cargo   feature on `holochain_wasmer_common`, and the `test-fuzz` dep   declarations in the common, guest and tests crates. Cargo.lock loses   ~210 lines as the AFL/test-fuzz dependency tree drops out. - Replace the six `#[test_fuzz::test_fuzz]` annotated functions with   plain `#[test]` fns that exercise the same property over a small   fixed seed table. The `some_*` sibling `#[test]` cases are folded   into the new seed tables. The `#[cfg(not(target_os = "windows"))]`   workarounds for trailofbits/test-fuzz#171 go away with them.
  - If at some future point the team wants to fuzz the high-value targets above, `cargo-fuzz` (libFuzzer-based, the de-facto standard) is the natural starting point — it doesn't need to inherit any of this.
- Remove docker-based fuzz pipeline by @synchwire
  - The Dockerfile is based on `holochain/fuzzbox:base`, uses `nix-shell` rather than `nix develop` (the repo migrated to flakes), and was last touched in late 2022. The accompanying `.github/workflows/build.yml` rebuilds and pushes `holochain/fuzzbox:holochain-wasmer` to Docker Hub on every push to `main`, with no documented consumer. The README points users at the same broken pipeline.
  - Remove all three. Closes #169.
- Remove wasmer_wamr backend by @synchwire
  - The wasmer_wamr backend has been a chronic source of pain — most recently the upstream WAMR iOS build only producing iwasm.dylib (prevented us from supporting iOS) and the slice::from_raw_parts UB in wasmer's wamr Function::call (forced wamr tests onto --release). With wasmer_wasmi now wired up as an iOS-buildable replacement we no longer need it, so drop it everywhere:
  - Crates/host: drop the `wasmer_wamr` cargo feature, delete   `module/wasmer_wamr.rs`, and collapse the lib.rs feature gate to   the two surviving backends. - test-crates/tests: drop the `wasmer_wamr` feature, collapse the   `any(wamr, wasmi)` cfg branches to plain `wasmer_wasmi`, and   retitle the metering stub error message. - scripts: delete `{test,bench,fuzz}-wasmer_wamr.sh` and drop the   wamr lines from the dispatcher scripts. - flake.nix: drop the wamr-only `cmake` and `ninja` packages from   the dev shell. `clang` / `libclang` / `LIBCLANG_PATH` stay   because wasmer's build script still runs bindgen against the wasmi   C API headers when the wasmi feature is enabled.
  - The two surviving backends (wasmer_sys and wasmer_wasmi) continue to pass their full test suites locally.
- Upgrade wasmer to 7.1.0 by @synchwire
  - Bumps wasmer and wasmer-middlewares from 6.0.0 to 7.1.0 (closes #167).
  - Wasmer 7.1.0 requires rustc >= 1.91, so the rust-toolchain.toml is moved from a pinned 1.85.0 to `stable`. The nix flake is updated from LLVM 18 to LLVM 21 to match wasmer's new llvm-sys 211 dependency for the production (LLVM) backend.
  - The wasmer wamr backend has a UB bug exposed by this upgrade: in `Function::call`, `slice::from_raw_parts` is invoked on the results vector without checking that the data pointer is non-null when the wasm function has zero return values, which trips Rust's stabilized unsafe-precondition check in debug builds. This is reported upstream as wasmerio/wasmer#6392; until that is fixed, the wamr workspace tests are run with --release (where the precondition check is compiled out).
  - A few incidental fixups required by the newer rustc/clippy: drop unused imports in the wamr module file and replace a `repeat().take()` with `repeat_n()` in a test wasm.
  - No source-level wasmer API changes were required.
- Replace holonix flake with standalone nix flake and add rust-toolchain.toml by @synchwire in [#172](https://github.com/holochain/holochain-wasmer/pull/172)
  - Remove the holonix dependency and add direct nixpkgs, flake-parts, and rust-overlay inputs. The Rust toolchain is now loaded from a new rust-toolchain.toml (pinned to 1.85.0) which also specifies wasm32-unknown-unknown and aarch64-apple-ios targets.
- Fix miscellaneous metadata, documentation, and config issues by @ThetaSinner in [#170](https://github.com/holochain/holochain-wasmer/pull/170)
  - Fix malformed repository URL in workspace Cargo.toml (was missing github.com) - Fix TOML spacing in crates/common/Cargo.toml (serde .workspace -> serde.workspace) - Fix wasm_memory test crate version from 0.0.84 to 0.0.90 to match other test crates - Fix README markdown code fence closure (4 backticks -> 3) - Fix "via." typos in README (should be "via") - Fix "specfic" typo in README - Fix stale llvm_15 comment in flake.nix (should be llvm_18) - Add root Apache-2.0 LICENSE file

### CI

- Install LLVM into RUNNER_TEMP, not the workspace by @synchwire in [#187](https://github.com/holochain/holochain-wasmer/pull/187)
  - The previous setup_script downloaded the LLVM bundle into `$PWD/.llvm`, which is inside `$GITHUB_WORKSPACE`. The prepare step itself ran fine and `cargo-semver-checks` passed, but the downstream `peter-evans/create-pull-request` step then tried to commit everything in the workspace into the auto-generated release PR branch — including the entire LLVM bundle. The push got rejected by GitHub's pre-receive hook because `clang-21` is 143 MB, well over the 100 MB hard file size limit:
  - Remote: error: File .llvm/bin/clang-21 is 143.01 MB; this exceeds     GitHub's file size limit of 100.00 MB
  - `$RUNNER_TEMP` is the canonical "persistent within the job, outside the workspace" location for files like this on a GitHub-hosted runner. Putting LLVM under `$RUNNER_TEMP/llvm` keeps the prefix reachable from later steps via `LLVM_SYS_211_PREFIX` while making the bundle invisible to git operations against the repo, so the release PR commit no longer accidentally vacuums it up.
  - The in-file comment is expanded to document why this matters, so the next person to look at this doesn't move it back under the workspace and rediscover the failure mode.
- Bump holochain/actions to v1.8.0 by @synchwire in [#186](https://github.com/holochain/holochain-wasmer/pull/186)
  - Picks up holochain/actions#9, which fixes the broken `if: ${{ inputs.skip_semver_checks == 'false' }}` conditional on the cargo-semver-checks install step. v1.7.0 had that conditional always evaluating false (boolean compared to a string in GitHub Actions expression syntax), so the install step was always skipped and the prepare step then died with `no such command: 'semver-checks'`. v1.8.0 evaluates the boolean directly.
  - The setup_script hook from #8 (which downloads LLVM for the `wasmer-sys-llvm` semver check) is unchanged and still wired up in this workflow file.
- Install LLVM in prepare-release via v1.7.0 setup_script hook by @synchwire in [#185](https://github.com/holochain/holochain-wasmer/pull/185)
  - Bumps the call into `holochain/actions/.github/workflows/prepare-release.yml` from `@v1.6.0` to `@v1.7.0`, which adds an optional `setup_script` input (holochain/actions#8). Use that hook to download the same prebuilt LLVM 21 bundle the Windows test job already uses and export `LLVM_SYS_211_PREFIX` into `$GITHUB_ENV` so the `Prepare release` step can build the host crate with `--all-features` enabled — including `wasmer-sys-llvm`, which transitively pulls in `llvm-sys = "211"` and fails out without an LLVM toolchain on disk.
  - Replaces the previous tactical workaround that passed `--i-am-so-sorry-but-my-features-clash` via `extra_release_util_args` to downgrade `cargo-semver-checks` to `--default-features` only. With the LLVM toolchain available, semver checks now run against the full feature matrix again and we get the coverage that the non-default backends (`wasmer-sys-llvm`, `wasmer-wasmi`, `debug-memory`) deserve.
  - The inline comment in the workflow file documents the why so this doesn't get reverted by accident the next time someone wonders why the release workflow is downloading LLVM.
- Clean up workflows after dropping wamr by @synchwire in [#175](https://github.com/holochain/holochain-wasmer/pull/175)
- Bump Windows LLVM to 21.x for wasmer 7 by @synchwire
  - The wasmer 7.x upgrade switched the LLVM backend from llvm-sys 180 to llvm-sys 211. The Linux/macOS jobs run via the nix flake which is already updated, but the Windows jobs download a custom LLVM build directly and set LLVM_SYS_180_PREFIX. Bump both the download URL (wasmerio/llvm-custom-builds 18.x -> 21.x) and the env var name (LLVM_SYS_180_PREFIX -> LLVM_SYS_211_PREFIX).

### Testing

- *(wasmi)* Wire wasmer_wasmi into the test workspace and CI matrix by @synchwire in [#174](https://github.com/holochain/holochain-wasmer/pull/174)
  - Mirrors the existing `wasmer_wamr` plumbing in the tests crate so the same suite runs against the wasmi interpreter, plus matching `scripts/{test,bench}-wasmer_wasmi.sh` and a `wasmer_wasmi` matrix entry in the test-and-bench CI workflow.
  - The test harness builds the Store from `make_engine()` (instead of `Store::default()`) on the wasmi backend so module/store/instance all share the same engine; wasmi's per-engine function-type registry otherwise panics on instantiation. `tests::short_circuit` is ignored under `wasmer_wasmi` because wasmer 7.1.0's wasmi backend builds a wasm trap from a non-NUL-terminated byte vector in `backend/wasmi/error.rs::Trap::into_wasm_trap`, which trips a non-unwinding panic inside `wasmi_c_api_impl::wasm_trap_new` and aborts the test process. Tracked upstream as wasmerio/wasmer#6397.
  - With these in place 25/26 root-workspace tests pass on wasmi (the one ignored is `short_circuit`; `decrease_points_test` and `infinite_loop` were already ignored on the interpreter backends).

### Refactor

- \[**BREAKING**\] Tighten public API surface for 1.0 by @synchwire in [#183](https://github.com/holochain/holochain-wasmer/pull/183)
  - Working towards a 1.0 of these crates. The public API has accumulated several items over the pre-release lifetime that aren't actually consumed by anything: dead structs, vestigial stubs from old refactors, and a vendored helper module that was never used externally. Removing or tightening them now means less to support across the 1.0 stability guarantee.
  - Each removal was checked against `holochain/holochain` on `develop` (the only published consumer) to confirm no current direct usage; the breaking-change list below is what downstream callers will need to adapt.
  - Delete `holochain_wasmer_host::module::ModuleWithStore`. Defined as   `pub` but never referenced anywhere in the workspace and never   consumed by holochain. Looks like the abandoned half of an early   refactor that introduced `InstanceWithStore` alongside it; only   the instance variant ever got used.
  - Delete `holochain_wasmer_host::module::sys::build_module`. This was   a `pub fn` that always panicked with `unimplemented!()`. It existed   because the old module layout had a single top-level   `module::build_module` symbol that the wasmi side implemented and   the sys side had to match for the glob re-export to resolve. The   additive-features refactor (commit d275f1f) moved both sides under   fully-qualified `module::sys::*` / `module::wasmi::*` paths and the   glob re-export went away — at which point the sys-side stub stopped   serving any purpose, but wasn't deleted. Worse than dead: it's a   footgun, since anything that finds it via tab-completion gets a   runtime panic instead of a compile error. With the stub gone, the   wasmi backend's `module::wasmi::build_module` is the only   `build_module` in the public API and the call site is unambiguous.
  - Demote `holochain_wasmer_host::plru` from `pub` to `pub(crate)`.   This is a vendored copy of the `ticki/plru` cache crate. Only   `MicroCache` is actually consumed (by the `InMemoryModuleCache`   in `module.rs`); the other size aliases (`SmallCache`, `MediumCache`,   `BigCache`, `HugeCache`, `DynamicCache`), the `create()` constructor   and most of the `Cache<B>` methods (`new`, `trash`, `len`,   `is_empty`, `is_hot`) are never called from anywhere in the   workspace, and nothing in holochain reaches into   `holochain_wasmer_host::plru` either. There's no reason for this   to be exposed as part of the host crate's stable surface — it's a   cache implementation detail. The module body is left intact and   marked `#[allow(dead_code)]` rather than carving out the unused   half, since the file is essentially vendored upstream and tracking   drift is easier when the shape stays close to the original.
  - Demote `holochain_wasmer_host::guest::{read_bytes, write_bytes,   from_guest_ptr}` from `pub` to `pub(crate)`. These are the lower-   level memory-copy and deserialisation primitives that   `guest::call` and `env.rs` use under the hood. They were exposed   as `pub` but the only actual consumer in the entire workspace is   the host crate itself, and holochain only reaches for `guest::call`   (also via the prelude). Tightening these to `pub(crate)` makes   `guest::call` the unambiguous supported entry point and means we   don't need to keep the lower-level signatures stable across 1.x   point releases.
  - Note: README.md mentions `host::guest::from_guest_ptr` as part of   a usage walkthrough. That reference is now stale and will be   updated separately when the README is rewritten — flagging it   here so it doesn't get missed.
  - The ribosome-side adaptation in holochain is straightforward:
  - `module::ModuleWithStore` — was unused; nothing to migrate. - `module::sys::build_module` — was unused; if anything ever called   it, it would have panicked. Switch to `module::wasmi::build_module`   if you genuinely want the wasmi direct-build path, or use   `ModuleCache` for the sys path. - `plru::*` — was unused; nothing to migrate. Vendor your own copy   if you somehow needed it. - `guest::{read_bytes, write_bytes, from_guest_ptr}` — replace with   `guest::call`, which is the supported entry point and what every   current ribosome host_fn already uses via `prelude::*`.
  - `cargo check` and `cargo test` clean across the host and common crates and the test workspace, including the all-backends-enabled matrix.

### Documentation

- Trim README and move guest content into rustdoc by @synchwire in [#184](https://github.com/holochain/holochain-wasmer/pull/184)
  - The "Holochain core" and "Being a good wasm guest" sections of the README had been steadily decaying since the wasmer 1.x era and documented an API that no longer exists: a `host::instantiate()` function that was never replaced after the additive-features refactor, the wasmer 1.x `&mut Ctx` host-function shape (current wasmer is `FunctionEnv` / `FunctionEnvMut`), the long-renamed `ImportObject` type (current wasmer is `Imports`), and method signatures that no longer typecheck against the current `Env` impl. README examples aren't compiled by `cargo test` so there's no mechanism that catches this kind of drift, and updating the snippets in place wouldn't fix the underlying problem — the next refactor would re-rot them.
  - Restructure as follows:
  - Replace the entire stale `### Holochain core` and   `### Being a good wasm guest` subtree (lines 73–445 of the old   README) with a much shorter `## How to use` section that names the   two host-side entry points (`ModuleCache` and `guest::call`),   cross-links them to their docs.rs rustdoc, points at the test   crates as the canonical worked examples, and defers guest-side   documentation to the guest crate's own rustdoc. Net README size   goes from 681 lines to 349.
  - Keep all the conceptual content (`## Why?`, `## What`,   `## Background information` and its subsections about wasm data   types, memory model, and the host↔guest data protocol). That   material is timeless, hard to find anywhere else, and was the   reason the README is worth keeping at all. Only the in-prose   reference to `host::guest::from_guest_ptr()` in the "Guest calling   host" section is reworded to talk about "the host crate's internal   byte-reading helpers" instead, since that helper is being   tightened to `pub(crate)` in #183 and the README should not name   it as a public API.
  - Adapt the guest-side walkthrough into a crate-level `//!` doc on   `holochain_wasmer_guest`. Same conceptual sections (declaring   externs, writing extern functions, receiving input, calling host   functions, returning to host, immediate-error rule), rewritten   against the current API rather than the wasmer 1.x version that   was in the README. The macro examples use the actual   `host_externs!(name:version)` shape, the extern functions use the   current `(usize, usize) -> DoubleUSize` signature, and `host_call`   is shown both standalone (returning `Result`) and wrapped in   `try_ptr!` for use inside an extern. Doctests are marked `ignore`   because guest code targets `wasm32-unknown-unknown` and isn't   meaningfully runnable as a host-side doctest, but they document   the same set of patterns the README used to spell out.
  - Add a runnable doctest to `ModuleCache::new` showing the simplest   end-to-end use: build a `ModuleCache` against the cranelift sys   backend, compile a trivial wat module via `wasmer::wat2wasm`, and   look it up by cache key. Gated on `wasmer-sys-cranelift` so the   same example renders correctly under the docs.rs feature set   configured in #182. This is the first runnable example a   newcomer browsing the docs.rs landing for the host crate will   see.
  - The dead links to docs.wasmer.io that the previous commit on this branch left in place are also fixed: `docs.wasmer.io/integrations/...` returned 404, replaced with the live equivalents in the [wasmerio/wasmer examples directory] and the [wasmer rustdoc on docs.rs]. The link to the long-abandoned `holochain/holochain-rust` PR is dropped in favour of the current test-crates examples.
  - `cargo doc` is verified clean for any warning introduced by these changes (the few remaining are pre-existing in `guest.rs`/`env.rs` and the unresolved `wasmi::*` / `sys::make_llvm_engine` links that will be fixed by the in-flight #182 once it merges). `cargo test --doc` clean across host and guest crates: the new `ModuleCache::new` doctest passes; the six guest doctests are correctly ignored.
  - [wasmerio/wasmer examples directory]: https://github.com/wasmerio/wasmer/tree/main/examples [wasmer rustdoc on docs.rs]: https://docs.rs/wasmer/latest/wasmer/
- Update README which has lagged behind changes to the code by @ThetaSinner
- Document feature flags and tidy crate docs for 1.0 by @synchwire in [#182](https://github.com/holochain/holochain-wasmer/pull/182)
  - Working towards a 1.0 of these crates. Several pre-1.0 untidiness points around documentation that should be fixed before declaring the API stable.
  - Add a crate-level `//!` doc to `holochain_wasmer_host`'s lib.rs   that gives a one-paragraph overview of what the crate does and   documents every cargo feature (the four wasmer backend features,   `error-as-host`, `debug-memory`) including which ones are on by   default and what each one is for. The intent is that when a new   user lands on the crate's docs.rs page, the feature matrix is   immediately visible without needing to read Cargo.toml.
  - Add a similar (smaller) crate-level doc to `holochain_wasmer_common`   documenting its single `error-as-host` feature.
  - Add `[package.metadata.docs.rs]` to the host crate so docs.rs   builds with `wasmer-sys`, `wasmer-sys-cranelift`, `wasmer-wasmi`   and `error-as-host` enabled. This is necessary for the intra-doc   links from the new lib.rs doc into `module::sys::*` and   `module::wasmi::*` to resolve. `wasmer-sys-llvm` is intentionally   omitted because `llvm-sys` requires a prebuilt LLVM toolchain that   the docs.rs builder doesn't provide; the LLVM compiler factory is   mentioned in prose without an intra-doc link.
  - Refresh `module.rs`'s top-level `//!` doc. The previous wording was   written before the additive-features change landed and reads like   the two backends are mutually exclusive choices; rewrite it to   match the actual current shape (both backends can coexist, picked   per-call-site via the engine factory passed to `ModuleBuilder` /   `ModuleCache`) and cross-link to the crate-level feature matrix   rather than duplicating it.
  - Fix a pre-existing rustdoc bug in `WasmError`'s doc comment in   common/src/result.rs: the example module path was   `holochain_wasmer_host::module::wasmer_sys`, but the actual Rust   module is named `sys`, not `wasmer_sys` — that path has never   existed. The example now reads `holochain_wasmer_host::module::sys`,   matching reality.
  - Drop the spurious `pub` on the `tests` module in   `holochain_wasmer_common`'s lib.rs. The module is gated on   `#[cfg(test)]` so the `pub` was harmless but stylistically odd; in a   1.0 codebase it shouldn't be there.
  - No code changes outside of doc comments and one Cargo.toml metadata addition. `cargo doc` for both crates is verified clean for any warning introduced by these changes (the few remaining doc warnings are pre-existing in `guest.rs`/`env.rs` and out of scope for this commit).
  - This branch is stacked on top of the kebab-case feature rename PR since the new feature-flag docs reference the kebab-case names.
- Explain wasm validation and cache trust model in ModuleBuilder by @synchwire in [#176](https://github.com/holochain/holochain-wasmer/pull/176)
  - Addresses the check issue #141 ('review wasm validation and ensure we are following best practices'). No behaviour change — the conclusion of the review is that we are already validating correctly via `Module::from_binary`, and the filesystem cache deserialize path is trusted by design per wasmer's `unsafe` contract. Capture both facts as doc comments on `ModuleBuilder::from_binary` and `ModuleBuilder::from_serialized_module` so the question doesn't need to be re-derived from the wasmer source.

### First-time Contributors

- @synchwire made their first contribution in [#187](https://github.com/holochain/holochain-wasmer/pull/187)

## \[[0.0.102](https://github.com/holochain/holochain-wasmer/compare/v0.0.101...v0.0.102)\] - 2026-04-02

### Miscellaneous Tasks

- Upgrade prepare release action to v1.6.0 by @jost-s
- Bump hsb to 0.0.57 by @jost-s
- Add iOS build check by @ThetaSinner in [#161](https://github.com/holochain/holochain-wasmer/pull/161)
- Test on Android by @ThetaSinner

### CI

- Bump more CI versions by @ThetaSinner in [#162](https://github.com/holochain/holochain-wasmer/pull/162)

## [0.0.101] - 2025-06-19

### Changed

- Workspace maintenance and update HSB (#157) by @ThetaSinner in [#157](https://github.com/holochain/holochain-wasmer/pull/157)
- Fix job name for ci_pass (#155) by @ThetaSinner in [#155](https://github.com/holochain/holochain-wasmer/pull/155)

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
