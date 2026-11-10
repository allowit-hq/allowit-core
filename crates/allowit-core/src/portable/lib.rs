#![no_std]
//! Canonical Allowit policy rules for host evaluation. Rails own native bindings and must
//! validate authored policies against their execution profile:
//!
//! ```ignore
//! use allowit_core as allowit;
//! use allowit::prelude::*;
//! /// Let my agent pay our invoices. Limit its total spending and pay only vendors I approve.
//! pub fn execute(ctx: &AllowitContext) -> allowit::Result {
//!     // "Limit its total spending"
//!     allowit::lifetime_spending_limit(ctx, &ctx.params[allowit::LIFETIME_SPENDING_LIMIT])?;
//!     // "pay only vendors I approve"
//!     allowit::destinations(ctx, &ctx.params[allowit::DESTINATIONS])?;
//!     Ok(())
//! }
//! ```
//!
//! `AllowitContext` is plain data: `env`, `params`, `state` and `request`. `params` holds the
//! owner's constructor params, key => `AllowitValue`; an absent key is unset. Policy reads a param
//! by key, `ctx.params[allowit::DESTINATIONS]` or `ctx.params["destinations"]`, and indexing an
//! unset key panics with `MissingParam`. `ctx.params.contains(key)` tells whether a key is set.
//! `state` is the current policy state, before this operation: one free-form map with standard
//! keys (`spent`, `day_spent`, `day_total`, `calls`, `day`, `unpriced`, `day_unpriced`) and
//! policy-defined keys. `request.args` holds the agent's execution params for this call. Params,
//! state and args share the one value type `AllowitValue`.
//!
//! A rule takes the param value and decodes it: a value of the wrong type or range is
//! `InvalidParam`. Rules add the request's own contribution to the current values and return their
//! passing constraint or an `AllowitError`. Only an error that the policy propagates with `?` ends
//! the run. A policy may catch a rule error, read it and continue. `operations` admits the
//! request's op only when the owner's `operations` param names it, and returns the op name, such
//! as `"transfer"`, for a `match`.
//!
//! `ctx.params[key].ok()` reads an owner param as the type of its binding or of the rule argument
//! it feeds: `let floor: i128 = ctx.params["swap_output_floor"].ok();` and
//! `allowit::payment_limit(ctx, &ctx.params["swap_limit"].ok())?`. `param` and `arg` read an owner
//! param or an agent arg by key: `let limit: i128 = allowit::param(ctx, "swap_limit");`. These
//! reads panic when the key is unset (`MissingParam`) or the value has another type or range
//! (`InvalidParam`). No zero, default or conversion stands in for the value. A panic, also from an
//! unset `ctx.params[key]`, `panic!`, `assert!`, `unreachable!` or an integer overflow, is a hard
//! stop: the run denies and its writes are discarded. `evaluate` catches the panic and returns
//! the failed-read error or `Panicked`. Rails must enforce their native abort and rollback behavior.
//!
//! After approval, `post_flight` rolls the UTC day over, counts the call, adds spending and
//! gross amounts, then applies policy state writes. It rejects invalid standard state and
//! results beyond `MAX_STATE_KEYS` or `MAX_STATE_BYTES`. The standard tier rejects policy writes.
//! `evaluate` checks request arguments and standard-tier parameters before executing a policy.
//! Denied runs discard their writes. `preflight` runs one canonical rule and skips absent params.
//!
//! Portable authored code uses rule calls, `ctx.params[key]`, `ok`, `contains`, `param`, `arg`,
//! `get_state` and `set_state`, typed scalar fields and enum comparisons.
//! Source admission selects only portable names, paths and methods.
//!
//! Types and rules use chain-neutral Rust values. Rails supply authenticated request facts and
//! implement native contract bindings outside this package. The `std` feature is enabled by default.
//! Host evaluation buffers state writes and discards them when a policy denies or panics.
//! Build hosts must enable `overflow-checks = true` so arithmetic overflow also denies.
//!
//! ```
//! use allowit_core as allowit;
//! use allowit::prelude::*;
//! /// Limit lifetime spending with a portable rule.
//! pub fn execute(ctx: &AllowitContext) -> allowit::Result {
//!     allowit::lifetime_spending_limit(ctx, &ctx.params[allowit::LIFETIME_SPENDING_LIMIT])?;
//!     Ok(())
//! }
//! ```
//!
//! Host enrichment functions return `Unsupported` until a server implements them.
//!
//! A policy never re-enters the wallet. `reentry_transfer` exists only on rails that implement
//! it; this package has no such function, so a policy that calls it does not compile:
//!
//! ```compile_fail,E0425
//! use allowit_core as allowit;
//! use allowit::prelude::*;
//! /// A policy that re-enters the wallet does not build.
//! pub fn execute(ctx: &AllowitContext) -> allowit::Result {
//!     allowit::reentry_transfer(ctx, 1)?;
//!     Ok(())
//! }
//! ```
//!
//! A `for` loop over a borrowed list binds owned clones, including lists in `ctx.params[key]`.
//! The source-admission compile checks the same loops.
//!
//! ```
//! use allowit_core as allowit;
//! use allowit::prelude::*;
//! /// Pay only listed vendors or suppliers.
//! pub fn execute(ctx: &AllowitContext) -> allowit::Result {
//!     let wanted = AllowitValue::Addr(ctx.request.recipient.clone());
//!     if let AllowitValue::List(items) = &ctx.params["vendors"] {
//!         for item in items {
//!             if item == wanted {
//!                 return Ok(());
//!             }
//!         }
//!     }
//!     let suppliers: Vec<Address> = allowit::param(ctx, "suppliers");
//!     for supplier in &suppliers {
//!         if supplier == ctx.request.recipient {
//!             return Ok(());
//!         }
//!     }
//!     Err(AllowitError::DestinationNotAllowed)
//! }
//! ```
//!
//! ```compile_fail,E0614
//! use allowit_core as allowit;
//! use allowit::prelude::*;
//! /// A loop item is owned, so a dereference does not compile.
//! pub fn execute(ctx: &AllowitContext) -> allowit::Result {
//!     let wanted = AllowitValue::Addr(ctx.request.recipient.clone());
//!     if let AllowitValue::List(items) = &ctx.params["vendors"] {
//!         for item in items {
//!             if *item == wanted {
//!                 return Ok(());
//!             }
//!         }
//!     }
//!     Err(AllowitError::DestinationNotAllowed)
//! }
//! ```
//!
//! Portable text `String` has the traits of a native contract string: comparison, ordering, `clone`
//! and `{:?}`. It does not dereference to `str`. The host compile therefore rejects the text uses
//! that a native build rejects, such as a key held in a `String`, a comparison with a literal and
//! `{}`.
//!
//! ```
//! use allowit_core as allowit;
//! use allowit::prelude::*;
//! /// Deny when the agent's memo repeats the last approved memo.
//! pub fn execute(ctx: &AllowitContext) -> allowit::Result {
//!     let memo: String = allowit::arg(ctx, "memo");
//!     let last: Option<String> = allowit::get_state(ctx, "last_memo")?;
//!     if last == Some(memo.clone()) {
//!         panic!("repeated memo {:?}", memo);
//!     }
//!     allowit::set_state(ctx, "last_memo", memo)
//! }
//! ```
//!
//! ```compile_fail,E0308
//! use allowit_core as allowit;
//! use allowit::prelude::*;
//! /// A key held in a `String` does not compile.
//! pub fn execute(ctx: &AllowitContext) -> allowit::Result {
//!     let key: String = allowit::param(ctx, "limit_key");
//!     let limit: i128 = allowit::param(ctx, &key);
//!     if ctx.request.amount > limit {
//!         return Err(AllowitError::PaymentLimitExceeded);
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ```compile_fail,E0614
//! use allowit_core as allowit;
//! use allowit::prelude::*;
//! /// A `String` does not dereference to `str`.
//! pub fn execute(ctx: &AllowitContext) -> allowit::Result {
//!     let key: String = allowit::param(ctx, "limit_key");
//!     let limit: i128 = allowit::param(ctx, &*key);
//!     if ctx.request.amount > limit {
//!         return Err(AllowitError::PaymentLimitExceeded);
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ```compile_fail,E0277
//! use allowit_core as allowit;
//! use allowit::prelude::*;
//! /// `{}` does not format a `String`.
//! pub fn execute(ctx: &AllowitContext) -> allowit::Result {
//!     let key: String = allowit::param(ctx, "limit_key");
//!     if ctx.request.amount > 100 {
//!         panic!("over limit for {}", key);
//!     }
//!     Ok(())
//! }
//! ```
#[cfg(feature = "std")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

