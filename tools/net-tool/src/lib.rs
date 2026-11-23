//! `net-tool` (roadmap T11-010): look inside replay files and see what a bad network does to a stream of
//! packets, without a game.
//!
//! * **Replays** (`ss2_net::replay`, `docs/REPLAY.md`): [`inspect_replay`] reads a file and reports its
//!   header, what it holds, which ticks have snapshots and **how it ended** (complete, or cut short and
//!   why); [`dump_records`] lists the records of a tick range.
//! * **The network simulator** (`ss2_net::netsim`, `docs/NETSIM.md`): [`run_scenario`] sends a steady stream of
//!   packets through a link (a named [preset](PRESETS) with optional overrides) and reports delivery,
//!   loss, duplication, reordering as seen by the receiver, and the latency percentiles. The same seed
//!   always gives the same report.
//!
//! What it does **not** do: record a session (there is no simulation to record yet) or play one back
//! (the playback driver is T08-025b), and the scenario is one steady one-way stream, not a game's traffic.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use ss2_net::netsim::{LinkConfig, NetSim};
use ss2_net::replay::{
    Limits, Record, Replay, ReplayEnd, ReplayError, ReplayHeader, TruncationReason,
};

// ---- replays ----------------------------------------------------------------------------------

/// What a replay file holds and how it ended; see [`inspect_replay`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayReport {
    /// The header.
    pub header: ReplayHeader,
    /// The size of the file.
    pub file_bytes: usize,
    /// The records that could be read.
    pub records: usize,
    /// Input records.
    pub inputs: usize,
    /// Snapshot records.
    pub snapshots: usize,
    /// Event records.
    pub events: usize,
    /// State-hash records.
    pub state_hashes: usize,
    /// The tick of the first record.
    pub first_tick: Option<u64>,
    /// The tick of the last record.
    pub last_tick: Option<u64>,
    /// The ticks that have a snapshot.
    pub keyframes: Vec<u64>,
    /// Input records per player, by player number.
    pub inputs_per_player: Vec<(u8, u64)>,
    /// How the file ended.
    pub end: ReplayEnd,
}

/// Read a replay file's bytes and report on it. Only a bad header is an error; a file that was
/// cut short is reported as such, with the records before the cut.
pub fn inspect_replay(bytes: &[u8]) -> Result<ReplayReport, ReplayError> {
    let replay = Replay::parse(bytes, &Limits::default())?;
    let mut report = ReplayReport {
        header: replay.header().clone(),
        file_bytes: bytes.len(),
        records: replay.records().len(),
        inputs: 0,
        snapshots: 0,
        events: 0,
        state_hashes: 0,
        first_tick: replay.records().first().map(Record::tick),
        last_tick: replay.last_tick(),
        keyframes: replay.keyframes(),
        inputs_per_player: Vec::new(),
        end: replay.end().clone(),
    };
    let mut per_player: BTreeMap<u8, u64> = BTreeMap::new();
    for record in replay.records() {
        match record {
            Record::Input { player, .. } => {
                report.inputs += 1;
                *per_player.entry(*player).or_default() += 1;
            }
            Record::Snapshot { .. } => report.snapshots += 1,
            Record::Event { .. } => report.events += 1,
            Record::StateHash { .. } => report.state_hashes += 1,
        }
    }
    report.inputs_per_player = per_player.into_iter().collect();
    Ok(report)
}

fn truncation_text(reason: TruncationReason) -> String {
    match reason {
        TruncationReason::MissingEnd => "no end record (the recorder never finished)".into(),
        TruncationReason::CutShort => "the file ends inside a record".into(),
        TruncationReason::BadChecksum => "a record fails its checksum".into(),
        TruncationReason::UnknownKind(k) => format!("a record of an unknown kind ({k})"),
        TruncationReason::TooLarge => "a record is larger than the limit".into(),
        TruncationReason::OutOfOrder => {
            "a record is for an earlier tick than the one before".into()
        }
        TruncationReason::TooManyRecords => "more records than the limit".into(),
        TruncationReason::BadPayload => "a record's payload is malformed".into(),
        TruncationReason::CountMismatch => {
            "the end record's count does not match the records".into()
        }
        TruncationReason::TrailingData => "bytes follow the end record".into(),
    }
}

/// `m:ss.mmm` for `ticks` at `tick_hz` (rounded to the millisecond).
fn clock(ticks: u64, tick_hz: u32) -> String {
    let hz = u128::from(tick_hz.max(1));
    let ms = (u128::from(ticks) * 1000 + hz / 2) / hz;
    format!("{}:{:02}.{:03}", ms / 60_000, ms / 1000 % 60, ms % 1000)
}

