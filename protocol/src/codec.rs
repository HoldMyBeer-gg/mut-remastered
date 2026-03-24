use postcard::{from_bytes, to_allocvec};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CodecError {
    #[error("postcard encode error: {0}")]
    Encode(#[from] postcard::Error),
    #[error("message too large: {size} bytes (max {max})")]
    TooLarge { size: usize, max: usize },
}

const MAX_MESSAGE_SIZE: usize = 64 * 1024; // 64 KiB

/// Encode a message to bytes with a 4-byte little-endian length prefix.
pub fn encode_message<T: Serialize>(msg: &T) -> Result<Vec<u8>, CodecError> {
    let payload = to_allocvec(msg)?;
    if payload.len() > MAX_MESSAGE_SIZE {
        return Err(CodecError::TooLarge {
            size: payload.len(),
            max: MAX_MESSAGE_SIZE,
        });
    }
    let len = (payload.len() as u32).to_le_bytes();
    let mut buf = Vec::with_capacity(4 + payload.len());
    buf.extend_from_slice(&len);
    buf.extend_from_slice(&payload);
    Ok(buf)
}

/// Decode a message from a postcard-encoded byte slice (without length prefix).
pub fn decode_message<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, CodecError> {
    Ok(from_bytes(bytes)?)
}
