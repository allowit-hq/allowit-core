//! A settlement service for sellers.
//!
//! A seller that takes payment authorizations names a service that checks and submits them.
//! The rail of the payment's network implements the service: it reads the seller's request in
//! the seller's payment protocol, checks the authorization against the ledger, submits it
//! with the service's own key and reads the settlement from the ledger. The host serves the
//! routes, keeps the key, bounds what a caller can ask and records what was settled.
//!
//! The service holds no customer funds. It can submit one authorized payment, once, to the
//! payee that the payer signed, and it pays the network fee of that submission within
//! [`Limits::max_fee`]. It serves any payer of the protocol, and it trusts no caller: every
//! field of a request is the caller's claim until the rail has checked it.
//!
//! This module names no rail and no payment protocol dialect.
use super::{error::Result, runtime::Signer};
use serde_json::Value;
use std::time::Duration;

/// The largest request document that a rail reads.
pub const MAX_REQUEST_BYTES: usize = 65_536;

/// What the host allows one settlement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// The largest network fee, in base units of the fee asset, that the service pays for one
    /// settlement. An authorization that needs more is refused before anything is submitted.
    pub max_fee: u128,
    /// How long the service waits for the ledger to confirm a submitted settlement.
    pub wait: Duration,
}

/// One payment authorization as the rail read and checked it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Authorization {
    /// The identity of the authorization. The ledger settles one identity once.
    pub id: [u8; 32],
    /// The address that pays.
    pub payer: String,
    /// The address that receives the payment.
    pub pay_to: String,
    /// The asset reference in the rail's own format.
    pub asset: String,
    /// The amount in base units of the asset.
    pub units: i128,
    /// The last ledger height at which the authorization can settle.
    pub expires_height: u64,
}

/// The kinds of payment that the service settles on one network.
#[derive(Clone, Debug, PartialEq)]
pub struct Supported {
    /// One document per supported kind, in the seller's protocol.
    pub kinds: Vec<Value>,
    /// The key under which the protocol lists the service's signers for this network.
    pub signers_key: String,
}

/// The answer to a check. `reply` is the document for the seller, in the seller's protocol.
#[derive(Clone, Debug, PartialEq)]
pub struct Verified {
    pub reply: Value,
    /// The authorization, when it is valid now. `None` when the rail refused the request;
    /// `reply` then states the reason.
    pub authorization: Option<Authorization>,
    /// The network fee that a settlement now would pay, in base units of the fee asset.
    pub fee: u128,
}

/// What happened to a settlement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The ledger confirmed the transaction, and it moved the payment.
    Settled,
    /// Nothing was submitted, or the ledger shows that the transaction failed.
    Refused,
    /// The transaction was submitted and the ledger has not answered in time. It can still
    /// settle. `Settled::transaction` names it.
    Pending,
}

/// The answer to a settlement. `reply` is the document for the seller.
#[derive(Clone, Debug, PartialEq)]
pub struct Settled {
    pub reply: Value,
    pub outcome: Outcome,
    /// The authorization, when the rail read one from the request.
    pub authorization: Option<Authorization>,
    /// The hash of the submitted transaction, when one was submitted.
    pub transaction: Option<[u8; 32]>,
    /// The network fee that the service paid, in base units of the fee asset, when the ledger
    /// reports it.
    pub fee_paid: Option<u128>,
}

/// The settlement-service hook of a rail.
pub trait SettlementProvider: Sync {
    /// The kinds that the service settles on `network`.
    fn supported(&self, network: &str) -> Result<Supported>;
    /// Check the authorization of `request` against the ledger at `endpoint`, and submit
    /// nothing. `signer` is the identity of the service's key: the rail refuses a request that
    /// would move the service's own funds or name the service as a party.
    fn verify(
        &self,
        network: &str,
        endpoint: &str,
        signer: &str,
        request: &Value,
        limits: &Limits,
    ) -> Result<Verified>;
    /// Check the authorization of `request` again, submit it with `signer` as the source of
    /// the transaction and its fee, and read the settlement from the ledger within
    /// `limits.wait`. The rail never submits a request that [`Self::verify`] refuses.
    fn settle(
        &self,
        network: &str,
        endpoint: &str,
        signer: &dyn Signer,
        request: &Value,
        limits: &Limits,
    ) -> Result<Settled>;
    /// Read the ledger again for a settlement that was [`Outcome::Pending`]: the same
    /// `request` and the `transaction` that the earlier answer named.
    fn observe(
        &self,
        network: &str,
        endpoint: &str,
        request: &Value,
        transaction: &[u8; 32],
    ) -> Result<Settled>;
}
