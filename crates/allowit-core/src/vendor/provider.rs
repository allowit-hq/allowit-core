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
    /// read after that, holds none. Once that history no longer holds the operation's
    /// preparation, the time bound alone decides, so an operation that was never sent always
    /// ends. A caller releases an open operation on its receipt or on this lapse. `false` means
    /// that the operation can still execute, that it executed, or that the rail cannot tell, and
    /// it stays open; a read that fails is an error. A rail without such a bound keeps this
    /// default.
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
    /// Prepare the upload of the wallet code `wasm` for the installation, from one of the server's
    /// sending accounts and signed by it, with the network fee bounded by `max_fee` in the rail's
    /// native units. A fee above `max_fee` is refused before anything is signed. `None` when the
    /// network has the code live: nothing is needed. The server pays the upload, and it gives no
    /// authority: the owner's creation names the code by its hash. Nothing restores code that
    /// lapsed.
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
    /// is not renewed: nothing restores it. The packet takes the path of a code upload: the
    /// caller sends it with `broadcast` and reads its receipt with `observe`. A rail without
    /// code renewals refuses.
    fn prepare_code_renewal(
        &self,
        code: &[u8; 32],
        max_fee: u64,
        now: u64,
    ) -> Result<Option<crate::vendor::runtime::Signed>> {
        let _ = (code, max_fee, now);
        Err(Error::unsupported("This rail does not renew wallet code"))
    }
    /// Whether the code transaction `p` from `prepare_code_within` or `prepare_code_renewal` is
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
