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
    fn prepare_owner(
        &self,
        i: &crate::vendor::runtime::Installation,
        a: crate::vendor::runtime::OwnerAction,
        fee: u64,
        now: u64,
    ) -> Result<crate::vendor::runtime::OwnerPacket>;
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
}
