# allowit-core

## Artifact storage

Keep code, required build and test inputs and concise API and usage documentation in this repository. Plans, design reports, reviews, run evidence, demo recordings and screenshots belong in the main project repository under `.agents/artifacts/`. Never commit them here or in a feature branch. Link to the canonical artifact instead.

## Dependencies

Core depends on no other Allowit repository. Pins point one way: the rail repositories (allowit-contracts-solana, allowit-contracts-tempo, allowit-contracts-stellar) depend on core, and allowit-sdk depends on the rail crates and on core. Never add a git dependency on those repositories or a `[patch]` section.

The package is `allowit-sdk` (library `allowit_sdk`). Consumers pin this repository by full `rev` and their `scripts/build.sh` tracks its head. A change to a public type changes every consumer; name the affected repositories in the pull request.

## Verification

`verification/` holds only the core Lean model. The Solana verification set, including the obligation ledger and every hash-bound lock and receipt, lives in allowit-contracts-solana `verification/`. Its receipts bind a byte-identical copy of `lean/AllowIt.lean`. Keep the model here in step with that copy, and record any model change in allowit-contracts-solana with a new receipt there.

## Before pushing

Read `.github/workflows/`. Run the affected commands locally, with their feature flags and selectors:

- `scripts/build.sh` (all targets and the Wasm module)
- `cargo fmt --all --check`
- `cargo test --locked`, and the feature-profile commands in `ci.yml` for a change behind a feature
- `cargo clippy --locked --all-targets -- -D warnings`
- `node tests/test-wasm.mjs` after `scripts/build.sh`
- `python3 -m unittest discover -s verification/tests` for a change in `verification/`

Resolve failures before pushing. Preserve existing reviews and deployment restrictions.