#[cfg(not(feature = "std"))]
compile_error!("The portable policy evaluator requires feature `std`");

/// Host backend for the offchain server and SDK evaluator. Addresses are text that the rail
/// supplies.
#[cfg(feature = "std")]
pub mod host {
    use alloc::collections::BTreeMap;
    use core::cell::Cell;
    use std::sync::{Arc, Mutex, MutexGuard};

    /// Portable text with the traits of a native contract string: comparison, ordering, `clone`
    /// and `{:?}`. It has no `str` dereference, `Display`, `Default`, `AsRef` or `Add`, so
    /// authored code cannot use it as a key, compare it with a literal, format it with `{}` or
    /// extend it. Host integration builds one with `From` and reads it with `as_str`.
    #[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
    pub struct String(alloc::string::String);

    impl String {
        /// The text, for the server.
        pub fn as_str(&self) -> &str {
            &self.0
        }
    }

    impl From<&str> for String {
        /// The portable text of `text`.
        fn from(text: &str) -> Self {
            Self(text.into())
        }
    }

    impl From<alloc::string::String> for String {
        /// The portable text of `text`.
        fn from(text: alloc::string::String) -> Self {
            Self(text)
        }
    }

    /// Exact account or contract identity, held as the text that its rail supplies. It compares
    /// only for equality. Rails own native address validation and ordering. Portable authored
    /// code cannot order addresses.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct Address(alloc::string::String);

