use super::error::{Error, Result, quoted};
/// One token a rail knows by code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    pub code: &'static str,
    pub decimals: u8,
    pub native: bool,
    /// The issuer, mint or token address on this rail, when it has one.
    pub issuer: Option<&'static str>,
}

/// The asset an amount names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asset {
    /// No asset named: a plain INTEGER amount of the wallet's installed asset,
    /// or of vault shares for [`Use::Shares`].
    Installed,
    Native {
        code: String,
    },
    Token {
        code: String,
        issuer: Option<String>,
    },
    /// An asset contract accepted by the selected rail.
    Contract(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Amount {
    pub asset: Asset,
    pub quantity: u128,
    /// Decimals of the asset; `None` for base units of the installed asset.
    pub decimals: Option<u8>,
    /// The original input, for the request journal.
    pub text: String,
}

/// Where an amount appears, which decides the forms it may take.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Use {
    /// `--amount`: a positive amount.
    Spend,
    /// `--token-out ASSET:MIN`: an asset is required and MIN may be zero.
    Minimum,
    /// Vault shares: a positive INTEGER of base units.
    Shares,
}

/// The uses of an operation's `--amount` and `--token-out`: a redeem spends
/// vault shares and a deposit receives them.
pub fn exec_uses(op: &str) -> (Use, Use) {
    match op {
        "redeem" => (Use::Shares, Use::Minimum),
        "deposit" => (Use::Spend, Use::Shares),
        _ => (Use::Spend, Use::Minimum),
    }
}

impl Amount {
    pub fn units(&self) -> u128 {
        self.quantity
    }
}

/// Native identity and units resolved by the selected adapter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedAsset {
    pub symbol: String,
    pub address: String,
    pub decimals: u32,
}

/// Converts a display decimal to base units, refusing extra precision.
pub fn decimal_units(text: &str, decimals: u8) -> Result<u128> {
    let invalid = || {
        Error::usage(format!(
            "{} is not a decimal amount such as 5 or 0.25",
            quoted(text)
        ))
    };
    if text.is_empty() || text.len() > 40 {
        return Err(invalid());
    }
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
        || (text.contains('.') && fraction.is_empty())
        || (whole.len() > 1 && whole.starts_with('0'))
    {
        return Err(invalid());
    }
    if fraction.len() > usize::from(decimals) {
        return Err(Error::usage(format!(
            "{} has more than {decimals} decimal places",
            quoted(text)
        )));
    }
    let scale = 10u128.pow(u32::from(decimals));
    let whole: u128 = whole.parse().map_err(|_| invalid())?;
    let fraction: u128 = if fraction.is_empty() {
        0
    } else {
        format!("{fraction:0<width$}", width = usize::from(decimals))
            .parse()
            .map_err(|_| invalid())?
    };
    whole
        .checked_mul(scale)
        .and_then(|w| w.checked_add(fraction))
        .ok_or_else(|| Error::usage(format!("{} is too large", quoted(text))))
}
