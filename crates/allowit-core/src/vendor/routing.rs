//! Compiled vendor and rail identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Family(pub &'static str);
impl Family {
    pub fn name(self) -> &'static str {
        self.0
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RailInfo {
    pub id: &'static str,
    pub label: &'static str,
    pub aliases: &'static [&'static str],
    pub family: Family,
    pub testnet: bool,
}
