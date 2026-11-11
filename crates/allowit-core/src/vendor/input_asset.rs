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

/// What a host shows for an asset. The rail plugin supplies it; shared code holds no unit.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetDisplay {
    /// The unit symbol or asset code that follows an amount.
    pub symbol: String,
    pub decimals: u8,
    /// The human name of the asset.
    pub name: String,
    pub native: bool,
}
impl From<&Token> for AssetDisplay {
    fn from(token: &Token) -> Self {
        Self {
            symbol: token.code.into(),
            decimals: token.decimals,
            name: token.code.into(),
            native: token.native,
        }
    }
}
impl AssetDisplay {
    /// Base units as an amount with its unit, such as `5 XYZ` or `-0.25 XYZ`.
    pub fn format(&self, units: i128) -> String {
        let sign = if units < 0 { "-" } else { "" };
        let amount = units_decimal(units.unsigned_abs(), self.decimals);
        format!("{sign}{amount} {}", self.symbol)
    }
}

/// Base units as an exact display decimal without trailing zeros: the inverse of
/// [`decimal_units`].
pub fn units_decimal(units: u128, decimals: u8) -> String {
    let digits = units.to_string();
    let scale = usize::from(decimals);
    let digits = format!("{digits:0>width$}", width = scale + 1);
    let (whole, fraction) = digits.split_at(digits.len() - scale);
    let fraction = fraction.trim_end_matches('0');
    if fraction.is_empty() {
        whole.into()
    } else {
        format!("{whole}.{fraction}")
    }
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
    let scale = 10u128
        .checked_pow(u32::from(decimals))
        .ok_or_else(|| Error::usage(format!("{} is too large", quoted(text))))?;
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn amounts_print_exact_decimals_with_the_asset_unit() {
        let asset = AssetDisplay {
            symbol: "XYZ".into(),
            decimals: 7,
            name: "Example".into(),
            native: true,
        };
        assert_eq!(asset.format(50_000_000), "5 XYZ");
        assert_eq!(asset.format(2_500_000), "0.25 XYZ");
        assert_eq!(asset.format(-1), "-0.0000001 XYZ");
        assert_eq!(asset.format(0), "0 XYZ");
        assert_eq!(units_decimal(u128::MAX, 0), u128::MAX.to_string());
        assert_eq!(units_decimal(1, 255).len(), 257);
        assert_eq!(
            asset.format(i128::MIN),
            format!("-{} XYZ", units_decimal(1 << 127, 7))
        );
        for (text, decimals) in [("5", 7), ("0.25", 7), ("123456789.0000001", 7), ("7", 0)] {
            let units = decimal_units(text, decimals).unwrap();
            assert_eq!(units_decimal(units, decimals), text);
        }
        let json = serde_json::to_value(&asset).unwrap();
        assert_eq!(json["symbol"], "XYZ");
        assert_eq!(json["decimals"], 7);
    }
    #[test]
    fn unsupported_decimal_scales_return_an_error() {
        assert_eq!(decimal_units("1", 38).unwrap(), 10u128.pow(38));
        for decimals in [39, 255] {
            assert!(decimal_units("1", decimals).is_err());
        }
    }
}
