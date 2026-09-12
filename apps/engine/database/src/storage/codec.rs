//! The binary encoding of every record and sidecar: bincode with fixed-width little-endian
//! integers and u64 length prefixes.

use bincode::config::{Configuration, Fixint, LittleEndian, NoLimit};
use serde::de::DeserializeOwned;
use serde::Serialize;

use piramid_core::error::Result;

/// The bincode configuration every stored byte is written and read with.
const CONFIG: Configuration<LittleEndian, Fixint, NoLimit> = bincode::config::legacy();

/// Encode a value into its stored bytes.
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    Ok(bincode::serde::encode_to_vec(value, CONFIG)?)
}

/// Decode a value from the start of its stored bytes. Bytes after the value are ignored.
pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let (value, _read) = bincode::serde::decode_from_slice(bytes, CONFIG)?;
    Ok(value)
}
