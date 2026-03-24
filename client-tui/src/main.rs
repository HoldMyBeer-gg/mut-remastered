use protocol::auth::ClientMsg;

fn main() {
    // Stub: proves protocol crate compiles into client binary.
    // Full TUI client implementation in Phase 4.
    let msg = ClientMsg::Ping;
    let encoded = protocol::codec::encode_message(&msg).expect("encode");
    let decoded: ClientMsg = protocol::codec::decode_message(&encoded[4..]).expect("decode");
    assert_eq!(msg, decoded);
    println!("client-tui: protocol crate linked successfully");
}
