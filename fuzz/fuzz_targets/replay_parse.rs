//! Any bytes presented as a replay are read or refused; a replay that reads can be played through
//! and written out again to the same records. Half of the inputs are raw bytes; the other half are
//! a script that builds a *valid* replay (so checksums are right and the fuzzer reaches the record
//! logic) and then damages it in a few places.
#![no_main]

use libfuzzer_sys::fuzz_target;
use ss2_net::replay::{Limits, Playback, Record, Replay, ReplayEnd, ReplayHeader, ReplayWriter};

fn scripted(script: &[u8], edits: &[u8]) -> Vec<u8> {
    let header = ReplayHeader {
        build: "fuzz".into(),
        protocol: 1,
        schema_hash: 0,
        tick_hz: 30,
        seed: 0,
        map: "m".into(),
        players: 1,
        meta: vec![],
    };
    let mut writer = ReplayWriter::new(header).expect("a fixed header is valid");
    let mut tick = 0_u64;
    let mut i = 0;
    while i + 3 <= script.len() {
        let kind = script[i] % 4;
        tick += u64::from(script[i + 1] % 3);
        let len = usize::from(script[i + 2] % 32);
        let end = (i + 3 + len).min(script.len());
        let payload = script[i + 3..end].to_vec();
        i = end.max(i + 3);
        let record = match kind {
            0 => Record::Input { tick, player: 0, data: payload },
            1 => Record::Snapshot { tick, data: payload },
            2 => Record::Event { tick, data: payload },
            _ => Record::StateHash { tick, hash: payload.iter().map(|b| u64::from(*b)).sum() },
        };
        let _ = writer.push(record);
    }
    let mut bytes = writer.finish();
    for edit in edits.chunks(3) {
        if let [a, b, c] = edit
            && !bytes.is_empty()
        {
            let position = usize::from(u16::from_le_bytes([*a, *b])) % bytes.len();
            bytes[position] ^= *c;
        }
    }
    bytes
}

fuzz_target!(|data: &[u8]| {
    let Some((mode, rest)) = data.split_first() else {
        return;
    };
    let bytes = if mode & 1 == 0 {
        rest.to_vec()
    } else {
        let (script, edits) = rest.split_at(rest.len() / 2);
        scripted(script, edits)
    };
    let limits = Limits::default();
    let Ok(replay) = Replay::parse(&bytes, &limits) else {
        return;
    };

    // Play it through, seek around, ask for hashes.
    let mut playback = Playback::new(&replay);
    let mut steps = 0;
    while playback.step().is_some() {
        steps += 1;
        assert!(steps <= replay.records().len());
    }
    let _ = replay.snapshot_at_or_before(u64::MAX);
    let _ = playback.seek(replay.last_tick().unwrap_or(0));
    let _ = playback.check_hash(0, 0);
    let _ = replay.keyframes();

    // Whatever was read can be written again and read back to the same records.
    let mut writer = ReplayWriter::new(replay.header().clone()).expect("a header that was read is valid");
    for record in replay.records() {
        writer.push(record.clone()).expect("a record that was read can be written");
    }
    let again = Replay::parse(&writer.finish(), &limits).expect("what was written reads");
    assert_eq!(again.records(), replay.records());
    assert_eq!(again.header(), replay.header());
    assert_eq!(again.end(), &ReplayEnd::Complete);
});
