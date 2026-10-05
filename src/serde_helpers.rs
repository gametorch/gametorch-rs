//! Serde helpers for the loose typing used by the GameTorch API.
//!
//! Credit and USD amounts are usually encoded as decimal strings
//! (`"5.003952358800"`) but a few endpoints emit JSON numbers (`300`). These
//! helpers accept either representation and decode into [`rust_decimal::Decimal`].

use std::str::FromStr;

use rust_decimal::Decimal;
use serde::{de, Deserialize, Deserializer};

fn value_to_decimal<E>(value: serde_json::Value) -> Result<Decimal, E>
where
    E: de::Error,
{
    match value {
        serde_json::Value::String(s) => Decimal::from_str(&s)
            .or_else(|_| Decimal::from_scientific(&s))
            .map_err(|e| E::custom(format!("invalid decimal string {s:?}: {e}"))),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Decimal::from(i))
            } else if let Some(u) = n.as_u64() {
                Ok(Decimal::from(u))
            } else if let Some(f) = n.as_f64() {
                Decimal::try_from(f)
                    .map_err(|e| E::custom(format!("invalid decimal number {f:?}: {e}")))
            } else {
                Err(E::custom(format!("unsupported decimal number {n}")))
            }
        }
        other => Err(E::custom(format!("expected decimal, found {other}"))),
    }
}

/// Deserializes a required decimal from a string or number.
pub fn decimal<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    value_to_decimal(value)
}

/// Deserializes an optional decimal from a string, number or `null`.
pub fn optional_decimal<'de, D>(deserializer: D) -> Result<Option<Decimal>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(value) => value_to_decimal(value).map(Some),
    }
}

/// Deserializes a `HashMap<String, Decimal>` whose values may be strings or
/// numbers, treating `null` as an empty map.
pub fn decimal_map<'de, D>(
    deserializer: D,
) -> Result<std::collections::HashMap<String, Decimal>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw =
        Option::<std::collections::HashMap<String, serde_json::Value>>::deserialize(deserializer)?;
    let mut out = std::collections::HashMap::new();
    for (key, value) in raw.unwrap_or_default() {
        if matches!(value, serde_json::Value::Null) {
            continue;
        }
        out.insert(key, value_to_decimal(value)?);
    }
    Ok(out)
}

/// Deserializes a count that the API may encode as a boolean (spec) or an
/// integer (wire): `true` maps to `1`, `false` to `0`.
pub fn bool_or_int<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Bool(true) => Ok(1),
        serde_json::Value::Bool(false) => Ok(0),
        serde_json::Value::Null => Ok(0),
        serde_json::Value::Number(n) => n
            .as_i64()
            .ok_or_else(|| de::Error::custom(format!("expected integer, found {n}"))),
        other => Err(de::Error::custom(format!(
            "expected boolean or integer, found {other}"
        ))),
    }
}

/// Deserializes a list of strings leniently: an array of strings is used as-is,
/// while `null` or a numeric count yields an empty list.
pub fn string_list_lenient<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(Vec::new()),
        Some(serde_json::Value::Array(items)) => items
            .into_iter()
            .map(|item| match item {
                serde_json::Value::String(s) => Ok(s),
                other => Err(de::Error::custom(format!("expected string, found {other}"))),
            })
            .collect(),
        Some(serde_json::Value::Number(_)) | Some(serde_json::Value::Bool(_)) => Ok(Vec::new()),
        Some(other) => Err(de::Error::custom(format!(
            "expected string array, found {other}"
        ))),
    }
}