impl ReplayReport {
    /// Whether the file has a good end record: nothing was lost.
    pub fn is_complete(&self) -> bool {
        self.end == ReplayEnd::Complete
    }

    /// The report as text.
    pub fn render(&self) -> String {
        let h = &self.header;
        let mut out = String::new();
        let _ = writeln!(out, "replay: {}", h.build);
        let _ = writeln!(out, "map: {}", h.map);
        let _ = writeln!(out, "tick rate: {} ticks per second", h.tick_hz);
        let _ = writeln!(out, "players: {}", h.players);
        let _ = writeln!(out, "seed: {}", h.seed);
        let _ = writeln!(
            out,
            "protocol: {}, schema hash: {:#018x}",
            h.protocol, h.schema_hash
        );
        for (k, v) in &h.meta {
            let _ = writeln!(out, "note: {k} = {v}");
        }
        let _ = writeln!(
            out,
            "records: {} in {} bytes",
            self.records, self.file_bytes
        );
        let _ = writeln!(out, "inputs: {}", self.inputs);
        for (player, count) in &self.inputs_per_player {
            let _ = writeln!(out, "  player {player}: {count}");
        }
        let shown: Vec<String> = self.keyframes.iter().take(8).map(u64::to_string).collect();
        let more = self.keyframes.len().saturating_sub(8);
        let _ = writeln!(
            out,
            "snapshots: {} (ticks {}{})",
            self.snapshots,
            shown.join(", "),
            if more > 0 {
                format!(", and {more} more")
            } else {
                String::new()
            }
        );
        let _ = writeln!(out, "events: {}", self.events);
        let _ = writeln!(out, "state hashes: {}", self.state_hashes);
        match (self.first_tick, self.last_tick) {
            (Some(first), Some(last)) => {
                let _ = writeln!(
                    out,
                    "ticks {first} to {last} ({} at {} Hz)",
                    clock(last, h.tick_hz),
                    h.tick_hz
                );
            }
            _ => {
                let _ = writeln!(out, "no records");
            }
        }
        match &self.end {
            ReplayEnd::Complete => {
                let _ = writeln!(out, "end: complete");
            }
            ReplayEnd::Truncated { offset, reason } => {
                let _ = writeln!(
                    out,
                    "end: TRUNCATED at byte {offset}: {}",
                    truncation_text(*reason)
                );
            }
        }
        out
    }
}

/// One line per record whose tick is in `from..=to` (either end open when `None`), in file order.
pub fn dump_records(
    bytes: &[u8],
    from: Option<u64>,
    to: Option<u64>,
) -> Result<Vec<String>, ReplayError> {
    let replay = Replay::parse(bytes, &Limits::default())?;
    let lines = replay
        .records()
        .iter()
        .filter(|r| from.is_none_or(|f| r.tick() >= f) && to.is_none_or(|t| r.tick() <= t))
        .map(|r| match r {
            Record::Input { tick, player, data } => {
                format!("tick {tick} input player {player} {} bytes", data.len())
            }
            Record::Snapshot { tick, data } => format!("tick {tick} snapshot {} bytes", data.len()),
            Record::Event { tick, data } => format!("tick {tick} event {} bytes", data.len()),
            Record::StateHash { tick, hash } => format!("tick {tick} state-hash {hash:#018x}"),
        })
        .collect();
    Ok(lines)
}

// ---- the network simulator -----------------------------------------------------------------------

/// The named link profiles and what they are.
pub const PRESETS: &[(&str, &str)] = &[
    ("perfect", "no delay, no loss, unlimited bandwidth"),
    ("lan", "wired local network: 0.5 ms, 0.2 ms jitter"),
    ("broadband", "15 ms, 5 ms jitter, 0.1 % loss"),
    ("mobile", "50 ms, 20 ms jitter, 1 % loss"),
    ("bad_wifi", "30 ms, 40 ms jitter, 5 % loss, 2 % reordering"),
    ("satellite", "300 ms, 30 ms jitter, 0.5 % loss"),
    (
        "acceptance_100ms_2pct",
        "the roadmap's acceptance profile: 100 ms and 2 % loss, nothing else",
    ),
];

