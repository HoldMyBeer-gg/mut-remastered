use postcard::{from_bytes, to_allocvec};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CodecError {
    #[error("postcard encode error: {0}")]
    Encode(#[from] postcard::Error),
    #[error("message too large: {size} bytes (max {max})")]
    TooLarge { size: usize, max: usize },
    #[error("wrong message namespace: expected 0x{expected:02x}, got 0x{got:02x}")]
    WrongNamespace { expected: u8, got: u8 },
    #[error("payload too short (missing namespace byte)")]
    MissingNamespace,
}

const MAX_MESSAGE_SIZE: usize = 64 * 1024; // 64 KiB

/// Message namespace tag bytes prepended to every payload.
///
/// Adding a namespace byte to the wire format ensures unambiguous message type
/// identification. Without it, postcard's lenient `from_bytes` (which ignores
/// trailing bytes) causes unit-variant enum members (e.g., auth::Logout at
/// variant index 2) to silently match data-carrying world messages
/// (e.g., world::Examine at variant index 2), corrupting dispatch.
pub const NS_AUTH: u8 = 0x01;
pub const NS_WORLD: u8 = 0x02;
pub const NS_CHAR: u8 = 0x03;

/// Encode a message to bytes with a 4-byte LE length prefix and 1-byte namespace tag.
///
/// Wire format: `[len: u32 LE][ns: u8][postcard payload...]`
/// The length stored in the prefix includes the namespace byte.
pub fn encode_message<T: Serialize>(ns: u8, msg: &T) -> Result<Vec<u8>, CodecError> {
    let payload = to_allocvec(msg)?;
    let total = 1 + payload.len(); // namespace byte + payload
    if total > MAX_MESSAGE_SIZE {
        return Err(CodecError::TooLarge {
            size: total,
            max: MAX_MESSAGE_SIZE,
        });
    }
    let len = (total as u32).to_le_bytes();
    let mut buf = Vec::with_capacity(4 + total);
    buf.extend_from_slice(&len);
    buf.push(ns);
    buf.extend_from_slice(&payload);
    Ok(buf)
}

/// Decode a message from a namespace-prefixed payload byte slice.
///
/// `bytes` is the frame payload AFTER the 4-byte length prefix has been stripped.
/// The first byte must match `expected_ns`; the remaining bytes are decoded as `T`.
pub fn decode_message<'a, T: Deserialize<'a>>(
    expected_ns: u8,
    bytes: &'a [u8],
) -> Result<T, CodecError> {
    let (&ns, payload) = bytes.split_first().ok_or(CodecError::MissingNamespace)?;
    if ns != expected_ns {
        return Err(CodecError::WrongNamespace {
            expected: expected_ns,
            got: ns,
        });
    }
    Ok(from_bytes(payload)?)
}
