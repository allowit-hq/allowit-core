use crate::client::error::{Error, Result};
type Retain<'a, T> = Box<dyn FnOnce(&T) -> Result<()> + 'a>;

pub trait RuntimeProvider {
    fn diagnostic_session(
        &self,
        _installation: &crate::vendor::runtime::Installation,
    ) -> Result<Option<crate::vendor::runtime::DiagnosticSession>> {
        Ok(None)
    }
    fn problem(&self, error: &Error) -> crate::vendor::runtime::Problem {
        crate::vendor::runtime::Problem {
            status: 409,
            code: "",
            message: error.to_string(),
            details: None,
        }
    }

    fn discover_snapshot(
        &self,
        configuration: &crate::vendor::runtime::owner::Configuration,
        accounting: Option<&crate::vendor::runtime::Accounting>,
    ) -> Result<crate::vendor::runtime::Snapshot>;
    fn current_height(&self) -> Result<u64>;
    fn observe_hash(&self, hash: &[u8; 32]) -> Result<Option<crate::vendor::runtime::Receipt>>;
    fn import(
        &self,
        a: &crate::vendor::config::ServerAuthority,
        b: &[u8],
    ) -> Result<crate::vendor::runtime::Installation>;
    fn request(
        &self,
        i: &crate::vendor::runtime::Installation,
        id: [u8; 32],
        a: crate::vendor::runtime::Action,
        accounting: Option<&crate::vendor::runtime::Accounting>,
    ) -> Result<crate::vendor::runtime::RequestPacket>;
    fn read_request(
        &self,
        i: &crate::vendor::runtime::Installation,
        b: &[u8],
    ) -> Result<crate::vendor::runtime::RequestPacket>;
    fn snapshot(
        &self,
        i: &crate::vendor::runtime::Installation,
        a: Option<&crate::vendor::runtime::Accounting>,
    ) -> Result<crate::vendor::runtime::Snapshot>;
    fn action(
        &self,
        i: &crate::vendor::runtime::Installation,
        r: &crate::vendor::runtime::RequestPacket,
    ) -> Result<crate::vendor::runtime::Action>;
    fn resources(
        &self,
        i: &crate::vendor::runtime::Installation,
    ) -> Result<crate::vendor::runtime::Resources>;
    fn redeemable(
        &self,
        i: &crate::vendor::runtime::Installation,
        v: &str,
        shares: i128,
    ) -> Result<i128>;
    fn finish<'a>(
        &self,
        i: &crate::vendor::runtime::Installation,
        p: &crate::vendor::runtime::Prepared,
        retain: Retain<'a, crate::vendor::runtime::Prepared>,
    ) -> Result<crate::vendor::runtime::Prepared>;
    fn prepare(
        &self,
        i: &crate::vendor::runtime::Installation,
        r: &crate::vendor::runtime::RequestPacket,
        t: &crate::vendor::runtime::Terms,
        a: Option<&crate::vendor::runtime::Accounting>,
        v: crate::vendor::runtime::Validity,
    ) -> Result<crate::vendor::runtime::Prepared>;
    fn authorize_preparation(
        &self,
        i: &crate::vendor::runtime::Installation,
        p: &crate::vendor::runtime::Prepared,
        signer: &dyn crate::vendor::runtime::Signer,
        evaluator: Option<crate::vendor::runtime::Evaluator<'_>>,
        now: u64,
    ) -> Result<crate::vendor::runtime::Prepared>;
    fn finalize<'a>(
        &self,
        i: &crate::vendor::runtime::Installation,
        p: &crate::vendor::runtime::Prepared,
        s: &dyn crate::vendor::runtime::Signer,
        e: Option<crate::vendor::runtime::Evaluator<'_>>,
        now: u64,
        retain: Retain<'a, crate::vendor::runtime::Prepared>,
    ) -> Result<crate::vendor::runtime::Prepared>;
    fn revalidate(
        &self,
        i: &crate::vendor::runtime::Installation,
        p: &crate::vendor::runtime::Prepared,
        now: u64,
    ) -> Result<crate::vendor::runtime::Prepared>;
    fn sign(
        &self,
        i: &crate::vendor::runtime::Installation,
        p: &crate::vendor::runtime::Prepared,
        s: &dyn crate::vendor::runtime::Signer,
        now: u64,
    ) -> Result<crate::vendor::runtime::Signature>;
    fn authorize(
        &self,
        i: &crate::vendor::runtime::Installation,
        p: &crate::vendor::runtime::Prepared,
        s: &[crate::vendor::runtime::Signature],
        now: u64,
    ) -> Result<crate::vendor::runtime::Signed>;
    fn validate_signed(&self, p: &crate::vendor::runtime::Signed, now: u64) -> Result<()>;
    /// Send `p` once. `Submitted` once the network accepted it; for a payment authorization that
    /// another party submits, such as a seller, the runtime sends nothing, and `Submitted` means
    /// that it handed the authorization over. `Prepared` when the network did not accept it
    /// yet: nothing is pending, and the caller can send the same bytes again.
    /// `SubmissionUnknown` when the outcome is uncertain: the caller looks the transaction up
    /// with `status` before any replacement. The send of a code transaction
    /// (`prepare_code_within`) never returns a final status: only its receipt (`observe`) is
    /// final.
    fn broadcast(
        &self,
        p: &crate::vendor::runtime::Signed,
        now: u64,
    ) -> Result<crate::vendor::runtime::Status>;
    /// What the network holds of `p` now. `SubmissionUnknown` when the lookup cannot rule out that
    /// the network accepted or included it, such as a lookup that finds no record of a transaction
    /// that may still apply. `Prepared` only on evidence that the network did not accept it and
    /// nothing is pending.
    fn status(&self, p: &crate::vendor::runtime::Signed) -> Result<crate::vendor::runtime::Status>;
    fn observe(
        &self,
        p: &crate::vendor::runtime::Signed,
    ) -> Result<Option<crate::vendor::runtime::Receipt>>;
    fn observe_authorizations(
        &self,
        p: &[crate::vendor::runtime::Prepared],
    ) -> Result<Vec<crate::vendor::runtime::ObservedAuthorization>>;
    /// Whether the prepared operation `p` lapsed: it did not execute, and no envelope that carries
    /// its authorization can execute it any more. The ledger passed the time bound that the
    /// operation signs, and the network's history of the operation's executions, in any envelope,
    /// read after that and from the operation's preparation on, holds none. The time bound alone
    /// proves no lapse: once that history no longer holds the preparation, the rail cannot tell,
    /// and the operation stays open until its owner releases it. A caller releases an open
    /// operation on a final settlement of one of its receipts (`settle_receipt`: `Executed` or
    /// `Failed`) or on this lapse. `false` means that the operation can still
    /// execute, that it executed, or that the rail cannot tell, and it stays open; a read that
    /// fails is an error. A rail without such a bound keeps this default.
    fn lapsed(&self, p: &crate::vendor::runtime::Prepared) -> Result<bool> {
        let _ = p;
        Ok(false)
    }
    /// Whether the owner transaction with hash `hash` lapsed: it never applied and can no longer
    /// apply. The server prepared it at height `since`, valid until `expires_at` (Unix seconds).
    /// The server drops a sent owner step only on this proof or on its receipt. `false` means
    /// not proven, and the step stays pending. A rail without such a proof keeps this default.
    /// [`Self::owner_step_lapsed`] takes the signed step itself, for a rail whose step identity
    /// is not a 32-byte hash.
    fn owner_lapsed(&self, hash: &[u8; 32], expires_at: u64, since: u64) -> Result<bool> {
        let _ = (hash, expires_at, since);
        Ok(false)
    }
    /// What the operation `p` adds to its wallet's figures when it executes, as the wallet
    /// counts it. A server keeps the figures that a wallet does not keep itself, and adds this
    /// once the operation's receipt confirms it. `None` for a rail without such a count.
    fn usage(
        &self,
        p: &crate::vendor::runtime::Prepared,
    ) -> Result<Option<crate::vendor::runtime::Usage>> {
        let _ = p;
        Ok(None)
    }
    /// What the receipt `r` of the operation `p`, carried by the signed envelope `signed`, changes
    /// ([`crate::vendor::runtime::Settlement`]). `p` is the operation as the agent signed it,
    /// whichever envelope `signed` is. `accounting` is the server's account of the wallet before
    /// the receipt, and `carry` its gas carry. The rail verifies `r` against `signed` as
    /// [`crate::vendor::runtime::Receipt::verify_operation`] does: the receipt of `signed`'s
    /// transaction, or, for a payment authorization that another party settles, the receipt of
    /// that party's transaction. It refuses a receipt that does not verify, and a `signed` that
    /// does not carry the operation of `p`. Only the rail reads what a receipt means; the default
    /// refuses.
    fn settle_receipt(
        &self,
        p: &crate::vendor::runtime::Prepared,
        signed: &crate::vendor::runtime::Signed,
        r: &crate::vendor::runtime::Receipt,
        accounting: Option<&crate::vendor::runtime::Accounting>,
        carry: u128,
    ) -> Result<crate::vendor::runtime::Settlement> {
        let _ = (p, signed, r, accounting, carry);
        Err(Error::unsupported("This rail does not settle a receipt"))
    }
    /// The expense bound of `action` on the installation, which its wallet counts against the
    /// spending limits, from the fee ceiling `ceiling`
    /// ([`crate::vendor::runtime::ExpenseBound`]). Its `gas_units`, `max_fee_units`,
    /// `observed_height` and `day` are those of `ceiling`; a ceiling that the rail cannot sign is
    /// refused. `conversion_units` is the
    /// worst-case conversion loss of a vault operation, which this rail values itself, and zero
    /// for a transfer. `spent_units` is the principal plus `gas_units`: a transfer's principal is
    /// its amount, and a vault operation's principal is its `conversion_units`. A deposit of 100
    /// whose shares redeem for at least 95, with a fee ceiling priced at 2, spends 7. An action
    /// whose loss the rail cannot bound is refused. The default refuses.
    fn expense(
        &self,
        i: &crate::vendor::runtime::Installation,
        action: &crate::vendor::runtime::Action,
        ceiling: &crate::vendor::runtime::FeeCeiling,
    ) -> Result<crate::vendor::runtime::ExpenseBound> {
        let _ = (i, action, ceiling);
        Err(Error::unsupported(
            "This rail does not bound the expense of an operation",
        ))
    }
    /// The configuration version that a host takes for the wallet of `i` from the admission read
    /// `observed`, compared with its own account `retained`: the version of an owner's
    /// configuration change that this rail verified on the network. The host asks while no
    /// operation of the wallet is open, whatever the two versions are. `None` when `observed` shows
    /// the configuration and the replay state (`Accounting::nonce`) that `retained` describes.
    /// Every other difference is refused: a change that the host cannot take, such as one that also
    /// moved the replay state or a version that went back, and a replay state that moved while the
    /// version stayed. The default refuses.
    fn configuration_change(
        &self,
        i: &crate::vendor::runtime::Installation,
        retained: &crate::vendor::runtime::Accounting,
        observed: &crate::vendor::runtime::Snapshot,
    ) -> Result<Option<u64>> {
        let _ = (i, retained, observed);
        Err(Error::unsupported(
            "This rail does not verify a configuration change",
        ))
    }
    fn prepare_owner(
        &self,
        i: &crate::vendor::runtime::Installation,
        a: crate::vendor::runtime::OwnerAction,
        fee: u64,
        now: u64,
    ) -> Result<crate::vendor::runtime::OwnerPacket>;
    /// Prepare the upload of the wallet code `wasm` for the installation, from one of the server's
    /// sending accounts and signed by it, with the network fee bounded by `max_fee` in the rail's
    /// native units. A fee above `max_fee` is refused before anything is signed. `None` when the
    /// network has the code live: nothing is needed. The server pays the upload, and it gives no
    /// authority: the owner's creation names the code by its hash. Nothing in Allowit restores
    /// code that lapsed.
    ///
    /// The rail sends nothing. The caller submits the transaction as any signed operation, with
    /// `broadcast`, and follows it with `status` and `observe`. Its identity,
    /// `Signed::info().operation_id`, is its transaction hash: the same after `RailPlugin::restore`
    /// of its retained bytes, and another for every later transaction. `validate_signed` refuses
    /// it after its time bound, `Signed::info().expires_at`. Its receipt (`observe`) is the
    /// receipt of that transaction: `Receipt::info` reports `Confirmed` or `Failed` and the fee
    /// that the sending account paid, also for a transaction that the ledger included and
    /// failed. A rail without code transactions refuses.
    fn prepare_code_within(
        &self,
        i: &crate::vendor::runtime::Installation,
        wasm: &[u8],
        max_fee: u64,
        now: u64,
    ) -> Result<Option<crate::vendor::runtime::Signed>> {
        let _ = (i, wasm, max_fee, now);
        Err(Error::unsupported(
            "This rail does not prepare wallet code transactions",
        ))
    }
    /// Prepare the renewal of the live wallet code `code` on this provider's network, from one
    /// of the server's sending accounts and signed by it, with the network fee bounded by
    /// `max_fee` as for `prepare_code_within`: a transaction that extends the code's lifetime.
    /// `None` while the code's lifetime is above the rail's renewal threshold. The server renews
    /// the code that it uploaded and that wallets share, such as the code of a fixed tier; it pays
    /// the renewal, charges it to no request, and the renewal gives no authority. Code that lapsed
    /// is not renewed: nothing in Allowit restores it. The packet takes the path of a code
    /// upload: the caller sends it with `broadcast` and reads its receipt with `observe`. A rail
    /// without code renewals refuses.
    fn prepare_code_renewal(
        &self,
        code: &[u8; 32],
        max_fee: u64,
        now: u64,
    ) -> Result<Option<crate::vendor::runtime::Signed>> {
        let _ = (code, max_fee, now);
        Err(Error::unsupported("This rail does not renew wallet code"))
    }
    /// [`Self::prepare_code_within`] of the wallet code `code`, in this rail's code format, within
    /// the time bound that the caller fixed. `validity` is an outer bound: the transaction applies
    /// no earlier than `validity.now`, and its own bound (`Signed::info().expires_at`) is at most
    /// `validity.expires_at` and can be earlier. The caller follows that own bound for every later
    /// send and lookup of the transaction. A bound that the rail cannot sign within is refused
    /// before anything is signed. The default refuses.
    fn prepare_code_until(
        &self,
        i: &crate::vendor::runtime::Installation,
        code: &[u8],
        max_fee: u64,
        validity: crate::vendor::runtime::Validity,
    ) -> Result<Option<crate::vendor::runtime::Signed>> {
        let _ = (i, code, max_fee, validity);
        Err(Error::unsupported(
            "This rail does not prepare wallet code transactions within a time bound",
        ))
    }
    /// [`Self::prepare_code_renewal`] with the time bound that the caller fixed, as for
    /// [`Self::prepare_code_until`]. The default refuses.
    fn prepare_code_renewal_until(
        &self,
        code: &[u8; 32],
        max_fee: u64,
        validity: crate::vendor::runtime::Validity,
    ) -> Result<Option<crate::vendor::runtime::Signed>> {
        let _ = (code, max_fee, validity);
        Err(Error::unsupported(
            "This rail does not renew wallet code within a time bound",
        ))
    }
    /// Whether the code transaction `p` from `prepare_code_within` or `prepare_code_renewal`, or
    /// from their time-bound forms `prepare_code_until` and `prepare_code_renewal_until`, is
    /// still needed: an upload until its code is live, a renewal while the code's lifetime is
    /// below the renewal threshold. A caller sends again a code transaction that the network did
    /// not accept only while it is needed, since another transaction can do its work while it
    /// waits. Only the rail can observe its code, so a rail that prepares code transactions
    /// answers this; the default refuses.
    fn code_needed(&self, p: &crate::vendor::runtime::Signed) -> Result<bool> {
        let _ = p;
        Err(Error::unsupported(
            "This rail does not say whether a code transaction is still needed",
        ))
    }
    /// Prepare the owner step of a params update on an active wallet. The packet takes the
    /// path of every owner packet: `read_owner_signed`, then the check and the send of the
    /// signed step with the installation `i` ([`Self::validate_owner_signed`],
    /// [`Self::broadcast_owner`]), and `observe_owner`.
    /// A rail refuses an update whose expected revision is not the installed one.
    fn prepare_params_update(
        &self,
        i: &crate::vendor::runtime::Installation,
        update: &crate::vendor::runtime::ParamsUpdate,
        fee: u64,
        now: u64,
    ) -> Result<crate::vendor::runtime::OwnerPacket> {
        let _ = (i, update, fee, now);
        Err(Error::unsupported(
            "This rail does not update the params of an installed wallet",
        ))
    }
    fn read_owner_signed(
        &self,
        p: &crate::vendor::runtime::OwnerPacket,
        signed_payload: &[u8],
    ) -> Result<crate::vendor::runtime::OwnerSigned>;
    /// The send-and-wait loop of a rail, of the contract before the caller's own owner steps. It
    /// takes no installation and reads no wallet: a caller sends an owner step with
    /// [`Self::validate_owner_signed`] and [`Self::broadcast_owner`] and does not call this
    /// method. It leaves the trait with the other superseded methods.
    fn submit_owner<'a>(
        &self,
        p: &crate::vendor::runtime::OwnerSigned,
        retain: Retain<'a, crate::vendor::runtime::OwnerSigned>,
        poll: crate::vendor::runtime::Poll,
        now: u64,
    ) -> Result<crate::vendor::runtime::Receipt>;
    fn observe_owner(
        &self,
        p: &crate::vendor::runtime::OwnerSigned,
    ) -> Result<Option<crate::vendor::runtime::Receipt>>;
    /// Whether the caller may send the owner-signed step `p` at `now`, by the rail's own rules.
    /// `on` is the installation that the step acts on, or `None` for a step that leaves before
    /// its wallet is installed, as a creation; the caller's record of the step binds it to its
    /// installation.
    ///
    /// The rail checks the step's send cutoff
    /// ([`crate::vendor::runtime::OwnerSignedInfo::expires_at`]) first. With the installation of
    /// a wallet of a kind that can lapse, the rail then reads the wallet's lifetime, and one
    /// coherent observation of its network decides: a network can apply a step that does not
    /// call the wallet, as a funding, although the wallet lapsed, and a step that calls a lapsed
    /// wallet fails on the network at its owner's cost. A wallet of a kind that cannot lapse is
    /// not read, also on a network where another kind can. The rail decides by the installation:
    /// it reads no action name. One step is never read for liveness: the step that renews or
    /// restores the wallet's own lifetime, which the rail knows by the signed transaction itself.
    /// It moves nothing, and it is how a wallet is kept or brought back.
    ///
    /// A refusal is a typed error (`Error::coded`).
    /// [`crate::vendor::runtime::OWNER_TRANSACTION_EXPIRED`] is final: the caller never sends the
    /// step again. A step that was possibly sent before then ends by its receipt or its lapse,
    /// and a step that the caller knows it never sent is dropped.
    /// [`crate::vendor::runtime::CONTRACT_EXPIRED`] refuses this attempt: the wallet was lapsed
    /// when the rail read it, and a later check before the cutoff passes once the wallet is live
    /// again. The check is of the read and promises nothing about the moment at which the
    /// network applies the step: a wallet that lapses after the read is not refused. A read that
    /// fails is an uncertain error (`Error::uncertain`), never a lapse and never a pass: the
    /// caller asks again. The default refuses.
    fn validate_owner_signed(
        &self,
        p: &crate::vendor::runtime::OwnerSigned,
        on: Option<&crate::vendor::runtime::Installation>,
        now: u64,
    ) -> Result<()> {
        let _ = (p, on, now);
        Err(Error::unsupported(
            "This rail does not check an owner step before it is sent",
        ))
    }
    /// Send the owner-signed step `p` once, after the checks of [`Self::validate_owner_signed`]
    /// for the same `on` at `now`: a step that they refuse is that refusal, and nothing is sent.
    /// A check that got no answer is its uncertain error, and nothing is sent either. Each says
    /// nothing about an earlier attempt of the same step, which stays as it was.
    /// `Submitted` once the network accepted it. `Prepared` when the network did not accept
    /// this send: nothing of this send is pending, and the caller can send the same bytes
    /// again. `Prepared` from a send speaks for that send only: an earlier attempt of the same
    /// step whose outcome was uncertain stays possibly sent. Only `Prepared` from
    /// [`Self::owner_status`] says that nothing of the step is pending. `SubmissionUnknown` when
    /// the
    /// outcome is uncertain: the caller looks the step up ([`Self::observe_owner`], then
    /// [`Self::owner_status`]) before any other send. A lookup that answers decides: a receipt
    /// ends the step, `Submitted` waits for its receipt, `Prepared` sends again. When the lookup
    /// stays inconclusive (no receipt, and `SubmissionUnknown` again), the caller may send the
    /// same signed bytes again while [`Self::validate_owner_signed`] passes. A rail that
    /// implements this method guarantees that this is safe: its network applies one signed step
    /// at most once, however often the identical bytes arrive. A lookup that fails is no answer:
    /// the step stays pending, and the caller sends nothing until a lookup answers. A caller
    /// never signs or sends other bytes for the step while these can still apply. An uncertain
    /// outcome of the send itself is always the status `SubmissionUnknown`, never an error. So
    /// every error means that nothing of this send is pending: the checks refused it, a check
    /// got no answer, or the network rejected it. The caller reports it, and it decides nothing
    /// about an earlier attempt of the same step whose outcome was uncertain, which stays
    /// pending. Never a final status: only the step's
    /// receipt (`observe_owner`) or its lapse ([`Self::owner_step_lapsed`]) ends a possibly sent
    /// step. The default refuses.
    fn broadcast_owner(
        &self,
        p: &crate::vendor::runtime::OwnerSigned,
        on: Option<&crate::vendor::runtime::Installation>,
        now: u64,
    ) -> Result<crate::vendor::runtime::Status> {
        let _ = (p, on, now);
        Err(Error::unsupported(
            "This rail does not send an owner step once",
        ))
    }
    /// What the network holds of the owner-signed step `p` now, never a final status:
    /// `Submitted` when it accepted or included the step (the receipt of an included step is
    /// `observe_owner`'s); `SubmissionUnknown` when the lookup cannot rule out that it accepted
    /// or included the step; `Prepared` only on evidence that it did not accept the step and
    /// nothing is pending. The default refuses.
    fn owner_status(
        &self,
        p: &crate::vendor::runtime::OwnerSigned,
    ) -> Result<crate::vendor::runtime::Status> {
        let _ = p;
        Err(Error::unsupported(
            "This rail does not look up an owner step",
        ))
    }
    /// Whether the sent owner step `p` lapsed: it never applied and can no longer apply. The
    /// server prepared it at height `since`. The rail looks the step up by its own identity
    /// ([`crate::vendor::runtime::OwnerSigned::info`]). `false` means not proven, and the step
    /// stays pending; a read that fails is an error. A caller drops a possibly sent step only on
    /// its receipt or on this proof. The default refuses.
    fn owner_step_lapsed(
        &self,
        p: &crate::vendor::runtime::OwnerSigned,
        since: u64,
    ) -> Result<bool> {
        let _ = (p, since);
        Err(Error::unsupported(
            "This rail does not prove that an owner step lapsed",
        ))
    }
    /// Whether the wallet of `i` is closed: a wallet of a kind that closes
    /// ([`Self::prepare_close`]) is gone from its network and takes no operation or owner step.
    /// `false` for a wallet that is there and for a wallet kind that never closes. A read that
    /// fails is an error. The default refuses.
    fn wallet_closed(&self, i: &crate::vendor::runtime::Installation) -> Result<bool> {
        let _ = i;
        Err(Error::unsupported(
            "This rail does not say whether a wallet is closed",
        ))
    }
    /// Prepare the owner step that closes the wallet of `i`, for a wallet kind that closes: the
    /// wallet returns what it holds to its owner and leaves its network. The network fee is
    /// bounded by `fee` in the rail's native units, and the step's send cutoff
    /// ([`crate::vendor::runtime::OwnerInfo::expires_at`]) is counted from `now`. The packet
    /// states no earliest time: a network can apply it as soon as it is sent.
    /// The packet takes the path of every owner packet: `read_owner_signed`, `broadcast_owner`
    /// and `observe_owner`. A wallet kind that does not close refuses, and so does the default.
    fn prepare_close(
        &self,
        i: &crate::vendor::runtime::Installation,
        fee: u64,
        now: u64,
    ) -> Result<crate::vendor::runtime::OwnerPacket> {
        let _ = (i, fee, now);
        Err(Error::unsupported("This rail does not close a wallet"))
    }
}

