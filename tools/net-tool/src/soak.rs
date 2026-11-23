//! Soak test with **headless bots** (roadmap T08-029): N scripted clients and one server run for M minutes of **virtual
//! time** over a conditioned link (`ss2_net::netsim`), using the real session layer (`ss2_net::session`) and the real input stream
//! (`ss2_net::inputstream`), and the run **checks invariants** instead of printing impressions.
//!
//! What is simulated: every bot joins the server (staggered), then sends one input per tick for a tick a little ahead of the
//! server's, with the redundant packets of `InputSender`; the server validates, buffers and takes one input per bot per
//! tick and sends back acknowledgements; session keepalives and timeouts run as they would. Optionally bots **leave and rejoin**
//! ("churn"). The "game" is a stand-in: the input is a number and the state is whatever the server applied. There is no
//! simulation, no snapshots and no prediction in this repository yet, so this soaks **the protocol modules**, not the game.
//!
//! What a run **proves** (see [`SoakReport::problems`]): every bot is in the session at the end and the server agrees; nobody was
//! dropped for silence who had not asked to leave; **every input the server applied as received is exactly the input the bot sent
//! for that tick** (no loss of identity, no duplicate or reordered application); a well-behaved client never earns a
//! `conflicting`, `too_far_ahead` or `rate_limited` count; and the share of ticks for which the server had no input stays under a
//! limit. What it only **reports**: bandwidth (by a stated size model, not real encoded bytes: the input and ack messages have no
//! wire encoding yet) and the late, duplicate and repeated counts.
//!
//! Deterministic: the same configuration and seed give the same report.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use ss2_net::inputstream::{InputBuffer, InputConfig, InputPacket, InputSender, InputTake};
use ss2_net::netsim::{Duplex, LinkConfig};
use ss2_net::protocol::{Handshake, ProtocolVersion};
use ss2_net::session::{
    ClientConfig, ClientEvent, ClientState, Message, PeerId, ServerConfig, ServerEvent,
    SessionClient, SessionServer,
};

/// The most bots a run may have.
pub const MAX_BOTS: u16 = 64;
/// The longest run, in minutes of virtual time.
pub const MAX_MINUTES: u32 = 24 * 60;

/// What to run.
#[derive(Clone, Debug, PartialEq)]
pub struct SoakConfig {
    /// How many bots, 1 to [`MAX_BOTS`].
    pub bots: u16,
    /// Minutes of virtual time, 1 to [`MAX_MINUTES`].
    pub minutes: u32,
    /// Simulation ticks per second (the server tells the bots).
    pub tick_hz: u32,
    /// What the link does, in both directions (each with its own random draws).
    pub link: LinkConfig,
    /// The random seed.
    pub seed: u64,
    /// How many ticks ahead of its estimate of the server's tick a bot stamps its input. A bot learns the server's tick from the welcome
    /// (there is no clock synchronisation here), so its estimate is a round trip behind: the lead must cover the **round trip, the jitter and one more tick**,
    /// or the inputs arrive late.
    pub lead_ticks: u64,
    /// The server's player slots (at least `bots` for everyone to fit).
    pub max_players: u8,
    /// Every this many seconds one bot (in turn) leaves and rejoins; 0 for no churn.
    pub churn_every_seconds: u32,
    /// The share of ticks (after joining and a short warm-up) the server may be without an input before the run counts as a failure.
    pub max_missing_fraction: f64,
    /// Whether input packets and acknowledgements count as proof of life (`SessionServer::heard_from`, `SessionClient::heard_from_server`). With
    /// `false` only session messages do, as before those methods existed: a lossy link then drops players whose game traffic is flowing.
    pub traffic_is_liveness: bool,
    /// **Fault injection, to prove that the checker notices:** every this many ticks a bot sends a different number than the one it recorded
    /// (0: never). A correct run of the server never applies such an input as "received" without the check reporting a mismatch.
    pub corrupt_every: u32,
}

