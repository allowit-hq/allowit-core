# allowit-core

## Artifact storage

Keep code, required build and test inputs and concise API and usage documentation in this repository. Plans, design reports, reviews, run evidence, demo recordings and screenshots belong in the main project repository under `.agents/artifacts/`. Never commit them here or in a feature branch. Link to the canonical artifact instead.

## Dependencies

Core depends on no other Allowit repository. Dependencies point one way: the rail repositories (allowit-contracts-solana, allowit-contracts-tempo, allowit-contracts-stellar) depend on core, and allowit-sdk depends on the rail crates and on core. Never add a dependency on those repositories or a `[patch]` section.

The package is `allowit-sdk` (library `allowit_sdk`). Consumers name a release tag `vX.Y.Z` and the minor version (`tag = "v0.1.0", version = "0.1"`). The README states the release steps. A breaking change to a public type needs a new minor version before 1.0; name the affected repositories in the pull request. Never move or delete a pushed tag.

## Verification

`verification/` holds only the core Lean model. The Solana verification set, including the obligation ledger and every hash-bound lock and receipt, lives in allowit-contracts-solana `verification/`. Its receipts bind a byte-identical copy of `lean/AllowIt.lean`. Keep the model here in step with that copy, and record any model change in allowit-contracts-solana with a new receipt there.

## Before pushing

Read `.github/workflows/`. Run the affected commands locally, with their feature flags and selectors:

- `cargo build --all-targets --locked` and the Wasm command in the README
- `cargo fmt --all --check`
- `cargo test --locked`, and the feature-profile commands in `ci.yml` for a change behind a feature
- `cargo clippy --locked --all-targets -- -D warnings`
- `node tests/test-wasm.mjs` after the Wasm build
- `python3 -m unittest discover -s verification/tests` for a change in `verification/`

Resolve failures before pushing. Preserve existing reviews and deployment restrictions.