    impl Address {
        /// The address with this text. The rail supplies text that it has validated.
        pub fn new(text: impl Into<alloc::string::String>) -> Self {
            Self(text.into())
        }

        /// The text of this address.
        pub fn as_str(&self) -> &str {
            &self.0
        }
    }

    /// Portable list. Iterating a borrowed list yields owned clones, including a list borrowed
    /// from `ctx.params[key]`. Authored code cannot dereference the list to a slice.
    /// Host integration can read a slice with `as_slice`.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct Vec<T>(alloc::vec::Vec<T>);

    impl<T> Vec<T> {
        /// The items, for the server.
        pub fn as_slice(&self) -> &[T] {
            &self.0
        }
    }

    impl<T> From<alloc::vec::Vec<T>> for Vec<T> {
        /// The list of these items.
        fn from(items: alloc::vec::Vec<T>) -> Self {
            Self(items)
        }
    }

    impl<T> FromIterator<T> for Vec<T> {
        /// The list of the iterated items.
        fn from_iter<I: IntoIterator<Item = T>>(items: I) -> Self {
            Self(items.into_iter().collect())
        }
    }

    impl<T> IntoIterator for Vec<T> {
        /// An owned item.
        type Item = T;
        /// The items in order.
        type IntoIter = alloc::vec::IntoIter<T>;
        /// The items in order.
        fn into_iter(self) -> Self::IntoIter {
            self.0.into_iter()
        }
    }

    impl<'a, T: Clone> IntoIterator for &'a Vec<T> {
        /// A clone of an item.
        type Item = T;
        /// Clones of the items in order.
        type IntoIter = core::iter::Cloned<core::slice::Iter<'a, T>>;
        /// Clones of the items in order.
        fn into_iter(self) -> Self::IntoIter {
            self.0.iter().cloned()
        }
    }

