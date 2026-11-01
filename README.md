# Allowit core

Allowit core defines the Allowit policy language. It compiles restricted Rust policy source to validated IR and evaluates that IR. It also holds the types that the SDK and the rail crates share: client primitives, service contracts, transports and the chain contract artifact.

The package is `allowit-sdk` (library `allowit_sdk`). The compiler and the evaluator do no I/O. Three feature-gated modules do: `lsp::run_stdio` (`compiler`) serves the language server on stdin and stdout, `client` (`client`) reads environment variables and local files, and `transport` (`transport`) makes blocking HTTP requests.

## Repositories

Dependencies point one way. The rail crates depend on core. The SDK depends on the rail crates and on core. Core depends on no Allowit repository.

| Repository | Depends on core for |
| --- | --- |
| [allowit-contracts-solana](https://github.com/allowit-hq/allowit-contracts-solana) | Solana programs and host crates |
| [allowit-contracts-tempo](https://github.com/allowit-hq/allowit-contracts-tempo) | Tempo host crates |
| [allowit-contracts-stellar](https://github.com/allowit-hq/allowit-contracts-stellar) | Stellar wallet |
| [allowit-sdk](https://github.com/allowit-hq/allowit-sdk) | The `allowit` CLI and the engine |

Releases are annotated git tags `vX.Y.Z`. A consumer names the tag and its minor version:

```toml
allowit-sdk = { git = "https://github.com/allowit-hq/allowit-core.git", tag = "v0.1.0", version = "0.1", default-features = false }
```

Every Allowit crate in one build names the same core tag, so the build holds one copy of core. Consumers commit `Cargo.lock`, which records the commit of the tag.

To release, set `version` in `Cargo.toml`, merge, then tag the merge commit: `git tag -a vX.Y.Z -m vX.Y.Z <commit> && git push origin vX.Y.Z`.

## Modules

| Module | Feature | Contents |
| --- | --- | --- |
| crate root | | Policy types, IR validation, `evaluate_ir`, `process_value`, `process_json` and `registry` |
| `prelude` | | The names that policy source imports with `use allowit::prelude::*;` |
| `lsp` | `compiler` | The stdio language server (`lsp::run_stdio`). `compile` and `evaluate` at the crate root need the same feature. |
| `typed_workflow` | | Typed workflow records for hosts and services |
| `admin` | | Owner operations that a bound vendor implementation performs |
| `engine` | | The source type-checking facade for the shared onchain engine guards |
| `v1` | `std` | The versioned source facade that policy crates compile against |
| `spending` | `oracle-ledger` | Purchase-tier ledger support |
| `client` | `client` | Client configuration, errors, journals, output and private local storage |
| `service` | `service` | Rail-agnostic service operation contracts that vendor crates implement |
| `transport` | `transport` | Bounded blocking HTTP and JSON-RPC transports for service adapters |
| `contract` | `contract` | The chain contract artifact, mandate, execution state and binary IR transport that the rail contracts share |

## Features

| Feature | Purpose |
| --- | --- |
| `std`, `compiler`, `oracle-ledger` | The defaults |
| `typed-workflow` | Typed workflow records. `compiler` enables it. |
| `compact-contract-validation` | Error codes instead of diagnostics for `no_std` contracts |
| `client` | The `client` module. Enables `std`. |
| `service` | The `service` module. Enables `std`. |
| `transport` | The `transport` module. Enables `service`. |
| `contract` | The `contract` module. It builds without `std`. |
| `integer-json` | Integer-only JSON for contract artifacts. Enables `contract`. |
| `contract-stellar` | The Stellar chain profile: `integer-json` and `compact-contract-validation` |

With no features, the crate is the bare `no_std + alloc` interpreter.

## Build and test

You need Rust 1.98, the `wasm32-unknown-unknown` target and Node.js.

```sh
cargo build --all-targets --locked
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

[`.github/workflows/ci.yml`](.github/workflows/ci.yml) lists every feature profile that CI checks. The policies and request contexts in [`docs/examples/`](docs/examples/) are test inputs.

`allowit-contract-artifact <policy.rs> <intent.txt> [--binary]` writes the contract artifact for a policy to stdout:

```sh
cargo run --locked --features compiler,contract --bin allowit-contract-artifact -- docs/examples/approval.rs intent.txt
```

## WebAssembly

```sh
cargo rustc --locked --release --lib --target wasm32-unknown-unknown --crate-type cdylib
```

`node tests/test-wasm.mjs` tests the module, which is `target/wasm32-unknown-unknown/release/allowit_sdk.wasm`. The module embeds the package version, so each release has its own module hash. It exports `alloc(len)`, `process(ptr, len)` and `dealloc(ptr, len)`. `process` takes UTF-8 JSON and packs the output pointer and length into one `u64`. Hosts set memory and time limits and discard an instance after any trap.

## Verification

[`verification/`](verification/README.md) holds a machine-checked Lean model of feature selection and a small policy decision language.

## History

This repository starts at allowit-sdk commit [67f5d69](https://github.com/allowit-hq/allowit-sdk/commit/67f5d696847992e18131f815ce905713ec3646e1), which created `core/`. The earlier history of the same code stays in allowit-sdk: `src/` and `tests/` at the repository root, and the crates `client-core/`, `services/core/`, `services/transport/` and `contracts/common/`, which became the `client`, `service`, `transport` and `contract` modules. In an allowit-sdk clone, run:

```sh
git log 67f5d69~1 -- src tests client-core services/core services/transport contracts/common
```

## License

Allowit-authored code is released under the [MIT License](LICENSE). Dependencies keep their own terms. The [release workflow](.github/workflows/release.yml) ships the Wasm module with `LICENSE` and `dependency-licenses.html`, which [cargo-about](https://github.com/EmbarkStudios/cargo-about) generates from [`.github/about.toml`](.github/about.toml).