impl Default for SoakConfig {
    /// 8 bots for 5 minutes at 30 Hz over a perfect link, a lead of 6 ticks, no churn, up to 20 % of ticks without input, game traffic counting as liveness.
    fn default() -> Self {
        Self {
            bots: 8,
            minutes: 5,
            tick_hz: 30,
            link: LinkConfig::default(),
            seed: 1,
            lead_ticks: 6,
            max_players: 64,
            churn_every_seconds: 0,
            max_missing_fraction: 0.2,
            traffic_is_liveness: true,
            corrupt_every: 0,
        }
    }
}

/// What the server saw of one run.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SoakReport {
    /// Bots that were in the session at the end.
    pub connected_at_end: u32,
    /// Players the server had at the end.
    pub server_connected_at_end: u32,
    /// Joins that were welcomed (a rejoin counts again).
    pub joins: u32,
    /// Bots that never got in (refused, or timed out while joining) and were not retried.
    pub failed_joins: u32,
    /// Connections that ended in a timeout or a refusal rather than because a bot left.
    pub false_disconnects: u32,
    /// Times a bot left and rejoined on purpose.
    pub churn_leaves: u32,
    /// Ticks simulated.
    pub ticks: u64,
    /// Inputs the server applied that arrived for their tick.
    pub applied_received: u64,
    /// Ticks for which the last input was repeated.
    pub applied_repeated: u64,
    /// Ticks with no input at all, after the warm-up of each join.
    pub missing: u64,
    /// Inputs applied as received that were **not** what the bot sent for that tick (must be 0).
    pub mismatched: u64,
    /// Frames whose tick had already been simulated: mostly the redundant repeats of frames that did arrive, but also real late arrivals.
    pub too_old: u64,
    /// Frames already held (the redundancy at work).
    pub duplicates: u64,
    /// A different input for a tick that already had one (must be 0).
    pub conflicting: u64,
    /// Frames too far ahead (must be 0).
    pub too_far_ahead: u64,
    /// Frames refused by the rate limit (must be 0).
    pub rate_limited: u64,
    /// Bytes bots sent to the server, by the size model.
    pub bytes_up: u64,
    /// Bytes the server sent to bots, by the size model.
    pub bytes_down: u64,
    /// The hard limit on the share of ticks without an input, from the configuration.
    pub max_missing_fraction: f64,
    /// Seconds of virtual time that bots were connected, summed over bots.
    pub connected_seconds: f64,
}

impl SoakReport {
    /// Ticks the server expected an input for (applied or not).
    pub fn counted_ticks(&self) -> u64 {
        self.applied_received + self.applied_repeated + self.missing
    }

    /// The share of those ticks that had no input at all.
    pub fn missing_fraction(&self) -> f64 {
        if self.counted_ticks() == 0 {
            0.0
        } else {
            self.missing as f64 / self.counted_ticks() as f64
        }
    }

    /// Average upstream rate of one bot, kilobits per second (0 if nobody was connected).
    pub fn up_kbit_per_bot(&self) -> f64 {
        if self.connected_seconds <= 0.0 {
            0.0
        } else {
            self.bytes_up as f64 * 8.0 / 1000.0 / self.connected_seconds
        }
    }

    /// Average downstream rate to one bot, kilobits per second.
    pub fn down_kbit_per_bot(&self) -> f64 {
        if self.connected_seconds <= 0.0 {
            0.0
        } else {
            self.bytes_down as f64 * 8.0 / 1000.0 / self.connected_seconds
        }
    }

    /// What went wrong, in words; empty when the run passed.
    pub fn problems(&self, bots: u32) -> Vec<String> {
        let mut out = Vec::new();
        if self.connected_at_end != bots {
            out.push(format!(
                "{} of {bots} bots are in the session at the end",
                self.connected_at_end
            ));
        }
        if self.server_connected_at_end != self.connected_at_end {
            out.push(format!(
                "the server has {} players but {} bots think they are connected",
                self.server_connected_at_end, self.connected_at_end
            ));
        }
        if self.failed_joins > 0 {
            out.push(format!("{} bots never got in", self.failed_joins));
        }
        if self.false_disconnects > 0 {
            out.push(format!(
                "{} connections were lost to a timeout or a refusal",
                self.false_disconnects
            ));
        }
        if self.mismatched > 0 {
            out.push(format!(
                "{} inputs were applied that are not what the bot sent for that tick",
                self.mismatched
            ));
        }
        for (name, n) in [
            ("conflicting", self.conflicting),
            ("too far ahead", self.too_far_ahead),
            ("rate-limited", self.rate_limited),
        ] {
            if n > 0 {
                out.push(format!("a well-behaved client earned {n} {name} frames"));
            }
        }
        if self.missing_fraction() > self.max_missing_fraction {
            out.push(format!(
                "{:.1} % of ticks had no input (limit {:.1} %)",
                self.missing_fraction() * 100.0,
                self.max_missing_fraction * 100.0
            ));
        }
        out
    }