pub trait OwnerProvider {
    fn deploy(
        &self,
        cx: &super::owner::Context<'_>,
        r: super::owner::Deploy<'_>,
    ) -> super::error::Result<serde_json::Value>;
    fn fund(
        &self,
        cx: &super::owner::Context<'_>,
        document: &[u8],
        id: &str,
        amount: &str,
        fee: &str,
    ) -> super::error::Result<serde_json::Value>;
    fn activate(
        &self,
        cx: &super::owner::Context<'_>,
        r: super::owner::Activate<'_>,
    ) -> super::error::Result<serde_json::Value>;
    fn export(
        &self,
        cx: &super::owner::Context<'_>,
        document: &[u8],
        id: &str,
    ) -> super::error::Result<Box<serde_json::value::RawValue>>;
    fn setup(
        &self,
        cx: &super::owner::Context<'_>,
        r: super::owner::Setup<'_>,
    ) -> super::error::Result<serde_json::Value>;
    fn status(
        &self,
        store: &crate::client::store::Store,
        network: &str,
        rpc: &str,
        id: &str,
    ) -> super::error::Result<serde_json::Value>;
    /// Change typed params of an unsigned draft through the backend's owner route. The reply
    /// is the next revision of the draft. A rail without the call declares
    /// `Feature::EditParams` skipped.
    fn edit_params(
        &self,
        cx: &super::owner::Context<'_>,
        r: super::owner::EditParams<'_>,
    ) -> super::error::Result<serde_json::Value> {
        let _ = (cx, r);
        Err(Error::unsupported(
            "This rail does not edit the params of a draft",
        ))
    }
    /// Revise an unsigned draft in the owner's words. The reply is the next revision.
    fn revise(
        &self,
        cx: &super::owner::Context<'_>,
        r: super::owner::Revise<'_>,
    ) -> super::error::Result<serde_json::Value> {
        let _ = (cx, r);
        Err(Error::unsupported("This rail does not revise a draft"))
    }
    /// Change installed params on an active wallet with one owner signature. The workflow is
    /// resume-safe under its request id. A rail without it declares `Feature::UpdateParams`
    /// skipped.
    fn update_params(
        &self,
        cx: &super::owner::Context<'_>,
        r: super::owner::UpdateParams<'_>,
    ) -> super::error::Result<serde_json::Value> {
        let _ = (cx, r);
        Err(Error::unsupported(
            "This rail does not update the params of an installed wallet",
        ))
    }
    /// Compare the owner's document with the ledger and read the wallet state back. A rail
    /// without the read declares `Feature::Verify` skipped.
    fn verify(
        &self,
        cx: &super::owner::Context<'_>,
        document: &[u8],
    ) -> super::error::Result<super::owner::Verification> {
        let _ = (cx, document);
        Err(Error::unsupported(
            "This rail does not verify an installation on the ledger",
        ))
    }
    /// The host and native placements of a draft or installed policy document.
    fn paired_export(
        &self,
        network: &str,
        document: &[u8],
    ) -> super::error::Result<super::owner::PairedExport> {
        let _ = (network, document);
        Err(Error::unsupported(
            "This rail does not export the placements of a policy",
        ))
    }
    /// The question that the owner confirms before this rail signs `step` on `network`: what
    /// the step does on the ledger, in the rail's own words, such as a token transfer or the
    /// approval of an allowance. A host shows it in place of its own question. `None`, the
    /// default, keeps the host's question. It reads nothing and signs nothing.
    fn question(&self, network: &str, step: super::owner::Question<'_>) -> Option<String> {
        let _ = (network, step);
        None
    }
    /// The asset of a new Native wallet on the context's network, read as [`Self::setup`] reads
    /// it and refused where the setup would refuse it, so a host can show it before the setup
    /// asks for the owner's signature. The owner signs in to read it and signs nothing else. A
    /// rail without Native wallets refuses, and so does the default.
    fn native_wallet_asset(
        &self,
        cx: &super::owner::Context<'_>,
    ) -> super::error::Result<super::owner::NativeWalletAsset> {
        let _ = cx;
        Err(Error::unsupported(
            "This rail does not read the asset of a new Native wallet",
        ))
    }
}

