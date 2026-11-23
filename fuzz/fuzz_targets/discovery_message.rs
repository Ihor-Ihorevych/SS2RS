//! Any bytes presented as a discovery datagram are decoded or refused, and what decodes is answered
//! or ignored without ever producing a reply longer than the request; any text presented as an
//! address or an invite code is read or refused, and a code that reads encodes back to the same
//! address. Half of the inputs are raw bytes; the other half build a *valid* reply or query from the
//! input and damage it in a few places, so the fuzzer reaches the checks behind the magic and the
//! lengths.
#![no_main]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use libfuzzer_sys::fuzz_target;
use ss2_net::discovery::{
    Announcer, Handled, Message, RateLimit, Scanner, ServerInfo, decode, decode_invite, encode_info, encode_invite, encode_query,
    parse_direct,
};
use ss2_net::protocol::{Handshake, ProtocolVersion};

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).chars().filter(|c| !c.is_control()).collect()
}

fn scripted(data: &[u8]) -> Vec<u8> {
    let (head, rest) = data.split_at(data.len().min(8));
    let third = rest.len() / 3;
    let info = ServerInfo {
        name: text(&rest[..third]).chars().take(32).collect(),
        map: text(&rest[third..2 * third]).chars().take(16).collect(),
        mode: text(&rest[2 * third..]).chars().take(8).collect(),
        players: head.first().copied().unwrap_or(0) % 65,
        max_players: head.get(1).copied().unwrap_or(0) % 65,
        port: u16::from_le_bytes([head.get(2).copied().unwrap_or(1), head.get(3).copied().unwrap_or(0)]),
        protocol: ProtocolVersion::new(u16::from(head.get(4).copied().unwrap_or(0)), u16::from(head.get(5).copied().unwrap_or(0))),
        schema_hash: u64::from(head.get(6).copied().unwrap_or(0)),
        mods_hash: 0,
        password: head.get(7).copied().unwrap_or(0) & 1 == 1,
    };
    let mut bytes = match encode_info(7, &info) {
        Ok(b) => b,
        Err(_) => encode_query(7, ProtocolVersion::new(0, 1)),
    };
    for edit in head.chunks(2) {
        if let [a, b] = edit
            && !bytes.is_empty()
        {
            let at = usize::from(*a) % bytes.len();
            bytes[at] ^= *b;
        }
    }
    bytes
}

fuzz_target!(|data: &[u8]| {
    let Some((mode, rest)) = data.split_first() else {
        return;
    };
    let datagram = if mode & 1 == 0 { rest.to_vec() } else { scripted(rest) };
    let from = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, rest.first().copied().unwrap_or(1))), 5000);

    // Decoding never panics; what decodes as a query is answered at most by something no longer than the query.
    let decoded = decode(&datagram);
    let info = ServerInfo {
        name: "fuzz".into(),
        map: "m".into(),
        mode: "c".into(),
        players: 0,
        max_players: 4,
        port: 27015,
        protocol: ProtocolVersion::new(0, 1),
        schema_hash: 1,
        mods_hash: 0,
        password: false,
    };
    let mut announcer = Announcer::new(info, RateLimit::default());
    if let Handled::Reply(reply) = announcer.handle(0, from, &datagram) {
        assert!(reply.len() <= datagram.len(), "a reply longer than its query");
        assert!(matches!(decode(&reply), Ok(Message::Info { .. })));
        assert!(matches!(decoded, Ok(Message::Query { .. })));
    }

    // A scanner fed the datagram keeps its list within bounds.
    let mut scanner = Scanner::new(Handshake { version: ProtocolVersion::new(0, 1), schema_hash: 1 });
    scanner.start(0, 7, 1_000_000);
    let _ = scanner.on_datagram(10, from, &datagram);
    assert!(scanner.list().len() <= 256);

    // The same bytes as text: an address, a code.
    let as_text = String::from_utf8_lossy(rest);
    let _ = parse_direct(&as_text);
    if let Some(addr) = decode_invite(&as_text) {
        let again = encode_invite(addr).expect("a decoded code is an address that can be encoded");
        assert_eq!(decode_invite(&again), Some(addr));
    }
});