    /// The report as text.
    pub fn render(&self, bots: u32) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "ticks: {}  bots in the session at the end: {}/{}  (server: {})",
            self.ticks, self.connected_at_end, bots, self.server_connected_at_end
        );
        let _ = writeln!(
            out,
            "joins: {}  failed joins: {}  false disconnects: {}  churn leaves: {}",
            self.joins, self.failed_joins, self.false_disconnects, self.churn_leaves
        );
        let _ = writeln!(
            out,
            "inputs: {} applied as received, {} repeated, {} missing ({:.2} % of {} ticks); mismatched: {}",
            self.applied_received,
            self.applied_repeated,
            self.missing,
            self.missing_fraction() * 100.0,
            self.counted_ticks(),
            self.mismatched
        );
        let _ = writeln!(
            out,
            "frames: {} stale (already simulated: redundant repeats and late arrivals), {} duplicate (redundancy), {} conflicting, {} too far ahead, {} rate-limited",
            self.too_old, self.duplicates, self.conflicting, self.too_far_ahead, self.rate_limited
        );
        let _ = writeln!(
            out,
            "bandwidth per bot (size model, not real bytes): {:.1} kbit/s up, {:.1} kbit/s down",
            self.up_kbit_per_bot(),
            self.down_kbit_per_bot()
        );
        out
    }
}

/// What travels between a bot and the server.
#[derive(Clone, Debug)]
enum Payload {
    Session(Message),
    Input(InputPacket<u32>),
    Ack(u64),
}

/// The size model: bytes on the wire for a payload (input and ack have no wire encoding yet).
fn size_of(payload: &Payload) -> usize {
    match payload {
        Payload::Session(Message::Hello {
            name,
            password,
            token,
            ..
        }) => 48 + name.len() + password.len() + token.len(),
        Payload::Session(_) => 24,
        Payload::Input(p) => 4 + 12 * p.frames.len(),
        Payload::Ack(_) => 12,
    }
}

