//! Any bytes presented as a session message are decoded or refused; what decodes survives a round
//! trip, and a server fed the message never panics and holds at most the slots it has.
#![no_main]

use libfuzzer_sys::fuzz_target;
use ss2_net::protocol::{Handshake, ProtocolVersion};
use ss2_net::session::{PeerId, ServerConfig, SessionServer, decode, encode};

fuzz_target!(|data: &[u8]| {
    let Ok(message) = decode(data) else {
        return;
    };
    // What decodes encodes to something that decodes to the same message.
    let again = decode(&encode(&message)).expect("an encoded message decodes");
    assert_eq!(again, message);

    // A server may be sent anything from anyone.
    let handshake = Handshake { version: ProtocolVersion::new(1, 0), schema_hash: 0 };
    let mut server = SessionServer::new(ServerConfig { max_players: 2, ..ServerConfig::new(handshake) });
    for (peer, now) in [(1_u64, 0_u64), (2, 1), (1, 2), (3, 3)] {
        let _ = server.handle(PeerId(peer), message.clone(), now);
        let _ = server.tick(now + 100, now);
        assert!(server.connected() <= 2);
        assert!(server.tracked_failures() <= ss2_net::session::MAX_TRACKED_PEERS);
    }
});
