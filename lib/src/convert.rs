//! Module for free utility functions that convert between JSON and Rust types.

use crate::{consts::TOTAL_PIECE_COUNT, errors::ValueVariant};
use alloc::{vec, vec::Vec};

/// Attempts to convert a JSON value into a byte (`u8`).
///
/// # Errors
/// Returns a `ValueVariant` of the expected type if something went wrong.
pub(crate) fn json_to_u8(value: &serde_json::Value) -> Result<u8, ValueVariant> {
    const EXPECTED_TYPE: ValueVariant = ValueVariant::Byte;

    value
        .as_number()
        .ok_or(EXPECTED_TYPE)?
        .as_u64()
        .ok_or(EXPECTED_TYPE)?
        .try_into()
        .map_err(|_| EXPECTED_TYPE)
}

/// Attempts to convert a JSON value into a long (`u64`).
///
/// # Errors
/// Returns a `ValueVariant` of the expected type if something went wrong.
pub(crate) fn json_to_u64(value: &serde_json::Value) -> Result<u64, ValueVariant> {
    const EXPECTED_TYPE: ValueVariant = ValueVariant::Long;

    value
        .as_number()
        .ok_or(EXPECTED_TYPE)?
        .as_u64()
        .ok_or(EXPECTED_TYPE)
}

/// Attempts to convert a JSON value into a float (`f64`).
///
/// # Errors
/// Returns a `ValueVariant` of the expected type if something went wrong.
pub(crate) fn json_to_f64(value: &serde_json::Value) -> Result<f64, ValueVariant> {
    const EXPECTED_TYPE: ValueVariant = ValueVariant::Float;

    value
        .as_number()
        .ok_or(EXPECTED_TYPE)?
        .as_f64()
        .ok_or(EXPECTED_TYPE)
}

/// Attempts to convert a JSON value into a boolean.
///
/// # Errors
/// Returns a `ValueVariant` of the expected type if something went wrong.
pub(crate) fn json_to_bool(value: &serde_json::Value) -> Result<bool, ValueVariant> {
    value.as_bool().ok_or(ValueVariant::Bool)
}

/// Attempts to convert a JSON value into a string slice.
///
/// # Errors
/// Returns a `ValueVariant` of the expected type if something went wrong.
pub(crate) fn json_to_str(value: &serde_json::Value) -> Result<&str, ValueVariant> {
    value.as_str().ok_or(ValueVariant::String)
}

/// Attempts to convert a JSON value into a byte array for every piece in the game.
///
/// # Errors
/// Returns a `ValueVariant` of the expected type if something went wrong.
pub(crate) fn json_to_piece_bytes(
    value: &serde_json::Value,
) -> Result<[u8; TOTAL_PIECE_COUNT], ValueVariant> {
    const EXPECTED_TYPE: ValueVariant = ValueVariant::PieceArray;

    let values: &[serde_json::Value] = value.as_array().ok_or(EXPECTED_TYPE)?.as_slice();
    let arr =
        <&[serde_json::Value; TOTAL_PIECE_COUNT]>::try_from(values).map_err(|_| EXPECTED_TYPE)?;

    let mut bytes = [0u8; TOTAL_PIECE_COUNT];

    for i in 0..TOTAL_PIECE_COUNT {
        bytes[i] = json_to_u8(&arr[i])?;
    }

    Ok(bytes)
}

/// Attempts to convert a JSON value into a mod list type.
///
/// # Errors
/// Returns a `ValueVariant` of the expected type if something went wrong.
pub(crate) fn json_to_modlist(
    value: &serde_json::Value,
) -> Result<Vec<(u64, serde_json::Value)>, ValueVariant> {
    const EXPECTED_TYPE: ValueVariant = ValueVariant::Array;

    let source = value.as_array().ok_or(EXPECTED_TYPE)?;

    let mut processed_list = Vec::with_capacity(source.len());

    for entry in source {
        let entry = entry.as_array().ok_or(EXPECTED_TYPE)?.as_slice();
        let [mod_id, mod_value] = <&[serde_json::Value; 2]>::try_from(entry)
            .ok()
            .ok_or(EXPECTED_TYPE)?;
        let mod_id = mod_id.as_u64().ok_or(EXPECTED_TYPE)?;
        let mod_value = mod_value.clone();

        processed_list.push((mod_id, mod_value));
    }

    Ok(processed_list)
}

/// Converts a modlist into JSON format.
pub(crate) fn modlist_to_json(modlist: Vec<(u64, serde_json::Value)>) -> serde_json::Value {
    let values: Vec<serde_json::Value> = modlist
        .into_iter()
        .map(|(mod_id, mod_value)| {
            serde_json::Value::Array(vec![
                serde_json::Value::Number(serde_json::Number::from(mod_id)),
                mod_value,
            ])
        })
        .collect();

    serde_json::Value::Array(values)
}