/// The link of a named profile; `None` for a name that is not in [`PRESETS`].
pub fn preset(name: &str) -> Option<LinkConfig> {
    Some(match name {
        "perfect" => LinkConfig::default(),
        "lan" => LinkConfig::lan(),
        "broadband" => LinkConfig::broadband(),
        "mobile" => LinkConfig::mobile(),
        "bad_wifi" => LinkConfig::bad_wifi(),
        "satellite" => LinkConfig::satellite(),
        "acceptance_100ms_2pct" => LinkConfig::acceptance_100ms_2pct(),
        _ => return None,
    })
}

/// A percentage from 0 to 100 as parts per million.
pub fn percent_to_ppm(percent: f64) -> Result<u32, String> {
    if !percent.is_finite() || !(0.0..=100.0).contains(&percent) {
        return Err(format!("{percent} is not a percentage from 0 to 100"));
    }
    Ok((percent * 10_000.0).round() as u32)
}

/// Changes to a preset, each applied only when given.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Overrides {
    /// One-way delay in milliseconds.
    pub latency_ms: Option<u64>,
    /// Jitter in milliseconds.
    pub jitter_ms: Option<u64>,
    /// Loss in percent.
    pub loss_pct: Option<f64>,
    /// Duplication in percent.
    pub duplicate_pct: Option<f64>,
    /// Reordering in percent (a link with no reorder delay gets 30 ms so that it has an effect).
    pub reorder_pct: Option<f64>,
    /// Bandwidth in kilobytes (1000 bytes) per second; at least 1.
    pub bandwidth_kbytes_per_sec: Option<u64>,
    /// The tail-drop queue in bytes (only meaningful with a bandwidth limit).
    pub queue_limit_bytes: Option<usize>,
}

fn ms_to_us(ms: u64) -> Result<u64, String> {
    ms.checked_mul(1000)
        .ok_or_else(|| format!("{ms} ms is too long"))
}

impl Overrides {
    /// `base` with these changes; an error for a value out of range.
    pub fn apply(&self, base: &LinkConfig) -> Result<LinkConfig, String> {
        let mut link = base.clone();
        if let Some(ms) = self.latency_ms {
            link.latency_us = ms_to_us(ms)?;
        }
        if let Some(ms) = self.jitter_ms {
            link.jitter_us = ms_to_us(ms)?;
        }
        if let Some(p) = self.loss_pct {
            link.loss_ppm = percent_to_ppm(p)?;
        }
        if let Some(p) = self.duplicate_pct {
            link.duplicate_ppm = percent_to_ppm(p)?;
        }
        if let Some(p) = self.reorder_pct {
            link.reorder_ppm = percent_to_ppm(p)?;
            if link.reorder_delay_us == 0 {
                link.reorder_delay_us = 30_000;
            }
        }
        if let Some(kb) = self.bandwidth_kbytes_per_sec {
            if kb == 0 {
                return Err("a bandwidth of 0 carries nothing".into());
            }
            link.bandwidth_bytes_per_sec =
                Some(kb.checked_mul(1000).ok_or("the bandwidth is too large")?);
        }
        if let Some(q) = self.queue_limit_bytes {
            link.queue_limit_bytes = q;
        }
        link.validate()?;
        Ok(link)
    }
}

/// The most packets a scenario may send.
pub const MAX_PACKETS: u32 = 5_000_000;

/// A steady one-way stream of packets through one link.
#[derive(Clone, Debug, PartialEq)]
pub struct Scenario {
    /// What the link does.
    pub link: LinkConfig,
    /// The random seed: the same seed gives the same report.
    pub seed: u64,
    /// How many packets to send, 1 to [`MAX_PACKETS`].
    pub packets: u32,
    /// The size of each packet in bytes, at least 1.
    pub size_bytes: usize,
    /// The time between packets in microseconds (0 sends them all at once).
    pub interval_us: u64,
}

/// Latency of the packets that arrived (the first copy of each), in microseconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Latency {
    /// The smallest.
    pub min_us: u64,
    /// The mean (rounded down).
    pub mean_us: u64,
    /// The median.
    pub p50_us: u64,
    /// The 95th percentile.
    pub p95_us: u64,
    /// The 99th percentile.
    pub p99_us: u64,
    /// The largest.
    pub max_us: u64,
}

/// What a link did to a scenario, as the receiver saw it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioReport {
    /// The scenario's packet size, for the text.
    pub size_bytes: usize,
    /// The scenario's interval, for the text.
    pub interval_us: u64,
    /// Packets sent.
    pub sent: u64,
    /// Packets that arrived at least once.
    pub delivered_unique: u64,
    /// Packets lost on the wire.
    pub lost: u64,
    /// Packets refused by a full queue.
    pub dropped_queue: u64,
    /// Second copies that arrived.
    pub duplicates_received: u64,
    /// Packets that arrived after a later-sent packet had arrived.
    pub out_of_order: u64,
    /// Latency, `None` when nothing arrived.
    pub latency: Option<Latency>,
}

