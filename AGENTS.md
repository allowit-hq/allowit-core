# allowit-core

## Artifact storage

Keep code, required build/test inputs and concise API/usage documentation here. Keep plans in the canonical Allowit knowledge project's `.agents/4-plans/`. Keep reviews, evidence and recordings in its `.agents/artifacts/`. Preserve original owner inputs in its `.agents/1-sources/`. Do not commit those artifacts in this implementation repository or its feature branches.

## Dependencies and packages

Core depends on no other Allowit repository. Rails depend on core. The shared SDK engine depends on core and rails. Never add a dependency on another Allowit repository or a cross-repository path dependency or `[patch]` section.

The virtual workspace has one package, `allowit-core` (library `allowit_core`), at `crates/allowit-core`. It contains generic host primitives and the portable policy module. Native request models, signing, custody and rail semantics belong to their rail repository. Core owns the portable source allowlist and its Python helper. Rail libraries bind source shape to native wallet tiers. Server owns trusted publication and admission. SDK engine owns shared execution workflows for CLI and server. Resolve the Python helper and portable library from the same released Core source. Preserve the copied helper and fixture terms. Keep the portable module's declared `UNLICENSED` terms separate from the host modules' MIT terms. Follow `crates/allowit-core/LICENSES.md`. Do not relicense either scope implicitly.

Every package releases one workspace version. Consumers name a release tag `vX.Y.Z` and its minor version. Breaking public API changes need a new minor version before 1.0. Name affected repositories in the pull request. Never move or delete a pushed tag. Preserve owner approval of each new public head, independent reviews, deployment restrictions and subscription authentication.

## Validation profiles

The portable policy evaluator defaults to `std` and uses chain-neutral Rust types. Rail SDKs, contract macros and native storage belong in rail repositories. Keep transport and filesystem capabilities out of the portable policy module. Preserve overflow checks and policy denial/write-discard semantics. The crate retains its minimal `no_std` host profile. The portable evaluator is available only with `std`.

Read `.github/workflows/` before implementation. Use the pinned Rust toolchain. Before pushing, run the affected checks locally with their exact package, target, feature and locked selectors:

- `cargo build --locked --workspace --all-targets`
- `cargo fmt --all --check`
- `cargo test --locked -p allowit-core --features client,service,transport,policy-source,vendor`
- `cargo clippy --locked -p allowit-core --all-targets --features client,service,transport,policy-source,vendor -- -D warnings`
- `cargo check --locked -p allowit-core --no-default-features --lib`
- `cargo test --locked -p allowit-core`
- `cargo test --locked -p allowit-core --no-default-features --features std`
- `cargo clippy --locked -p allowit-core --all-targets -- -D warnings`
- `cargo clippy --locked -p allowit-core --no-default-features --features std --all-targets -- -D warnings`
- `python3 -m unittest discover -s scripts/tests`

Run loopback transport tests in an execution context that permits local sockets. A restricted bind or compiler-cache failure is not a test pass. Preserve the source graph and feature exclusions when adapting CI. Resolve actual warnings and failures before publishing. Do not weaken remaining validation when retiring obsolete compiler/IR/Wasm or legacy DSL model targets. The old formal model did not connect to the Rust implementation. Its immutable prior sources and Solana receipts remain unchanged.
