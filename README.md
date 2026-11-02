# Allowit core

Allowit core contains portable policy rules and reusable host primitives.
The shared SDK engine composes those primitives for both CLI and server hosts.
Rail libraries own native requests, codecs, signing and finality.
The server adds its exclusive policy authority and private state.

## Packages

| Package | Library | Responsibility | License |
| --- | --- | --- | --- |
| `allowit` | `allowit` | Deterministic policy types, nine rules, evaluation and accounting | `UNLICENSED` |
| `allowit-sdk` | `allowit_sdk` | Generic owner capabilities, private storage, configuration, source checks and bounded transport | MIT |

Core depends on no other Allowit repository. Rails, vendor adapters and SDK engine can use its
admitted package profiles directly. Importing a library grants no signing or payment authority.

`allowit` selects exactly one backend:

- Default `soroban` binds policy values to Soroban types for contracts.
- `std`, with default features disabled, binds the same rule logic to host values.

Both profiles compile the same rule definitions. Denials discard policy writes.
The contract profile excludes host network and filesystem capabilities.
Custody, native transaction encoding and native receipt checks remain in the rail library.

`allowit-sdk` defaults to `std`. Its `admin` capabilities and exact-byte `digest` also build without
`std`. Host-only features expose these modules:

| Feature | Modules |
| --- | --- |
| `client` | Private atomic storage, configuration, errors, journals and output |
| `service` | Protocol-neutral RPC operation contracts |
| `transport` | Bounded blocking HTTP/JSON-RPC. Enables `service` |
| `policy-source` | Portable authored-source allowlist and shape checks. Selects the portable `std` backend |

RPC endpoints require HTTPS or loopback HTTP, without URL credentials or fragments.
Transport performs one attempt. The calling workflow chooses recovery using the original signed
operation. A failed response does not establish transaction failure.

The host-only `policy_source::source_shape` derives the canonical prefix and detects state writes.
It rejects source constructs outside the portable allowlist. It grants no publication or admission
authority. Stellar binds that shape to its native wallet tier. The server enforces trusted admission.

`scripts/policy_source.py` provides the corresponding Python gate and direct-rustc host compile.
Resolve it and `crates/allowit` from the same released Core Cargo checkout.
Do not substitute a sibling checkout or a helper selected through an environment variable.
The source gate and host compile are separate required checks before trusted publication.

## Dependencies and releases

Every package in this repository releases the same annotated tag `vX.Y.Z`.
Consumers name that tag and its minor version, and commit `Cargo.lock`:

```toml
allowit = { git = "https://github.com/allowit-hq/allowit-core.git", tag = "v0.2.0", version = "0.2", default-features = false, features = ["std"] }
allowit-sdk = { git = "https://github.com/allowit-hq/allowit-core.git", tag = "v0.2.0", version = "0.2", default-features = false, features = ["client", "transport"] }
```

All Allowit packages in one build name the same core tag.
Contracts select the portable package's default Soroban profile separately from host builds.
Do not combine its `std` and `soroban` features.

Set the workspace version, merge the reviewed release, then annotate the merge commit with that tag.
Never move an existing tag. Registry publication is a separate operation.

## Build and test

Use Rust 1.98 with `wasm32v1-none`, rustfmt and Clippy, plus Stellar CLI 28.1.0.

```sh
cargo build --locked --workspace --all-targets
cargo fmt --all --check
cargo test --locked -p allowit-sdk --features client,service,transport,policy-source
cargo clippy --locked -p allowit-sdk --all-targets --features client,service,transport,policy-source -- -D warnings
cargo test --locked -p allowit
cargo test --locked -p allowit --no-default-features --features std
stellar contract build --locked --package allowit
python3 -m unittest discover -s scripts/tests
```

[CI](.github/workflows/ci.yml) also checks both portable profiles with Clippy and the minimal host package.
The policy library links into its caller. The Stellar CLI checks its no_std contract profile through
the official Soroban build pipeline. It does not export an IR-interpreter Wasm ABI.

## Source and terms

The portable package originates in [the Stellar policy library][portable-source].
The retired compiler and bounded legacy model remain in [core's original source][legacy-source].

The host package, including the Rust source gate from Stellar's MIT host crate, uses the
[MIT License](LICENSE). The portable package declares `UNLICENSED` in
[its manifest](crates/allowit/Cargo.toml). The copied Python helper and corpus generator retain their
original undeclared terms under [scripts/LICENSE](scripts/LICENSE).
The original corpus and portable fixture retain their terms under [tests/fixtures/LICENSE](tests/fixtures/LICENSE).
Dependencies retain their own terms.

[portable-source]: https://github.com/allowit-hq/allowit-contracts-stellar/tree/d0ae9be9e3d7a51e9ac4a1008ba207937aef36a2/crates/allowit
[legacy-source]: https://github.com/allowit-hq/allowit-core/tree/64ec4e989b878b13995c139832a5901e92b020fd