/// The input a bot sends for a tick: deterministic, never zero.
fn input_value(bot: usize, tick: u64) -> u32 {
    let mut h =
        (bot as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ tick.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 29;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    ((h >> 32) as u32) | 1
}

struct Bot {
    client: SessionClient,
    link: Duplex<Payload>,
    sender: InputSender<u32>,
    sent: BTreeMap<u64, u32>,
    /// The server tick the welcome named and the tick of this run at which it arrived.
    admitted: Option<(u64, u64)>,
    last_pushed: Option<u64>,
    /// The tick at which a bot that left starts again.
    restart_at: Option<u64>,
    started: bool,
    connected_since_us: Option<u64>,
    /// Ticks to skip counting after a join (the inputs have not reached the server yet).
    warmup_until: u64,
}

fn handshake() -> Handshake {
    Handshake {
        version: ProtocolVersion::new(0, 1),
        schema_hash: 0x50AC,
    }
}

/// Run a soak. `Err` for a configuration that makes no sense.
pub fn run_soak(config: &SoakConfig) -> Result<SoakReport, String> {
    if config.bots == 0 || config.bots > MAX_BOTS {
        return Err(format!("bots must be 1 to {MAX_BOTS}"));
    }
    if config.minutes == 0 || config.minutes > MAX_MINUTES {
        return Err(format!("minutes must be 1 to {MAX_MINUTES}"));
    }
    if config.tick_hz == 0 || config.tick_hz > 240 {
        return Err("the tick rate must be 1 to 240 Hz".into());
    }
    if config.lead_ticks == 0 || config.lead_ticks > 25 {
        return Err(
            "the lead must be 1 to 25 ticks (the server accepts input at most 30 ticks ahead)"
                .into(),
        );
    }
    if !(0.0..=1.0).contains(&config.max_missing_fraction) {
        return Err("the missing fraction limit must be between 0 and 1".into());
    }
    config.link.validate()?;
    let hz = u64::from(config.tick_hz);
    let total_ticks = u64::from(config.minutes) * 60 * hz;
    let tick_us = |t: u64| t * 1_000_000 / hz;

    let mut server_cfg = ServerConfig::new(handshake());
    server_cfg.max_players = config.max_players;
    server_cfg.tick_hz = config.tick_hz;
    let mut server = SessionServer::new(server_cfg);
    let input_cfg = InputConfig::default();
    let mut buffers: BTreeMap<usize, InputBuffer<u32>> = BTreeMap::new();

    let new_client =
        |i: usize| SessionClient::new(ClientConfig::new(handshake(), &format!("bot{i}")));
    let mut bots: Vec<Bot> = (0..usize::from(config.bots))
        .map(|i| Bot {
            client: new_client(i),
            link: Duplex::symmetric(
                config.link.clone(),
                config.seed.wrapping_add(i as u64 * 0x1000_0001),
            ),
            sender: InputSender::new(input_cfg.clone()),
            sent: BTreeMap::new(),
            admitted: None,
            last_pushed: None,
            restart_at: Some(i as u64 * 3),
            started: false,
            connected_since_us: None,
            warmup_until: 0,
        })
        .collect();

    let mut r = SoakReport {
        ticks: total_ticks,
        max_missing_fraction: config.max_missing_fraction,
        ..SoakReport::default()
    };
    let churn_every = u64::from(config.churn_every_seconds) * hz;
    let mut quitters: Vec<bool> = vec![false; bots.len()];

    for t in 0..total_ticks {
        let now = tick_us(t);

        // ---- the bots ----
        for (i, bot) in bots.iter_mut().enumerate() {
            if bot.restart_at.is_some_and(|at| t >= at) {
                bot.restart_at = None;
                bot.started = true;
                bot.client = new_client(i);
                bot.sender = InputSender::new(input_cfg.clone());
                bot.sent.clear();
                bot.admitted = None;
                bot.last_pushed = None;
                quitters[i] = false;
                for m in bot.client.start(now) {
                    r.bytes_up += send_up(bot, now, Payload::Session(m));
                }
            }
            if !bot.started {
                continue;
            }
            // Churn: one bot at a time leaves, to come back two seconds later.
            if churn_every > 0
                && t > 0
                && t % churn_every == 0
                && (t / churn_every) as usize % usize::from(config.bots) == i
                && matches!(bot.client.state(), ClientState::Connected { .. })
            {
                for m in bot.client.disconnect() {
                    r.bytes_up += send_up(bot, now, Payload::Session(m));
                }
                quitters[i] = true;
                bot.started = false;
                bot.restart_at = Some(t + 2 * hz);
                if let Some(since) = bot.connected_since_us.take() {
                    r.connected_seconds += (now - since) as f64 / 1e6;
                }
                bot.admitted = None;
                r.churn_leaves += 1;
                continue;
            }
            for m in bot.client.tick(now) {
                r.bytes_up += send_up(bot, now, Payload::Session(m));
            }
            // What the server sent.
            for (_, payload) in bot.link.b_to_a.poll(now) {
                match payload {
                    Payload::Session(m) => {
                        for reply in bot.client.handle(m, now) {
                            r.bytes_up += send_up(bot, now, Payload::Session(reply));
                        }
                    }
                    Payload::Ack(tick) => {
                        if config.traffic_is_liveness {
                            bot.client.heard_from_server(now);
                        }
                        bot.sender.ack(tick);
                    }
                    Payload::Input(_) => {}
                }
            }
            for event in bot.client.take_events() {
                match event {
                    ClientEvent::Connected { server_tick, .. } => {
                        bot.admitted = Some((server_tick, t));
                        bot.connected_since_us = Some(now);
                        bot.warmup_until = t + config.lead_ticks + 4;
                        r.joins += 1;
                    }
                    ClientEvent::TimedOut | ClientEvent::Refused(_) => {
                        if !quitters[i] {
                            if bot.admitted.is_some() {
                                r.false_disconnects += 1;
                            } else {
                                r.failed_joins += 1;
                            }
                        }
                        if let Some(since) = bot.connected_since_us.take() {
                            r.connected_seconds += (now - since) as f64 / 1e6;
                        }
                        bot.admitted = None;
                        bot.started = false; // a bot that is refused or times out does not try again
                    }
                    _ => {}
                }
            }
            // One input per tick, for a tick a little ahead of the server's.
            if let (Some((server_tick, admitted_t)), ClientState::Connected { .. }) =
                (bot.admitted, bot.client.state().clone())
            {
                let estimate = server_tick + (t - admitted_t);
                let target = estimate + config.lead_ticks;
                if bot.last_pushed.is_none_or(|last| target > last) {
                    let value = input_value(i, target);
                    let wire = if config.corrupt_every > 0
                        && target.is_multiple_of(u64::from(config.corrupt_every))
                    {
                        value ^ 0xFFFF
                    } else {
                        value
                    };
                    if bot.sender.push(target, wire) {
                        bot.sent.insert(target, value);
                        bot.last_pushed = Some(target);
                        while bot.sent.len() > 256 {
                            bot.sent.pop_first();
                        }
                    }
                }
                if let Some(packet) = bot.sender.packet() {
                    r.bytes_up += send_up(bot, now, Payload::Input(packet));
                }
            }
        }

        // ---- the server ----
        for (i, bot) in bots.iter_mut().enumerate() {
            for (_, payload) in bot.link.a_to_b.poll(now) {
                match payload {
                    Payload::Session(m) => {
                        for out in server.handle(PeerId(i as u64), m, now) {
                            r.bytes_down += send_down(bot, now, out.message);
                        }
                    }
                    Payload::Input(packet) => {
                        if config.traffic_is_liveness {
                            server.heard_from(PeerId(i as u64), now);
                        }
                        if let Some(buffer) = buffers.get_mut(&i) {
                            let report = buffer.receive(&packet);
                            r.too_old += u64::from(report.too_old);
                            r.duplicates += u64::from(report.duplicates);
                            r.conflicting += u64::from(report.conflicting);
                            r.too_far_ahead += u64::from(report.too_far_ahead);
                            r.rate_limited += u64::from(report.rate_limited);
                        }
                    }
                    Payload::Ack(_) => {}
                }
            }
        }
        for event in server.take_events() {
            match event {
                ServerEvent::Joined { peer, .. } => {
                    buffers.insert(peer.0 as usize, InputBuffer::new(input_cfg.clone(), t));
                }
                ServerEvent::Left { peer, .. } => {
                    // A timeout is counted from the bot's side when it notices; the final check compares both views.
                    buffers.remove(&(peer.0 as usize));
                }
            }
        }
        for out in server.tick(now, t) {
            let i = out.to.0 as usize;
            r.bytes_down += send_down(&mut bots[i], now, out.message);
        }
        for (&i, buffer) in &mut buffers {
            let bot = &mut bots[i];
            let counted = bot.admitted.is_some() && t >= bot.warmup_until;
            match buffer.take(t) {
                InputTake::Received(v) => {
                    if bot.sent.get(&t) == Some(&v) {
                        r.applied_received += u64::from(counted);
                    } else {
                        r.mismatched += 1;
                    }
                }
                InputTake::Repeated(_) => r.applied_repeated += u64::from(counted),
                InputTake::Missing => r.missing += u64::from(counted),
            }
            if let Some(ack) = buffer.ack_tick() {
                let size = size_of(&Payload::Ack(ack)) as u64;
                bot.link.b_to_a.send(now, Payload::Ack(ack), size as usize);
                r.bytes_down += size;
            }
        }
    }

    let end = tick_us(total_ticks);
    for bot in &mut bots {
        if let Some(since) = bot.connected_since_us.take() {
            r.connected_seconds += (end - since) as f64 / 1e6;
        }
    }
    r.connected_at_end = bots
        .iter()
        .filter(|b| matches!(b.client.state(), ClientState::Connected { .. }))
        .count() as u32;
    r.server_connected_at_end = server.connected() as u32;
    Ok(r)
}

fn send_up(bot: &mut Bot, now: u64, payload: Payload) -> u64 {
    let size = size_of(&payload);
    bot.link.a_to_b.send(now, payload, size);
    size as u64
}

fn send_down(bot: &mut Bot, now: u64, message: Message) -> u64 {
    let payload = Payload::Session(message);
    let size = size_of(&payload);
    bot.link.b_to_a.send(now, payload, size);
    size as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quick(link: LinkConfig) -> SoakConfig {
        SoakConfig {
            bots: 4,
            minutes: 1,
            link,
            ..SoakConfig::default()
        }
    }

    #[test]
    fn a_perfect_link_gives_a_clean_run() {
        let r = run_soak(&quick(LinkConfig::default())).unwrap();
        assert_eq!(r.problems(4), Vec::<String>::new(), "{}", r.render(4));
        assert_eq!(
            (
                r.connected_at_end,
                r.server_connected_at_end,
                r.joins,
                r.failed_joins
            ),
            (4, 4, 4, 0)
        );
        assert_eq!(
            (
                r.missing,
                r.mismatched,
                r.conflicting,
                r.too_far_ahead,
                r.rate_limited,
                r.false_disconnects
            ),
            (0, 0, 0, 0, 0, 0)
        );
        assert!(
            r.applied_received > 4 * 60 * 30 * 9 / 10,
            "almost every tick of every bot: {}",
            r.applied_received
        );
        assert_eq!(
            r.counted_ticks(),
            r.applied_received + r.applied_repeated + r.missing
        );
        assert!(
            r.duplicates > 0,
            "the redundant frames arrive twice or more"
        );
        assert!(
            r.bytes_up > 0
                && r.bytes_down > 0
                && r.up_kbit_per_bot() > 0.0
                && r.down_kbit_per_bot() > 0.0
        );
    }

    #[test]
    fn the_bandwidth_follows_the_size_model() {
        // On a perfect link an acknowledgement comes back at once, so each packet carries one frame: 4 + 12 = 16 bytes at 30 Hz, plus a
        // keepalive every two seconds; the downstream is a 12-byte acknowledgement per tick plus keepalives.
        let r = run_soak(&quick(LinkConfig::default())).unwrap();
        assert!((3.5..4.5).contains(&r.up_kbit_per_bot()), "{}", r.render(4));
        assert!(
            (2.7..3.3).contains(&r.down_kbit_per_bot()),
            "{}",
            r.render(4)
        );
        // With a round trip the frames wait for their acknowledgement and each packet carries more.
        let slow = run_soak(&SoakConfig {
            lead_ticks: 12,
            ..quick(LinkConfig {
                latency_us: 100_000,
                ..LinkConfig::default()
            })
        })
        .unwrap();
        assert!(
            slow.up_kbit_per_bot() > r.up_kbit_per_bot() * 1.5,
            "{} vs {}",
            slow.up_kbit_per_bot(),
            r.up_kbit_per_bot()
        );
    }

    #[test]
    fn the_checker_notices_an_input_that_is_not_what_the_bot_recorded() {
        let r = run_soak(&SoakConfig {
            corrupt_every: 10,
            ..quick(LinkConfig::default())
        })
        .unwrap();
        assert!(r.mismatched > 100, "{}", r.render(4));
        assert!(
            r.problems(4)
                .iter()
                .any(|p| p.contains("not what the bot sent")),
            "{:?}",
            r.problems(4)
        );
        let clean = run_soak(&quick(LinkConfig::default())).unwrap();
        assert_eq!(clean.mismatched, 0);
    }

    #[test]
    fn an_ordinary_internet_link_is_fine() {
        let r = run_soak(&quick(ss2_net::netsim::LinkConfig::broadband())).unwrap();
        assert_eq!(r.problems(4), Vec::<String>::new(), "{}", r.render(4));
        assert!(r.missing_fraction() < 0.05, "{}", r.render(4));
    }

    #[test]
    fn redundancy_absorbs_loss_and_a_wrong_input_is_never_applied() {
        // 60 ms each way with 20 ms of jitter and 10 % loss, with a lead that covers the round trip and the jitter.
        let link = LinkConfig {
            latency_us: 60_000,
            jitter_us: 20_000,
            loss_ppm: 100_000,
            ..LinkConfig::default()
        };
        let r = run_soak(&SoakConfig {
            lead_ticks: 12,
            ..quick(link.clone())
        })
        .unwrap();
        assert_eq!(r.problems(4), Vec::<String>::new(), "{}", r.render(4));
        assert_eq!(
            (r.mismatched, r.conflicting, r.too_far_ahead, r.rate_limited),
            (0, 0, 0, 0)
        );
        assert!(
            r.duplicates > 1_000,
            "redundant frames arrive in more than one packet: {}",
            r.render(4)
        );
        assert!(
            r.missing_fraction() < 0.01,
            "three-fold redundancy against 10 % loss: {}",
            r.render(4)
        );
        // The same link with a lead of 6 ticks is too tight: inputs arrive late and the server repeats or does without.
        let tight = run_soak(&quick(link)).unwrap();
        assert!(
            tight.applied_repeated + tight.missing > r.applied_repeated + r.missing,
            "{}",
            tight.render(4)
        );
        assert_eq!(
            tight.mismatched,
            0,
            "late is not wrong: {}",
            tight.render(4)
        );
    }

    #[test]
    fn the_lead_must_cover_a_round_trip_because_the_bot_learns_the_servers_tick_from_the_welcome() {
        // Without clock synchronisation (ss2_net::timesync) a bot's idea of the server's tick is one round trip behind, so an input
        // stamped `lead` ticks ahead of it has only `lead - round trip - 1` ticks to arrive. 300 ms each way is 9 ticks at 30 Hz.
        let link = || LinkConfig {
            latency_us: 300_000,
            ..LinkConfig::default()
        };
        let short = run_soak(&quick(link())).unwrap();
        assert!(
            short.problems(4).iter().any(|p| p.contains("had no input")),
            "{}",
            short.render(4)
        );
        let long = run_soak(&SoakConfig {
            lead_ticks: 24,
            ..quick(link())
        })
        .unwrap();
        assert_eq!(long.problems(4), Vec::<String>::new(), "{}", long.render(4));
    }

    #[test]
    fn a_lossy_link_drops_players_whose_game_traffic_flows_unless_that_traffic_counts_as_life() {
        // 30 % loss each way: 2 s keepalives against a 10 s timeout, so five in a row must be lost for a drop; over half an hour that happens.
        let link = || LinkConfig {
            latency_us: 40_000,
            loss_ppm: 300_000,
            ..LinkConfig::default()
        };
        let old = run_soak(&SoakConfig {
            minutes: 30,
            traffic_is_liveness: false,
            ..quick(link())
        })
        .unwrap();
        assert!(old.false_disconnects > 0, "{}", old.render(4));
        assert!(
            old.problems(4).iter().any(|p| p.contains("timeout")),
            "{:?}",
            old.problems(4)
        );
        let fixed = run_soak(&SoakConfig {
            minutes: 30,
            ..quick(link())
        })
        .unwrap();
        assert_eq!(
            (fixed.false_disconnects, fixed.failed_joins),
            (0, 0),
            "{}",
            fixed.render(4)
        );
        assert_eq!(fixed.connected_at_end, 4);
        assert_eq!(fixed.mismatched, 0);
    }

    #[test]
    fn a_link_that_drops_most_packets_still_fails_the_run_and_says_so() {
        let link = LinkConfig {
            latency_us: 40_000,
            loss_ppm: 900_000,
            ..LinkConfig::default()
        }; // 90 % loss
        let r = run_soak(&SoakConfig {
            minutes: 2,
            ..quick(link)
        })
        .unwrap();
        assert!(!r.problems(4).is_empty(), "{}", r.render(4));
    }

    #[test]
    fn the_same_seed_gives_the_same_report_and_another_seed_does_not() {
        let link = LinkConfig {
            latency_us: 50_000,
            jitter_us: 30_000,
            loss_ppm: 50_000,
            reorder_ppm: 30_000,
            reorder_delay_us: 40_000,
            ..LinkConfig::default()
        };
        let a = run_soak(&quick(link.clone())).unwrap();
        let b = run_soak(&quick(link.clone())).unwrap();
        let c = run_soak(&SoakConfig {
            seed: 2,
            ..quick(link)
        })
        .unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn bots_that_leave_and_rejoin_free_their_slots_and_the_server_agrees() {
        let cfg = SoakConfig {
            bots: 4,
            minutes: 2,
            churn_every_seconds: 10,
            max_players: 4,
            ..SoakConfig::default()
        };
        let r = run_soak(&cfg).unwrap();
        assert_eq!(r.problems(4), Vec::<String>::new(), "{}", r.render(4));
        assert!(r.churn_leaves >= 10, "{}", r.render(4));
        assert_eq!(
            r.joins,
            4 + r.churn_leaves,
            "every leave is followed by a join: {}",
            r.render(4)
        );
        assert_eq!((r.connected_at_end, r.server_connected_at_end), (4, 4));
    }

    #[test]
    fn more_bots_than_slots_leaves_some_out_and_the_run_says_so() {
        let cfg = SoakConfig {
            bots: 5,
            minutes: 1,
            max_players: 4,
            ..SoakConfig::default()
        };
        let r = run_soak(&cfg).unwrap();
        assert_eq!(r.connected_at_end, 4, "{}", r.render(5));
        assert_eq!(r.failed_joins, 1);
        assert!(r.problems(5).iter().any(|p| p.contains("never got in")));
    }

    #[test]
    fn nonsense_configurations_are_refused() {
        let ok = SoakConfig::default();
        for bad in [
            SoakConfig {
                bots: 0,
                ..ok.clone()
            },
            SoakConfig {
                bots: MAX_BOTS + 1,
                ..ok.clone()
            },
            SoakConfig {
                minutes: 0,
                ..ok.clone()
            },
            SoakConfig {
                minutes: MAX_MINUTES + 1,
                ..ok.clone()
            },
            SoakConfig {
                tick_hz: 0,
                ..ok.clone()
            },
            SoakConfig {
                tick_hz: 241,
                ..ok.clone()
            },
            SoakConfig {
                lead_ticks: 0,
                ..ok.clone()
            },
            SoakConfig {
                lead_ticks: 26,
                ..ok.clone()
            },
            SoakConfig {
                max_missing_fraction: 1.5,
                ..ok.clone()
            },
            SoakConfig {
                max_missing_fraction: f64::NAN,
                ..ok.clone()
            },
            SoakConfig {
                link: LinkConfig {
                    loss_ppm: 2_000_000,
                    ..LinkConfig::default()
                },
                ..ok.clone()
            },
        ] {
            assert!(run_soak(&bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn the_input_values_are_never_zero_and_differ_by_bot_and_tick() {
        let mut seen = std::collections::BTreeSet::new();
        for bot in 0..8 {
            for tick in 0..200 {
                let v = input_value(bot, tick);
                assert_ne!(v, 0);
                seen.insert(v);
            }
        }
        assert!(seen.len() > 1590, "{} distinct values of 1600", seen.len());
    }

    #[test]
    fn the_report_text_and_the_problem_list_name_what_matters() {
        let r = run_soak(&quick(LinkConfig::default())).unwrap();
        let text = r.render(4);
        for want in [
            "ticks:",
            "bots in the session at the end: 4/4",
            "joins: 4",
            "applied as received",
            "mismatched: 0",
            "stale",
            "kbit/s up",
        ] {
            assert!(text.contains(want), "missing {want:?} in\n{text}");
        }
        let bad = SoakReport {
            connected_at_end: 2,
            server_connected_at_end: 3,
            failed_joins: 1,
            false_disconnects: 2,
            mismatched: 5,
            conflicting: 1,
            too_far_ahead: 1,
            rate_limited: 1,
            applied_received: 10,
            missing: 90,
            max_missing_fraction: 0.2,
            ..SoakReport::default()
        };
        let problems = bad.problems(4).join("\n");
        for want in [
            "2 of 4 bots",
            "server has 3 players",
            "never got in",
            "timeout or a refusal",
            "not what the bot sent",
            "conflicting",
            "too far ahead",
            "rate-limited",
            "90.0 % of ticks had no input",
        ] {
            assert!(problems.contains(want), "missing {want:?} in\n{problems}");
        }
    }
}
