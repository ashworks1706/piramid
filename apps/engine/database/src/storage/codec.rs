//! The binary encoding of every record and sidecar: bincode with fixed-width little-endian
//! integers and u64 length prefixes.

use bincode::config::{Configuration, Fixint, LittleEndian, NoLimit};
use serde::de::DeserializeOwned;
use serde::Serialize;

use piramid_core::error::{Result, StorageError};

/// The bincode configuration every stored byte is written and read with.
const CONFIG: Configuration<LittleEndian, Fixint, NoLimit> = bincode::config::legacy();

/// Encode a value into its stored bytes.
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    Ok(bincode::serde::encode_to_vec(value, CONFIG)?)
}

/// Decode a value from its stored bytes. Bytes left after the value are an error.
pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let (value, read) = bincode::serde::decode_from_slice(bytes, CONFIG)?;
    if read != bytes.len() {
        return Err(StorageError::CorruptedData(format!(
            "{} trailing bytes after decoded value",
            bytes.len() - read
        ))
        .into());
    }
    Ok(value)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "a failed assertion is the point of a test"
)]
mod tests {
    use super::*;

    #[test]
    fn trailing_bytes_after_a_value_are_corruption() {
        let mut bytes = encode(&7u64).unwrap();
        assert_eq!(decode::<u64>(&bytes).unwrap(), 7);

        bytes.extend_from_slice(&[0, 0]);
        let error = decode::<u64>(&bytes).unwrap_err();

        assert!(error.to_string().contains("2 trailing bytes"), "{error}");
    }
}
