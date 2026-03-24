//! Network connection to the MUT server using length-prefixed TCP frames.
//!
//! Same wire format as the server: [4-byte LE length][1-byte namespace][postcard payload]

use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;

use protocol::codec::{decode_message, encode_message};

const MAX_FRAME_SIZE: usize = 64 * 1024;

/// Connect to the server and split into read/write halves.
pub async fn connect(addr: &str) -> Result<(OwnedReadHalf, OwnedWriteHalf)> {
    let stream = TcpStream::connect(addr).await?;
    Ok(stream.into_split())
}

/// Read one length-prefixed frame. Returns None on clean disconnect.
pub async fn read_frame(reader: &mut OwnedReadHalf) -> Result<Option<Vec<u8>>> {
    let mut len_buf = [0u8; 4];
    match reader.read_exact(&mut len_buf).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e.into()),
    }
    let payload_len = u32::from_le_bytes(len_buf) as usize;
    if payload_len > MAX_FRAME_SIZE {
        return Err(anyhow::anyhow!("frame too large: {}", payload_len));
    }
    let mut payload = vec![0u8; payload_len];
    reader.read_exact(&mut payload).await?;
    Ok(Some(payload))
}

/// Send a protocol message with namespace prefix.
pub async fn send_message<T: serde::Serialize>(
    writer: &mut OwnedWriteHalf,
    ns: u8,
    msg: &T,
) -> Result<()> {
    let bytes = encode_message(ns, msg)?;
    writer.write_all(&bytes).await?;
    Ok(())
}

/// Incoming server message — decoded from any namespace.
#[derive(Debug)]
pub enum ServerMessage {
    Auth(protocol::auth::ServerMsg),
    Character(protocol::character::ServerMsg),
    Combat(protocol::combat::ServerMsg),
    World(protocol::world::ServerMsg),
    Unknown,
}

/// Try to decode a raw frame payload into a typed server message.
pub fn decode_server_message(payload: &[u8]) -> ServerMessage {
    use protocol::codec::*;

    if let Ok(msg) = decode_message::<protocol::auth::ServerMsg>(NS_AUTH, payload) {
        return ServerMessage::Auth(msg);
    }
    if let Ok(msg) = decode_message::<protocol::character::ServerMsg>(NS_CHAR, payload) {
        return ServerMessage::Character(msg);
    }
    if let Ok(msg) = decode_message::<protocol::combat::ServerMsg>(NS_COMBAT, payload) {
        return ServerMessage::Combat(msg);
    }
    if let Ok(msg) = decode_message::<protocol::world::ServerMsg>(NS_WORLD, payload) {
        return ServerMessage::World(msg);
    }
    ServerMessage::Unknown
}
