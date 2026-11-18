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
    fn broadcast(
        &self,
        p: &crate::vendor::runtime::Signed,
        now: u64,
    ) -> Result<crate::vendor::runtime::Status>;
    fn status(&self, p: &crate::vendor::runtime::Signed) -> Result<crate::vendor::runtime::Status>;
    fn observe(
        &self,
        p: &crate::vendor::runtime::Signed,
    ) -> Result<Option<crate::vendor::runtime::Receipt>>;
    fn observe_authorizations(
        &self,
        p: &[crate::vendor::runtime::Prepared],
    ) -> Result<Vec<crate::vendor::runtime::ObservedAuthorization>>;
    /// Whether the prepared operation `p` lapsed: it did not execute, and no envelope that
    /// carries its authorization can execute it any more. The server releases an open operation
    /// only on this proof or on a terminal receipt. `false` means not proven, and the operation
    /// stays open. A rail without such a proof keeps this default.
    fn lapsed(&self, p: &crate::vendor::runtime::Prepared) -> Result<bool> {
        let _ = p;
        Ok(false)
    }
    /// Whether the owner transaction with hash `hash` lapsed: it never applied and can no longer
    /// apply. The server prepared it at height `since`, valid until `expires_at` (Unix seconds).
    /// The server drops a sent owner step only on this proof or on its receipt. `false` means
    /// not proven, and the step stays pending. A rail without such a proof keeps this default.
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
    fn prepare_owner(
        &self,
        i: &crate::vendor::runtime::Installation,
        a: crate::vendor::runtime::OwnerAction,
        fee: u64,
        now: u64,
    ) -> Result<crate::vendor::runtime::OwnerPacket>;
    /// Upload the wallet code `wasm` of an installation from the server's own account, which
    /// pays the network's fee and rent. An upload gives no authority: the owner's creation
    /// names the code by its hash. Returns the receipt of the upload, or `None` when the network
    /// holds the code already. A rail without uploads refuses.
    fn upload_code(
        &self,
        i: &crate::vendor::runtime::Installation,
        wasm: &[u8],
        poll: crate::vendor::runtime::Poll,
        now: u64,
    ) -> Result<Option<crate::vendor::runtime::Receipt>> {
        let _ = (i, wasm, poll, now);
        Err(Error::unsupported("This rail does not upload wallet code"))
    }
    /// As `upload_code`, with the network fee bounded by `max_fee`, in the rail's native units:
    /// an upload whose fee would exceed it is refused before anything is signed. The server pays
    /// an upload, so it bounds what one upload can cost. A rail without the bound uploads
    /// nothing.
    fn upload_code_within(
        &self,
        i: &crate::vendor::runtime::Installation,
        wasm: &[u8],
        max_fee: u64,
        poll: crate::vendor::runtime::Poll,
        now: u64,
    ) -> Result<Option<crate::vendor::runtime::Receipt>> {
        let _ = (i, wasm, max_fee, poll, now);
        Err(Error::unsupported(
            "This rail does not upload wallet code within a fee ceiling",
        ))
    }
    /// Make the installation's wallet code live again when it lapsed, from the server's sending
    /// account, with the network fee bounded by `max_fee` as for `upload_code_within`. `None` when
    /// the code is live. A restoration names the code by its hash, so it needs no code bytes, and
    /// it gives no authority. The server pays it: a wallet operation that restored its code would
    /// repay the code's rent for the network's minimum lifetime. A rail whose code does not lapse
    /// restores nothing.
    fn restore_code_within(
        &self,
        i: &crate::vendor::runtime::Installation,
        max_fee: u64,
        poll: crate::vendor::runtime::Poll,
        now: u64,
    ) -> Result<Option<crate::vendor::runtime::Receipt>> {
        let _ = (i, max_fee, poll, now);
        Err(Error::unsupported("This rail does not restore wallet code"))
    }
    /// Prepare the owner step of a params update on an active wallet. The packet takes the
    /// path of every owner packet: `read_owner_signed`, `submit_owner` and `observe_owner`.
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
}
