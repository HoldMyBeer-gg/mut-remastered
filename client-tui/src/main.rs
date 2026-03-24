use protocol::auth::ClientMsg;
use protocol::codec::{NS_AUTH, decode_message, encode_message};

fn main() {
    // Stub: proves protocol crate compiles into client binary.
    // Full TUI client implementation in Phase 4.
    let msg = ClientMsg::Ping;
    let encoded = encode_message(NS_AUTH, &msg).expect("encode");
    // encoded = [len: 4 bytes][ns: 1 byte][postcard payload...]
    let decoded: ClientMsg = decode_message(NS_AUTH, &encoded[4..]).expect("decode");
    assert_eq!(msg, decoded);
    println!("client-tui: protocol crate linked successfully");
}