#[cfg(test)]
mod defaults {
    use super::*;
    /// A rail that implements only what it must, so every default answers.
    struct Bare;
    #[allow(unused_variables)]
    impl RuntimeProvider for Bare {
        fn discover_snapshot(
            &self,
            configuration: &crate::vendor::runtime::owner::Configuration,
            accounting: Option<&crate::vendor::runtime::Accounting>,
        ) -> Result<crate::vendor::runtime::Snapshot> {
            unimplemented!()
        }
        fn current_height(&self) -> Result<u64> {
            unimplemented!()
        }
        fn observe_hash(&self, hash: &[u8; 32]) -> Result<Option<crate::vendor::runtime::Receipt>> {
            unimplemented!()
        }
        fn import(
            &self,
            a: &crate::vendor::config::ServerAuthority,
            b: &[u8],
        ) -> Result<crate::vendor::runtime::Installation> {
            unimplemented!()
        }
        fn request(
            &self,
            i: &crate::vendor::runtime::Installation,
            id: [u8; 32],
            a: crate::vendor::runtime::Action,
            accounting: Option<&crate::vendor::runtime::Accounting>,
        ) -> Result<crate::vendor::runtime::RequestPacket> {
            unimplemented!()
        }
        fn read_request(
            &self,
            i: &crate::vendor::runtime::Installation,
            b: &[u8],
        ) -> Result<crate::vendor::runtime::RequestPacket> {
            unimplemented!()
        }
        fn snapshot(
            &self,
            i: &crate::vendor::runtime::Installation,
            a: Option<&crate::vendor::runtime::Accounting>,
        ) -> Result<crate::vendor::runtime::Snapshot> {
            unimplemented!()
        }
        fn action(
            &self,
            i: &crate::vendor::runtime::Installation,
            r: &crate::vendor::runtime::RequestPacket,
        ) -> Result<crate::vendor::runtime::Action> {
            unimplemented!()
        }
        fn resources(
            &self,
            i: &crate::vendor::runtime::Installation,
        ) -> Result<crate::vendor::runtime::Resources> {
            unimplemented!()
        }
        fn redeemable(
            &self,
            i: &crate::vendor::runtime::Installation,
            v: &str,
            shares: i128,
        ) -> Result<i128> {
            unimplemented!()
        }
        fn finish<'a>(
            &self,
            i: &crate::vendor::runtime::Installation,
            p: &crate::vendor::runtime::Prepared,
            retain: super::Retain<'a, crate::vendor::runtime::Prepared>,
        ) -> Result<crate::vendor::runtime::Prepared> {
            unimplemented!()
        }
        fn prepare(
            &self,
            i: &crate::vendor::runtime::Installation,
            r: &crate::vendor::runtime::RequestPacket,
            t: &crate::vendor::runtime::Terms,
            a: Option<&crate::vendor::runtime::Accounting>,
            v: crate::vendor::runtime::Validity,
        ) -> Result<crate::vendor::runtime::Prepared> {
            unimplemented!()
        }
        fn authorize_preparation(
            &self,
            i: &crate::vendor::runtime::Installation,
            p: &crate::vendor::runtime::Prepared,
            signer: &dyn crate::vendor::runtime::Signer,
            evaluator: Option<crate::vendor::runtime::Evaluator<'_>>,
            now: u64,
        ) -> Result<crate::vendor::runtime::Prepared> {
            unimplemented!()
        }
        fn finalize<'a>(
            &self,
            i: &crate::vendor::runtime::Installation,
            p: &crate::vendor::runtime::Prepared,
            s: &dyn crate::vendor::runtime::Signer,
            e: Option<crate::vendor::runtime::Evaluator<'_>>,
            now: u64,
            retain: super::Retain<'a, crate::vendor::runtime::Prepared>,
        ) -> Result<crate::vendor::runtime::Prepared> {
            unimplemented!()
        }
        fn revalidate(
            &self,
            i: &crate::vendor::runtime::Installation,
            p: &crate::vendor::runtime::Prepared,
            now: u64,
        ) -> Result<crate::vendor::runtime::Prepared> {
            unimplemented!()
        }
        fn sign(
            &self,
            i: &crate::vendor::runtime::Installation,
            p: &crate::vendor::runtime::Prepared,
            s: &dyn crate::vendor::runtime::Signer,
            now: u64,
        ) -> Result<crate::vendor::runtime::Signature> {
            unimplemented!()
        }
        fn authorize(
            &self,
            i: &crate::vendor::runtime::Installation,
            p: &crate::vendor::runtime::Prepared,
            s: &[crate::vendor::runtime::Signature],
            now: u64,
        ) -> Result<crate::vendor::runtime::Signed> {
            unimplemented!()
        }
        fn validate_signed(&self, p: &crate::vendor::runtime::Signed, now: u64) -> Result<()> {
            unimplemented!()
        }
        fn broadcast(
            &self,
            p: &crate::vendor::runtime::Signed,
            now: u64,
        ) -> Result<crate::vendor::runtime::Status> {
            unimplemented!()
        }
        fn status(
            &self,
            p: &crate::vendor::runtime::Signed,
        ) -> Result<crate::vendor::runtime::Status> {
            unimplemented!()
        }
        fn observe(
            &self,
            p: &crate::vendor::runtime::Signed,
        ) -> Result<Option<crate::vendor::runtime::Receipt>> {
            unimplemented!()
        }
        fn observe_authorizations(
            &self,
            p: &[crate::vendor::runtime::Prepared],
        ) -> Result<Vec<crate::vendor::runtime::ObservedAuthorization>> {
            unimplemented!()
        }
        fn prepare_owner(
            &self,
            i: &crate::vendor::runtime::Installation,
            a: crate::vendor::runtime::OwnerAction,
            fee: u64,
            now: u64,
        ) -> Result<crate::vendor::runtime::OwnerPacket> {
            unimplemented!()
        }
        fn read_owner_signed(
            &self,
            p: &crate::vendor::runtime::OwnerPacket,
            signed_payload: &[u8],
        ) -> Result<crate::vendor::runtime::OwnerSigned> {
            unimplemented!()
        }
        fn submit_owner<'a>(
            &self,
            p: &crate::vendor::runtime::OwnerSigned,
            retain: super::Retain<'a, crate::vendor::runtime::OwnerSigned>,
            poll: crate::vendor::runtime::Poll,
            now: u64,
        ) -> Result<crate::vendor::runtime::Receipt> {
            unimplemented!()
        }
        fn observe_owner(
            &self,
            p: &crate::vendor::runtime::OwnerSigned,
        ) -> Result<Option<crate::vendor::runtime::Receipt>> {
            unimplemented!()
        }
    }
    struct Packets;
    static PACKETS: Packets = Packets;
    #[allow(unused_variables)]
    impl crate::vendor::runtime::PacketProvider for Packets {
        fn request_id(&self, p: &crate::vendor::runtime::Packet) -> Result<[u8; 32]> {
            unimplemented!()
        }
        fn owner_info(
            &self,
            p: &crate::vendor::runtime::Packet,
        ) -> Result<crate::vendor::runtime::OwnerInfo> {
            unimplemented!()
        }
        fn prepared_metadata(
            &self,
            p: &crate::vendor::runtime::Packet,
        ) -> Result<crate::vendor::runtime::PreparedMetadata> {
            unimplemented!()
        }
        fn info(
            &self,
            p: &crate::vendor::runtime::Packet,
            k: crate::vendor::runtime::Kind,
        ) -> Result<crate::vendor::runtime::PacketInfo> {
            unimplemented!()
        }
        fn validate_signed(&self, p: &crate::vendor::runtime::Packet) -> Result<()> {
            unimplemented!()
        }
        fn signed_preparation(
            &self,
            p: &crate::vendor::runtime::Packet,
        ) -> Result<crate::vendor::runtime::Prepared> {
            unimplemented!()
        }
        fn verify_consent(
            &self,
            s: &crate::vendor::runtime::Snapshot,
            message: &str,
            signatures: &[crate::vendor::runtime::Signature],
        ) -> Result<()> {
            unimplemented!()
        }
        fn signer_weight(
            &self,
            s: &crate::vendor::runtime::Snapshot,
            identity: &str,
        ) -> Result<u32> {
            unimplemented!()
        }
        fn authority_document(
            &self,
            s: &crate::vendor::runtime::Snapshot,
        ) -> Result<serde_json::Value> {
            unimplemented!()
        }
        fn same_snapshot(
            &self,
            a: &crate::vendor::runtime::Snapshot,
            b: &crate::vendor::runtime::Snapshot,
        ) -> Result<bool> {
            unimplemented!()
        }
        fn verify_receipt(
            &self,
            r: &crate::vendor::runtime::Packet,
            p: &crate::vendor::runtime::Packet,
        ) -> Result<()> {
            unimplemented!()
        }
        fn receipt_info(
            &self,
            p: &crate::vendor::runtime::Packet,
        ) -> Result<crate::vendor::runtime::ReceiptInfo> {
            unimplemented!()
        }
    }
    pub(super) fn installation() -> crate::vendor::runtime::Installation {
        crate::vendor::runtime::Installation {
            description: crate::vendor::profiles::Description {
                network: "test:network".into(),
                wallet: String::new(),
                agent: String::new(),
                policy_id: String::new(),
                owner: String::new(),
                server: String::new(),
                asset: String::new(),
                asset_symbol: String::new(),
                source: String::new(),
                source_hash: String::new(),
                module_digest: String::new(),
                params: Default::default(),
            },
            config: serde_json::value::RawValue::from_string("{}".into()).unwrap(),
            export: None,
            authority: None,
        }
    }
    pub(super) fn packet() -> crate::vendor::runtime::Packet {
        crate::vendor::runtime::Packet::new("test:network", &serde_json::json!({}), &PACKETS)
            .unwrap()
    }
    /// The code of a refusal of `result`: 3 is a rail that does not implement the call.
    pub(super) fn refused<T>(result: Result<T>) -> i32 {
        result.map(drop).unwrap_err().code
    }
    /// A rail that does not verify a configuration change or sign a code transaction within a
    /// time bound is refused, never given an answer.
    #[test]
    fn a_rail_refuses_the_time_bounds_and_configuration_changes_that_it_does_not_implement() {
        use crate::vendor::runtime::{Accounting, Snapshot, Validity};
        let snapshot = Snapshot {
            accounting: Accounting::default(),
            observed_height: 1,
            paused: false,
            params: Default::default(),
            state: Default::default(),
            evidence: packet(),
        };
        assert_eq!(
            refused(Bare.configuration_change(&installation(), &Accounting::default(), &snapshot)),
            3
        );
        let validity = Validity {
            now: 1,
            expires_at: 2,
        };
        assert_eq!(
            refused(Bare.prepare_code_until(&installation(), &[], 1, validity)),
            3
        );
        assert_eq!(
            refused(Bare.prepare_code_renewal_until(&[1; 32], 1, validity)),
            3
        );
    }
    /// A rail that does not read the asset of a new Native wallet is refused, never given an
    /// asset.
    #[test]
    fn a_rail_refuses_the_native_wallet_asset_that_it_does_not_read() {
        use crate::vendor::owner::{Activate, Context, Deploy, Setup};
        struct Owner;
        impl OwnerProvider for Owner {
            fn deploy(&self, _: &Context<'_>, _: Deploy<'_>) -> Result<serde_json::Value> {
                unreachable!()
            }
            fn fund(
                &self,
                _: &Context<'_>,
                _: &[u8],
                _: &str,
                _: &str,
                _: &str,
            ) -> Result<serde_json::Value> {
                unreachable!()
            }
            fn activate(&self, _: &Context<'_>, _: Activate<'_>) -> Result<serde_json::Value> {
                unreachable!()
            }
            fn export(
                &self,
                _: &Context<'_>,
                _: &[u8],
                _: &str,
            ) -> Result<Box<serde_json::value::RawValue>> {
                unreachable!()
            }
            fn setup(&self, _: &Context<'_>, _: Setup<'_>) -> Result<serde_json::Value> {
                unreachable!()
            }
            fn status(
                &self,
                _: &crate::client::store::Store,
                _: &str,
                _: &str,
                _: &str,
            ) -> Result<serde_json::Value> {
                unreachable!()
            }
        }
        struct Key;
        impl crate::vendor::wallet::Signer for Key {
            fn network(&self) -> &str {
                "test:network"
            }
            fn address(&self) -> &str {
                "owner"
            }
            fn sign_hash(&self, _: &[u8]) -> Result<Vec<u8>> {
                unreachable!("the default signs nothing")
            }
        }
        let authority = crate::vendor::config::ServerAuthority::validated(
            "test:network",
            "https://server.example",
            "server",
        )
        .unwrap();
        let cx = Context {
            network: "test:network",
            backend: "https://server.example",
            authority: &authority,
            origin: "https://app.example",
            rpc_url: "https://rpc.example",
            store: &|| unreachable!("the default opens no store"),
            signer: &Key,
            clock: &|| Ok(1),
        };
        assert_eq!(refused(Owner.native_wallet_asset(&cx)), 3);
        // A rail that states no question of its own keeps the host's.
        use crate::vendor::owner::Question;
        for step in [
            Question::Setup { amount: None },
            Question::NativeDeploy { amount: Some("1") },
            Question::Fund { amount: "1" },
        ] {
            assert_eq!(Owner.question("test:network", step), None);
        }
    }
    /// A rail that does not read a receipt or bound an operation's expense is refused, never
    /// given an answer.
    #[test]
    fn a_rail_refuses_the_receipts_and_expenses_that_it_does_not_implement() {
        use crate::vendor::runtime::{Action, FeeCeiling, Prepared, Receipt, Signed};
        assert_eq!(
            refused(Bare.settle_receipt(
                &Prepared(packet()),
                &Signed(packet()),
                &Receipt(packet()),
                None,
                7
            )),
            3
        );
        let transfer = Action::Transfer {
            recipient: String::new(),
            units: 1,
        };
        let ceiling = FeeCeiling {
            max_fee_units: 1,
            gas_units: 1,
            observed_height: 1,
            day: 1,
        };
        assert_eq!(
            refused(Bare.expense(&installation(), &transfer, &ceiling)),
            3
        );
    }
    /// A rail that does not send, look up, prove the lapse of or close an owner step is refused,
    /// never given an answer.
    #[test]
    fn a_rail_refuses_the_owner_steps_that_it_does_not_implement() {
        use crate::vendor::runtime::OwnerSigned;
        let step = OwnerSigned(packet());
        assert_eq!(refused(Bare.validate_owner_signed(&step, None, 1)), 3);
        assert_eq!(refused(Bare.broadcast_owner(&step, None, 1)), 3);
        assert_eq!(refused(Bare.owner_status(&step)), 3);
        assert_eq!(refused(Bare.owner_step_lapsed(&step, 1)), 3);
        assert_eq!(refused(Bare.wallet_closed(&installation())), 3);
        assert_eq!(refused(Bare.prepare_close(&installation(), 100, 1)), 3);
        assert_eq!(refused(step.info()), 3);
    }
    /// A rail that does not say whether a code transaction is still needed refuses: only the
    /// rail can observe its code, and a caller that took a default answer could send again a
    /// transaction that another one made needless. A rail without code transactions prepares
    /// none.
    #[test]
    fn a_rail_without_code_transactions_answers_nothing_about_them() {
        let packet = crate::vendor::runtime::Signed(
            crate::vendor::runtime::Packet::new("test:network", &serde_json::json!({}), &PACKETS)
                .unwrap(),
        );
        assert!(Bare.code_needed(&packet).is_err());
        assert!(Bare.prepare_code_renewal(&[1; 32], 100, 0).is_err());
    }
}