/// The value at percentile `p` (1 to 100) of a sorted slice, by the nearest-rank rule; `None` for an empty slice.
pub fn percentile(sorted: &[u64], p: u32) -> Option<u64> {
    if sorted.is_empty() {
        return None;
    }
    let n = sorted.len() as u64;
    let rank = (u64::from(p) * n).div_ceil(100).max(1);
    sorted
        .get((rank - 1) as usize)
        .copied()
        .or(sorted.last().copied())
}

/// How many packets in `arrivals` (the sequence numbers in the order the first copies arrived)
/// came after a packet that was sent later.
pub fn count_out_of_order(arrivals: &[u32]) -> u64 {
    let mut newest: Option<u32> = None;
    let mut count = 0;
    for &index in arrivals {
        if newest.is_some_and(|n| index < n) {
            count += 1;
        }
        newest = Some(newest.map_or(index, |n| n.max(index)));
    }
    count
}

/// Send the scenario's packets through the link and report what arrived.
pub fn run_scenario(scenario: &Scenario) -> Result<ScenarioReport, String> {
    if scenario.packets == 0 {
        return Err("there are no packets to send".into());
    }
    if scenario.packets > MAX_PACKETS {
        return Err(format!(
            "{} packets is more than the limit of {MAX_PACKETS}",
            scenario.packets
        ));
    }
    if scenario.size_bytes == 0 {
        return Err("a packet needs at least 1 byte".into());
    }
    scenario.link.validate()?;
    let mut sim: NetSim<u32> = NetSim::new(scenario.link.clone(), scenario.seed);
    let mut send_us: Vec<u64> = Vec::with_capacity(scenario.packets as usize);
    let mut seen = vec![false; scenario.packets as usize];
    let mut latencies: Vec<u64> = Vec::new();
    let mut duplicates = 0u64;
    let mut arrivals: Vec<u32> = Vec::new();
    let mut take = |sim: &mut NetSim<u32>, now_us: u64, send_us: &[u64]| {
        for (at_us, index) in sim.poll(now_us) {
            let slot = &mut seen[index as usize];
            if *slot {
                duplicates += 1;
                continue;
            }
            *slot = true;
            latencies.push(at_us.saturating_sub(send_us[index as usize]));
            arrivals.push(index);
        }
    };
    for i in 0..scenario.packets {
        let at_us = u64::from(i).saturating_mul(scenario.interval_us);
        send_us.push(at_us);
        sim.send(at_us, i, scenario.size_bytes);
        take(&mut sim, at_us, &send_us);
    }
    take(&mut sim, u64::MAX, &send_us);
    let stats = sim.stats();
    latencies.sort_unstable();
    let latency = if latencies.is_empty() {
        None
    } else {
        let sum: u128 = latencies.iter().map(|&l| u128::from(l)).sum();
        Some(Latency {
            min_us: latencies[0],
            mean_us: (sum / latencies.len() as u128) as u64,
            p50_us: percentile(&latencies, 50).unwrap_or(0),
            p95_us: percentile(&latencies, 95).unwrap_or(0),
            p99_us: percentile(&latencies, 99).unwrap_or(0),
            max_us: latencies[latencies.len() - 1],
        })
    };
    Ok(ScenarioReport {
        size_bytes: scenario.size_bytes,
        interval_us: scenario.interval_us,
        sent: stats.sent,
        delivered_unique: latencies.len() as u64,
        lost: stats.dropped_loss,
        dropped_queue: stats.dropped_queue,
        duplicates_received: duplicates,
        out_of_order: count_out_of_order(&arrivals),
        latency,
    })
}

fn ms3(us: u64) -> String {
    format!("{}.{:03}", us / 1000, us % 1000)
}

impl ScenarioReport {
    /// Packets lost on the wire as a percentage of those sent.
    pub fn loss_percent(&self) -> f64 {
        if self.sent == 0 {
            0.0
        } else {
            100.0 * self.lost as f64 / self.sent as f64
        }
    }