    /// Portable map with bounded text keys.
    type Map<K, V> = BTreeMap<K, V>;
    /// Host map key.
    type Symbol = alloc::string::String;

    /// Host environment of policy evaluations. It buffers the policy's `set_state` writes for
    /// `post_flight`; clones share the buffer. `evaluate` clears it before a run and on denial.
    #[derive(Clone, Debug)]
    pub struct AllowitEnv {
        writes: Arc<Mutex<AllowitMap>>,
    }

    impl AllowitEnv {
        /// The environment of a new evaluation, with no buffered writes.
        #[allow(clippy::new_without_default)]
        pub fn new() -> Self {
            Self {
                writes: Arc::new(Mutex::new(BTreeMap::new())),
            }
        }

        /// The write buffer. A panic while it was held leaves no partial write, so it stays usable.
        fn buffer(&self) -> MutexGuard<'_, AllowitMap> {
            self.writes
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
        }
    }

    std::thread_local! {
        /// The error of the last failed read on this thread: an unset `ctx.params[key]` or a
        /// failed `param` or `arg` read. A policy runs on the thread of its `evaluate` call, and
        /// indexing has no `AllowitEnv`, so the record is per thread.
        static FAILURE: Cell<Option<AllowitError>> = const { Cell::new(None) };
    }

    /// Keep a `core.rs` value type a plain Rust type.
    macro_rules! value {
        ($item:item) => {
            $item
        };
    }
    /// Keep the `core.rs` error enum a plain Rust enum.
    macro_rules! error {
        ($item:item) => {
            $item
        };
    }
    /// Keep the map of `AllowitArgs` private to this crate, so the std-only compile of authored
    /// policy rejects any read other than `arg` and `contains`. The server converts with `From`.
    macro_rules! args {
        ($(#[$meta:meta])* $name:ident($($map:tt)*)) => {
            $(#[$meta])*
            pub struct $name(
                /// The key => value map.
                pub(crate) $($map)*,
            );
        };
    }
    include!("core.rs");

    /// The owner's params, read by key: `ctx.params[allowit::DESTINATIONS]`. The map stays
    /// private to this crate, so the std-only compile of authored policy rejects any read other
    /// than indexing, `contains` and `param`. The server converts with `From`. It derives only
    /// `Clone`, so authored policies cannot compare entire parameter maps.
    #[derive(Clone)]
    pub struct AllowitParams(pub(crate) AllowitMap);

    impl AllowitParams {
        /// The key => value map.
        pub(crate) fn map(&self) -> &AllowitMap {
            &self.0
        }
    }

    /// The server converts params and args to and from key => value maps.
    macro_rules! map_conversions {
        ($($name:ident),*) => {$(
            /// The server builds this from a key => value map.
            impl From<AllowitMap> for $name {
                /// The value holding `map`.
                fn from(map: AllowitMap) -> Self {
                    Self(map)
                }
            }
            /// The server reads back the key => value map.
            impl From<$name> for AllowitMap {
                /// The map of `value`.
                fn from(value: $name) -> Self {
                    value.0
                }
            }
        )*};
    }
    map_conversions!(AllowitParams, AllowitArgs);

    /// The value of `key` in `params`, or `None` when it is unset or no map can hold the key.
    fn find<'a>(params: &'a AllowitParams, key: &str) -> Option<&'a AllowitValue> {
        key_ok(key).then(|| params.0.get(key)).flatten()
    }

    /// Record `MissingParam` for `evaluate` and panic with a message that names unset param `key`.
    #[track_caller]
    fn missing(_params: &AllowitParams, key: &str) -> ! {
        stop(AllowitError::MissingParam, key)
    }

    /// The value of `key` in `map`, or `None` when it is absent or no map can hold the key.
    fn lookup(map: &AllowitMap, key: &str) -> Option<AllowitValue> {
        key_ok(key).then(|| map.get(key).cloned()).flatten()
    }

    /// Set `key` to `value` in `map`. An invalid key is `InvalidParam`.
    fn insert(map: &mut AllowitMap, key: &str, value: AllowitValue) -> Result {
        if !key_ok(key) {
            return Err(AllowitError::InvalidParam);
        }
        map.insert(Symbol::from(key), value);
        Ok(())
    }

    /// The number of entries in `map`.
    fn count(map: &AllowitMap) -> u32 {
        u32::try_from(map.len()).unwrap_or(u32::MAX)
    }

    /// The byte length of a text value.
    fn text_len(text: &String) -> u32 {
        u32::try_from(text.0.len()).unwrap_or(u32::MAX)
    }

    /// The bytes of a text value, or `None` beyond `MAX_TEXT` bytes.
    fn text_bytes<'a>(text: &'a String, _buf: &'a mut [u8; MAX_TEXT as usize]) -> Option<&'a [u8]> {
        (text.0.len() <= MAX_TEXT as usize).then_some(text.0.as_bytes())
    }

    /// The text of an address, as its rail supplied it.
    fn address_text(address: &Address) -> String {
        String::from(address.as_str())
    }

    /// Record `error` for `evaluate` and panic with a message that names it and `key`.
    #[track_caller]
    fn fail(_env: &AllowitEnv, error: AllowitError, key: &str) -> ! {
        stop(error, key)
    }

    /// Record `InvalidParam` for `evaluate` and panic: a typed read met a value of another type
    /// or range. A value carries no key, so the message names none.
    #[track_caller]
    fn mistyped() -> ! {
        stop(AllowitError::InvalidParam, "value")
    }

    /// Record `error` for this thread's `guarded` run and panic with a message that names it and
    /// `key`.
    #[track_caller]
    fn stop(error: AllowitError, key: &str) -> ! {
        FAILURE.set(Some(error));
        panic!("allowit: {error:?} for `{key}`")
    }

    /// Run the policy and turn a panic into a denial: the recorded error of a failed read, or
    /// `Panicked`. The process keeps running.
    fn guarded(_env: &AllowitEnv, run: impl FnOnce() -> Result) -> Result {
        FAILURE.take();
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(run))
            .unwrap_or_else(|_| Err(FAILURE.take().unwrap_or(AllowitError::Panicked)))
    }

    /// Read a `List` of `Addr` values as addresses; any other value is `InvalidParam`.
    impl TryFrom<AllowitValue> for Vec<Address> {
        /// A value of another type.
        type Error = AllowitError;
        /// The addresses of a `List` of `Addr`.
        fn try_from(value: AllowitValue) -> Result<Self> {
            match value {
                AllowitValue::List(items) => items.into_iter().map(Address::try_from).collect(),
                _ => Err(AllowitError::InvalidParam),
            }
        }
    }

    /// Run `check` on each value of `map`. An invalid portable key is `InvalidParam`.
    fn each_value(map: &AllowitMap, mut check: impl FnMut(AllowitValue) -> Result) -> Result {
        for (key, value) in map {
            if !key_ok(key) {
                return Err(AllowitError::InvalidParam);
            }
            check(value.clone())?;
        }
        Ok(())
    }

    /// Run `check` on each item of a list.
    fn each_item(items: &Vec<AllowitValue>, check: impl FnMut(AllowitValue) -> Result) -> Result {
        items.into_iter().try_for_each(check)
    }

    /// Buffer one policy write for `post_flight`.
    fn record(env: &AllowitEnv, key: &str, value: AllowitValue) -> Result {
        insert(&mut env.buffer(), key, value)
    }

    /// Remove and return the buffered policy writes.
    fn take_writes(env: &AllowitEnv) -> AllowitMap {
        core::mem::take(&mut *env.buffer())
    }

    /// Apply `writes` over `state`.
    fn merge(state: &mut AllowitMap, writes: AllowitMap) {
        state.extend(writes);
    }

    /// Server-only enrichment. Each function takes its owner-written request, returns its passing
    /// constraint or an `AllowitError`, and is an API stub that the offchain server fills; here it
    /// is `Unsupported`. Rails determine which APIs their native execution profiles admit.
    pub mod server {
        use super::{Address, AllowitContext, AllowitError, AllowitValue, Result};

        /// Price check: `amount` of `from` priced in `to` must stay within `slip` of `price`.
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct AllowitPrice {
            /// Asset priced.
            pub from: Address,
            /// Quote asset.
            pub to: Address,
            /// Units of `from` priced.
            pub amount: i128,
            /// Expected price: units of `to` per unit of `from`, scaled by 10^7.
            pub price: i128,
            /// Largest accepted deviation from `price`, in basis points.
            pub slip: u32,
        }
        /// Fetch request: the response must verify under `pubkey`.
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct AllowitFetch<'a> {
            /// HTTP method, such as `GET`.
            pub method: &'a str,
            /// Requested URL.
            pub url: &'a str,
            /// Request body; empty for none.
            pub data: &'a str,
            /// Key that signs the expected response.
            pub pubkey: &'a str,
        }
        /// Internet search request.
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct AllowitInternetSearch<'a> {
            /// Search query.
            pub query: &'a str,
        }
        /// Marketplace search request.
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct AllowitMarketplaceSearch<'a> {
            /// Search query.
            pub query: &'a str,
        }
        /// Request screening: the request must score at least `deny_below`.
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct AllowitRequestScreening<'a> {
            /// What the screening checks the request for.
            pub query: &'a str,
            /// Lowest passing score, in basis points.
            pub deny_below: u32,
        }
        /// SystemOne question within the evaluation's SystemOne context `id`.
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct AllowitSystemOne<'a> {
            /// SystemOne context identifier.
            pub id: &'a str,
            /// Question asked.
            pub query: &'a str,
            /// A confidence below this denies, in basis points.
            pub deny_below: u32,
            /// A confidence at or above this passes without review, in basis points.
            pub allow_above: u32,
        }

        /// Check a price; returns the observed price.
        pub fn price(_ctx: &AllowitContext, _request: &AllowitPrice) -> Result<i128> {
            Err(AllowitError::Unsupported)
        }
        /// Fetch a verified response; returns it as a value.
        pub fn fetch(_ctx: &AllowitContext, _request: &AllowitFetch) -> Result<AllowitValue> {
            Err(AllowitError::Unsupported)
        }
        /// Search the internet; returns the results as a value.
        pub fn internet_search(
            _ctx: &AllowitContext,
            _request: &AllowitInternetSearch,
        ) -> Result<AllowitValue> {
            Err(AllowitError::Unsupported)
        }
        /// Search marketplaces; returns the results as a value.
        pub fn marketplace_search(
            _ctx: &AllowitContext,
            _request: &AllowitMarketplaceSearch,
        ) -> Result<AllowitValue> {
            Err(AllowitError::Unsupported)
        }
        /// Screen the request; returns its passing score.
        pub fn request_screening(
            _ctx: &AllowitContext,
            _request: &AllowitRequestScreening,
        ) -> Result<u32> {
            Err(AllowitError::Unsupported)
        }
        /// Ask SystemOne; returns its passing confidence.
        pub fn systemone(_ctx: &AllowitContext, _request: &AllowitSystemOne) -> Result<u32> {
            Err(AllowitError::Unsupported)
        }
        /// Clear the evaluation's SystemOne context.
        pub fn clear_systemone_context(_ctx: &AllowitContext) -> Result {
            Err(AllowitError::Unsupported)
        }
    }
    pub use server::*;
}

#[cfg(feature = "std")]
pub use host::*;

/// Exact crate-root source for a host build against this release.
#[cfg(feature = "std")]
pub const SOURCE_LIB_RS: &str = include_str!("lib.rs");
/// Exact shared rule source for a host build against this release.
#[cfg(feature = "std")]
pub const SOURCE_CORE_RS: &str = include_str!("core.rs");
/// The portable package's declared terms. Embedding its source grants no additional license.
#[cfg(feature = "std")]
pub const SOURCE_TERMS: &str = "UNLICENSED";

#[cfg(all(test, feature = "std"))]
mod host_tests;

#[cfg(all(test, feature = "std"))]
mod tests;