    /// The report as text.
    pub fn render(&self) -> String {
        let pct = |n: u64| {
            if self.sent == 0 {
                0.0
            } else {
                100.0 * n as f64 / self.sent as f64
            }
        };
        let mut out = String::new();
        let _ = writeln!(
            out,
            "sent: {} packets of {} bytes, one every {} ms",
            self.sent,
            self.size_bytes,
            ms3(self.interval_us)
        );
        let _ = writeln!(
            out,
            "received: {} ({:.2} %)",
            self.delivered_unique,
            pct(self.delivered_unique)
        );
        let _ = writeln!(
            out,
            "lost on the wire: {} ({:.2} %)",
            self.lost,
            pct(self.lost)
        );
        let _ = writeln!(
            out,
            "dropped by a full queue: {} ({:.2} %)",
            self.dropped_queue,
            pct(self.dropped_queue)
        );
        let _ = writeln!(out, "duplicates received: {}", self.duplicates_received);
        let _ = writeln!(out, "out of order: {}", self.out_of_order);
        match &self.latency {
            Some(l) => {
                let _ = writeln!(
                    out,
                    "latency (ms): min {} · mean {} · p50 {} · p95 {} · p99 {} · max {}",
                    ms3(l.min_us),
                    ms3(l.mean_us),
                    ms3(l.p50_us),
                    ms3(l.p95_us),
                    ms3(l.p99_us),
                    ms3(l.max_us)
                );
            }
            None => {
                let _ = writeln!(out, "latency: no packet arrived");
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss2_net::replay::{Record, ReplayHeader, ReplayWriter};

    fn header() -> ReplayHeader {
        ReplayHeader {
            build: "ss2rs test".into(),
            protocol: 3,
            schema_hash: 0xABCD_EF01_2345_6789,
            tick_hz: 60,
            seed: 42,
            map: "Hatshepsut".into(),
            players: 2,
            meta: vec![("mode".into(), "coop".into())],
        }
    }

    /// Ticks 0 to 10: a snapshot at 0 and 5, two players' inputs every tick, an event at 3 and a hash every tick.
    fn replay_bytes() -> Vec<u8> {
        let mut w = ReplayWriter::new(header()).unwrap();
        for tick in 0..=10u64 {
            if tick % 5 == 0 {
                w.push(Record::Snapshot {
                    tick,
                    data: vec![1; 32],
                })
                .unwrap();
            }
            w.push(Record::Input {
                tick,
                player: 0,
                data: vec![7; 4],
            })
            .unwrap();
            if tick != 4 {
                w.push(Record::Input {
                    tick,
                    player: 1,
                    data: vec![9; 4],
                })
                .unwrap();
            }
            if tick == 3 {
                w.push(Record::Event {
                    tick,
                    data: vec![5; 2],
                })
                .unwrap();
            }
            w.push(Record::StateHash {
                tick,
                hash: tick * 1000,
            })
            .unwrap();
        }
        w.finish()
    }

    // ---- replay ---------------------------------------------------------------------------

    #[test]
    fn a_replay_report_counts_what_the_file_holds() {
        let bytes = replay_bytes();
        let r = inspect_replay(&bytes).unwrap();
        assert_eq!(
            (r.inputs, r.snapshots, r.events, r.state_hashes),
            (21, 3, 1, 11)
        );
        assert_eq!(r.records, 36);
        assert_eq!((r.first_tick, r.last_tick), (Some(0), Some(10)));
        assert_eq!(r.keyframes, vec![0, 5, 10]);
        assert_eq!(r.inputs_per_player, vec![(0, 11), (1, 10)]);
        assert_eq!(r.header.map, "Hatshepsut");
        assert_eq!(r.file_bytes, bytes.len());
        assert!(r.is_complete());
    }

    #[test]
    fn the_rendered_report_shows_the_header_the_counts_and_how_it_ended() {
        let text = inspect_replay(&replay_bytes()).unwrap().render();
        for want in [
            "ss2rs test",
            "Hatshepsut",
            "60 ticks per second",
            "players: 2",
            "seed: 42",
            "0xabcdef0123456789",
            "mode = coop",
            "inputs: 21",
            "snapshots: 3 (ticks 0, 5, 10)",
            "player 0: 11",
            "player 1: 10",
            "ticks 0 to 10 (0:00.167 at 60 Hz)",
            "complete",
        ] {
            assert!(text.contains(want), "missing {want:?} in:\n{text}");
        }
    }

    #[test]
    fn a_replay_that_was_cut_short_reports_how_and_where_and_keeps_the_good_records() {
        let bytes = replay_bytes();
        let cut = &bytes[..bytes.len() - 40];
        let r = inspect_replay(cut).unwrap();
        assert!(!r.is_complete());
        assert!(r.records > 0 && r.records < 36, "{}", r.records);
        let text = r.render();
        assert!(
            text.contains("TRUNCATED") && text.contains("byte"),
            "{text}"
        );
    }

    #[test]
    fn a_file_that_is_not_a_replay_is_an_error() {
        assert!(inspect_replay(b"hello").is_err());
        assert!(inspect_replay(&[]).is_err());
        let mut damaged = replay_bytes();
        damaged[6] ^= 0xFF; // inside the header
        assert!(inspect_replay(&damaged).is_err());
    }

    #[test]
    fn dump_lists_the_records_of_a_tick_range() {
        let bytes = replay_bytes();
        let lines = dump_records(&bytes, Some(3), Some(4)).unwrap();
        assert_eq!(
            lines,
            vec![
                "tick 3 input player 0 4 bytes",
                "tick 3 input player 1 4 bytes",
                "tick 3 event 2 bytes",
                "tick 3 state-hash 0x0000000000000bb8",
                "tick 4 input player 0 4 bytes",
                "tick 4 state-hash 0x0000000000000fa0",
            ]
        );
        let all = dump_records(&bytes, None, None).unwrap();
        assert_eq!(all.len(), 36);
        assert_eq!(all[0], "tick 0 snapshot 32 bytes");
        assert!(dump_records(&bytes, Some(11), None).unwrap().is_empty());
        assert!(
            dump_records(&bytes, Some(5), Some(3)).unwrap().is_empty(),
            "an empty range is empty, not an error"
        );
        assert!(dump_records(b"nope", None, None).is_err());
    }

    // ---- percentiles and conversions ----------------------------------------------------------

    #[test]
    fn percentiles_use_the_nearest_rank() {
        let v: Vec<u64> = (1..=100).collect();
        assert_eq!(percentile(&v, 50), Some(50));
        assert_eq!(percentile(&v, 95), Some(95));
        assert_eq!(percentile(&v, 99), Some(99));
        assert_eq!(percentile(&v, 100), Some(100));
        assert_eq!(percentile(&v, 1), Some(1));
        assert_eq!(percentile(&[42], 99), Some(42));
        assert_eq!(percentile(&[], 50), None);
        assert_eq!(percentile(&[1, 2, 3, 4], 50), Some(2));
        assert_eq!(
            percentile(&[1, 2, 3], 50),
            Some(2),
            "rank 1.5 rounds up to 2"
        );
    }

    #[test]
    fn out_of_order_counts_packets_that_arrive_after_a_later_one() {
        assert_eq!(count_out_of_order(&[]), 0);
        assert_eq!(count_out_of_order(&[0, 1, 2, 3]), 0);
        assert_eq!(count_out_of_order(&[1, 0]), 1);
        assert_eq!(count_out_of_order(&[0, 2, 1, 3]), 1);
        assert_eq!(
            count_out_of_order(&[5, 3, 4]),
            2,
            "both came after 5, even though 4 came after 3"
        );
        assert_eq!(count_out_of_order(&[3, 2, 1, 0]), 3);
    }

    #[test]
    fn the_clock_text_rolls_over_minutes_and_rounds_to_the_millisecond() {
        assert_eq!(clock(0, 60), "0:00.000");
        assert_eq!(clock(60, 60), "0:01.000");
        assert_eq!(clock(3600, 60), "1:00.000");
        assert_eq!(clock(3659, 60), "1:00.983");
        assert_eq!(clock(10, 60), "0:00.167");
        assert_eq!(clock(5, 0), "0:05.000", "a tick rate of 0 is read as 1");
    }

    #[test]
    fn percent_to_parts_per_million_is_exact_and_checked() {
        assert_eq!(percent_to_ppm(2.0), Ok(20_000));
        assert_eq!(percent_to_ppm(0.0), Ok(0));
        assert_eq!(percent_to_ppm(100.0), Ok(1_000_000));
        assert_eq!(percent_to_ppm(0.5), Ok(5_000));
        assert_eq!(percent_to_ppm(0.0001), Ok(1));
        assert_eq!(
            percent_to_ppm(0.0003),
            Ok(3),
            "rounded, not cut off: 0.0003 % is 2.9999999999999996 ppm in floating point"
        );
        assert_eq!(
            percent_to_ppm(0.57),
            Ok(5_700),
            "0.57 % is 5699.999999999999 ppm in floating point"
        );
        for bad in [-1.0, 100.5, f64::NAN, f64::INFINITY] {
            assert!(percent_to_ppm(bad).is_err(), "{bad}");
        }
    }

    // ---- presets and overrides -----------------------------------------------------------------

    #[test]
    fn every_preset_resolves_and_is_valid_and_an_unknown_name_is_not_a_preset() {
        assert!(PRESETS.len() >= 6);
        for (name, description) in PRESETS {
            let link = preset(name).unwrap_or_else(|| panic!("{name}"));
            link.validate().unwrap();
            assert!(!description.is_empty());
        }
        assert_eq!(preset("lan").unwrap(), ss2_net::netsim::LinkConfig::lan());
        assert_eq!(
            preset("perfect").unwrap(),
            ss2_net::netsim::LinkConfig::default()
        );
        assert!(preset("dialup").is_none());
        assert!(preset("").is_none());
    }

    #[test]
    fn overrides_change_only_what_they_name() {
        let base = preset("broadband").unwrap();
        let none = Overrides::default().apply(&base).unwrap();
        assert_eq!(none, base);
        let o = Overrides {
            latency_ms: Some(80),
            jitter_ms: Some(10),
            loss_pct: Some(2.5),
            duplicate_pct: Some(1.0),
            reorder_pct: Some(3.0),
            bandwidth_kbytes_per_sec: Some(64),
            ..Overrides::default()
        };
        let link = o.apply(&base).unwrap();
        assert_eq!(
            (
                link.latency_us,
                link.jitter_us,
                link.loss_ppm,
                link.duplicate_ppm,
                link.reorder_ppm
            ),
            (80_000, 10_000, 25_000, 10_000, 30_000)
        );
        assert_eq!(link.bandwidth_bytes_per_sec, Some(64_000));
        assert!(
            link.reorder_delay_us > 0,
            "a reorder chance needs a delay to have any effect"
        );
        assert!(
            Overrides {
                loss_pct: Some(150.0),
                ..Overrides::default()
            }
            .apply(&base)
            .is_err()
        );
        assert!(
            Overrides {
                bandwidth_kbytes_per_sec: Some(0),
                ..Overrides::default()
            }
            .apply(&base)
            .is_err()
        );
        assert!(
            Overrides {
                latency_ms: Some(u64::MAX),
                ..Overrides::default()
            }
            .apply(&base)
            .is_err(),
            "no overflow"
        );
    }

    // ---- scenarios -----------------------------------------------------------------------------

    fn scenario(link: ss2_net::netsim::LinkConfig, seed: u64) -> Scenario {
        Scenario {
            link,
            seed,
            packets: 2_000,
            size_bytes: 100,
            interval_us: 16_667,
        }
    }

    #[test]
    fn a_perfect_link_delivers_everything_at_once_in_order() {
        let r = run_scenario(&scenario(preset("perfect").unwrap(), 1)).unwrap();
        assert_eq!(
            (r.sent, r.delivered_unique, r.lost, r.dropped_queue),
            (2_000, 2_000, 0, 0)
        );
        assert_eq!((r.duplicates_received, r.out_of_order), (0, 0));
        let l = r.latency.unwrap();
        assert_eq!((l.min_us, l.p50_us, l.max_us), (0, 0, 0));
    }

    #[test]
    fn a_fixed_latency_link_has_exactly_that_latency_everywhere() {
        let link = ss2_net::netsim::LinkConfig {
            latency_us: 50_000,
            ..Default::default()
        };
        let l = run_scenario(&scenario(link, 1)).unwrap().latency.unwrap();
        assert_eq!(
            (l.min_us, l.mean_us, l.p50_us, l.p95_us, l.p99_us, l.max_us),
            (50_000, 50_000, 50_000, 50_000, 50_000, 50_000)
        );
    }

    #[test]
    fn jitter_spreads_the_latency_within_its_bounds() {
        let link = ss2_net::netsim::LinkConfig {
            latency_us: 20_000,
            jitter_us: 10_000,
            ..Default::default()
        };
        let l = run_scenario(&scenario(link, 7)).unwrap().latency.unwrap();
        assert!(l.min_us >= 20_000 && l.max_us <= 30_000);
        assert!(l.max_us - l.min_us > 5_000, "{l:?}");
        assert!(
            l.min_us <= l.p50_us
                && l.p50_us <= l.p95_us
                && l.p95_us <= l.p99_us
                && l.p99_us <= l.max_us
        );
        assert!(l.mean_us >= 22_000 && l.mean_us <= 28_000, "{}", l.mean_us);
    }

    #[test]
    fn loss_is_counted_and_everything_adds_up() {
        let r = run_scenario(&scenario(preset("acceptance_100ms_2pct").unwrap(), 3)).unwrap();
        assert!(r.lost > 10 && r.lost < 100, "about 2 % of 2000: {}", r.lost);
        assert_eq!(r.delivered_unique + r.lost + r.dropped_queue, r.sent);
        assert!((r.loss_percent() - 100.0 * r.lost as f64 / 2_000.0).abs() < 1e-9);
    }

    #[test]
    fn total_loss_delivers_nothing_and_has_no_latency() {
        let link = ss2_net::netsim::LinkConfig {
            loss_ppm: 1_000_000,
            ..Default::default()
        };
        let r = run_scenario(&scenario(link, 1)).unwrap();
        assert_eq!((r.delivered_unique, r.lost), (0, 2_000));
        assert!(r.latency.is_none());
        assert!(r.render().contains("no packet arrived"), "{}", r.render());
    }

    #[test]
    fn duplicates_and_reordering_are_seen_by_the_receiver() {
        let link = ss2_net::netsim::LinkConfig {
            latency_us: 10_000,
            duplicate_ppm: 100_000,
            reorder_ppm: 200_000,
            reorder_delay_us: 100_000,
            ..Default::default()
        };
        let r = run_scenario(&scenario(link, 5)).unwrap();
        assert!(r.duplicates_received > 100, "{}", r.duplicates_received);
        assert!(r.out_of_order > 100, "{}", r.out_of_order);
        assert_eq!(r.delivered_unique, 2_000, "nothing is lost here");
    }

    #[test]
    fn a_narrow_link_drops_the_tail_when_the_queue_is_full() {
        // 100-byte packets every 1 ms is 100 kB/s; the link carries 20 kB/s with room for 1 kB.
        let link = ss2_net::netsim::LinkConfig {
            bandwidth_bytes_per_sec: Some(20_000),
            queue_limit_bytes: 1_000,
            ..Default::default()
        };
        let s = Scenario {
            link,
            seed: 1,
            packets: 1_000,
            size_bytes: 100,
            interval_us: 1_000,
        };
        let r = run_scenario(&s).unwrap();
        assert!(r.dropped_queue > 500, "{}", r.dropped_queue);
        assert_eq!(r.delivered_unique + r.lost + r.dropped_queue, r.sent);
        let l = r.latency.unwrap();
        assert!(
            l.max_us > 5_000,
            "queueing delay shows in the latency: {l:?}"
        );
    }

    #[test]
    fn the_same_seed_gives_the_same_report_and_another_seed_does_not() {
        let link = preset("bad_wifi").unwrap();
        let a = run_scenario(&scenario(link.clone(), 11)).unwrap();
        let b = run_scenario(&scenario(link.clone(), 11)).unwrap();
        let c = run_scenario(&scenario(link, 12)).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.render(), b.render());
        assert_ne!(a, c);
    }

    #[test]
    fn a_scenario_with_nothing_to_send_or_too_much_is_refused() {
        let ok = scenario(preset("lan").unwrap(), 1);
        assert!(
            run_scenario(&Scenario {
                packets: 0,
                ..ok.clone()
            })
            .is_err()
        );
        assert!(
            run_scenario(&Scenario {
                size_bytes: 0,
                ..ok.clone()
            })
            .is_err()
        );
        assert!(
            run_scenario(&Scenario {
                packets: MAX_PACKETS + 1,
                ..ok.clone()
            })
            .is_err()
        );
        let bad_link = ss2_net::netsim::LinkConfig {
            loss_ppm: 2_000_000,
            ..Default::default()
        };
        assert!(
            run_scenario(&Scenario {
                link: bad_link,
                ..ok.clone()
            })
            .is_err()
        );
        assert!(
            run_scenario(&Scenario {
                interval_us: 0,
                ..ok
            })
            .is_ok(),
            "a burst is allowed"
        );
    }

    #[test]
    fn the_report_text_shows_the_numbers_a_person_looks_for() {
        let text = run_scenario(&scenario(preset("mobile").unwrap(), 2))
            .unwrap()
            .render();
        for want in [
            "sent: 2000",
            "received:",
            "lost on the wire:",
            "duplicates received:",
            "out of order:",
            "latency (ms):",
            "p95",
            "p99",
        ] {
            assert!(text.contains(want), "missing {want:?} in:\n{text}");
        }
    }
}
