//! Sound decoders (roadmap T06-002): the RIFF `.wav` files (PCM 8, 16, 24 and 32 bit, IEEE float 32 and 64 bit, and IMA ADPCM, also in the
//! `WAVE_FORMAT_EXTENSIBLE` form) and Ogg Vorbis, into interleaved `f32` samples, and the rule for what is decoded whole and what is streamed.
//!
//! Written from the public RIFF/WAVE, IMA ADPCM and Vorbis descriptions, not from any game: the formats of the game's own sounds are in
//! `docs/re/audio.md`, and every one of them is a case here. The Vorbis decoding is the `lewton` crate (pure Rust, safe).
//!
//! A file from outside (a mod, a damaged archive) must never crash the program or allocate without limit: every size is checked against what
//! the file actually holds, the channel count and the length are limited ([`MAX_CHANNELS`], [`MAX_FRAMES`]), and a panic inside the Vorbis
//! decoder is caught and becomes an error. See `docs/AUDIO_DECODERS.md`.

use std::io::Cursor;
use std::panic::{AssertUnwindSafe, catch_unwind};

use lewton::inside_ogg::OggStreamReader;
use lewton::samples::InterleavedSamples;
use ss2_core::{Error, Result};

/// The most channels a decoded sound may have (7.1).
pub const MAX_CHANNELS: u16 = 8;
/// The most frames a decoded sound may have: half an hour at 44.1 kHz. A longer one is an error, not a gigabyte of memory.
pub const MAX_FRAMES: u64 = 44_100 * 60 * 30;
/// The highest sample rate accepted.
pub const MAX_RATE: u32 = 384_000;

/// A fully decoded sound: interleaved samples, nominally -1 to 1.
#[derive(Clone, Debug, PartialEq)]
pub struct Pcm {
    /// Frames per second.
    pub rate: u32,
    /// Samples per frame.
    pub channels: u16,
    /// `frames * channels` samples, interleaved.
    pub samples: Vec<f32>,
}

impl Pcm {
    /// The number of frames.
    pub fn frames(&self) -> usize {
        self.samples.len() / usize::from(self.channels.max(1))
    }

    /// The length in seconds.
    pub fn seconds(&self) -> f64 {
        self.frames() as f64 / f64::from(self.rate.max(1))
    }
}

/// The file format around the audio.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Container {
    /// RIFF/WAVE.
    Wav,
    /// Ogg.
    Ogg,
}

/// How the samples are coded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Codec {
    /// Unsigned 8-bit PCM.
    Pcm8,
    /// Signed 16-bit PCM.
    Pcm16,
    /// Signed 24-bit PCM.
    Pcm24,
    /// Signed 32-bit PCM.
    Pcm32,
    /// IEEE float, 32 bit.
    Float32,
    /// IEEE float, 64 bit.
    Float64,
    /// IMA ADPCM (4 bits a sample, in blocks).
    ImaAdpcm,
    /// Vorbis.
    Vorbis,
}

/// What a file is, from its headers alone (cheap: nothing is decoded).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamInfo {
    /// The file format.
    pub container: Container,
    /// The coding.
    pub codec: Codec,
    /// Frames per second.
    pub rate: u32,
    /// Samples per frame.
    pub channels: u16,
    /// The number of frames, if the headers say (always for a WAV; for an Ogg from its last page).
    pub frames: Option<u64>,
}

impl StreamInfo {
    /// The size of the sound decoded to 16-bit samples, if the length is known: what keeping it in memory costs the game.
    pub fn decoded_bytes(&self) -> Option<u64> {
        self.frames
            .map(|f| f.saturating_mul(u64::from(self.channels)).saturating_mul(2))
    }
}

/// What a file looks like from its first bytes.
pub fn sniff(bytes: &[u8]) -> Option<Container> {
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
        Some(Container::Wav)
    } else if bytes.len() >= 4 && &bytes[0..4] == b"OggS" {
        Some(Container::Ogg)
    } else {
        None
    }
}

fn bad<T>(what: &'static str, why: impl Into<String>) -> Result<T> {
    Err(Error::invalid(what, why))
}

/// Read what a file is without decoding it.
pub fn probe(bytes: &[u8]) -> Result<StreamInfo> {
    match sniff(bytes) {
        Some(Container::Wav) => {
            let wav = parse_wav(bytes)?;
            Ok(StreamInfo {
                container: Container::Wav,
                codec: wav.codec,
                rate: wav.fmt.rate,
                channels: wav.fmt.channels,
                frames: Some(wav.frames()?),
            })
        }
        Some(Container::Ogg) => {
            let reader = open_vorbis(bytes)?;
            Ok(StreamInfo {
                container: Container::Ogg,
                codec: Codec::Vorbis,
                rate: reader.ident_hdr.audio_sample_rate,
                channels: u16::from(reader.ident_hdr.audio_channels),
                frames: last_granule(bytes),
            })
        }
        None => bad("sound", "neither a RIFF/WAVE nor an Ogg file"),
    }
}

/// Decode a whole file by what it looks like.
pub fn decode(bytes: &[u8]) -> Result<Pcm> {
    match sniff(bytes) {
        Some(Container::Wav) => decode_wav(bytes),
        Some(Container::Ogg) => decode_vorbis(bytes),
        None => bad("sound", "neither a RIFF/WAVE nor an Ogg file"),
    }
}

// ---------------------------------------------------------------------------------------------------------------------------------
// WAV

struct Fmt {
    tag: u16,
    channels: u16,
    rate: u32,
    block_align: u16,
    bits: u16,
    samples_per_block: u16,
}

struct Wav<'a> {
    fmt: Fmt,
    codec: Codec,
    data: &'a [u8],
    fact: Option<u32>,
}

/// The tail of the GUIDs of `WAVE_FORMAT_EXTENSIBLE`'s sub-formats (`xxxxxxxx-0000-0010-8000-00aa00389b71`).
const GUID_TAIL: [u8; 14] = [
    0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
];

fn u16_at(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}

fn u32_at(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

fn parse_fmt(body: &[u8]) -> Result<Fmt> {
    if body.len() < 16 {
        return bad("wav", "the fmt chunk is shorter than 16 bytes");
    }
    let mut fmt = Fmt {
        tag: u16_at(body, 0),
        channels: u16_at(body, 2),
        rate: u32_at(body, 4),
        block_align: u16_at(body, 12),
        bits: u16_at(body, 14),
        samples_per_block: 0,
    };
    if fmt.tag == 0xFFFE {
        if body.len() < 40 || u16_at(body, 16) < 22 {
            return bad("wav", "an extensible fmt chunk shorter than 40 bytes");
        }
        if body[26..40] != GUID_TAIL {
            return bad(
                "wav",
                "an extensible fmt chunk with a sub-format that is not a registered format tag",
            );
        }
        fmt.tag = u16_at(body, 24);
    }
    if fmt.tag == 0x11 && body.len() >= 20 && u16_at(body, 16) >= 2 {
        fmt.samples_per_block = u16_at(body, 18);
    }
    Ok(fmt)
}

fn codec_of(fmt: &Fmt) -> Result<Codec> {
    Ok(match (fmt.tag, fmt.bits) {
        (1, 8) => Codec::Pcm8,
        (1, 16) => Codec::Pcm16,
        (1, 24) => Codec::Pcm24,
        (1, 32) => Codec::Pcm32,
        (3, 32) => Codec::Float32,
        (3, 64) => Codec::Float64,
        (0x11, 4) => Codec::ImaAdpcm,
        (tag, bits) => {
            return bad(
                "wav",
                format!(
                    "format tag {tag:#06x} with {bits} bits per sample is not one of PCM 8/16/24/32, float 32/64 or IMA ADPCM"
                ),
            );
        }
    })
}

fn parse_wav(bytes: &[u8]) -> Result<Wav<'_>> {
    if sniff(bytes) != Some(Container::Wav) {
        return bad("wav", "not a RIFF/WAVE file");
    }
    let (mut fmt, mut data, mut fact) = (None, None, None);
    let mut pos = 12usize;
    // Every pass advances by at least 8 bytes, so a file of n bytes is walked in at most n/8 passes.
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = u32_at(bytes, pos + 4);
        let start = pos + 8;
        let available = bytes.len() - start;
        let declared = size as usize;
        let truncated = declared > available;
        match id {
            b"fmt " => {
                if truncated {
                    return bad("wav", "the fmt chunk runs past the end of the file");
                }
                if fmt.is_none() {
                    fmt = Some(parse_fmt(&bytes[start..start + declared])?);
                }
            }
            b"fact" if declared >= 4 && !truncated => {
                fact.get_or_insert(u32_at(bytes, start));
            }
            // A data chunk that claims more than the file holds (a file cut short, or 0xFFFFFFFF from a writer that streams) is the rest of the file.
            b"data" if data.is_none() => {
                data = Some(&bytes[start..start + declared.min(available)]);
            }
            _ => {}
        }
        if truncated {
            break;
        }
        pos = start + declared + (declared & 1);
    }
    let Some(fmt) = fmt else {
        return bad("wav", "no fmt chunk");
    };
    let Some(data) = data else {
        return bad("wav", "no data chunk");
    };
    if fmt.channels == 0 || fmt.channels > MAX_CHANNELS {
        return bad(
            "wav",
            format!(
                "{} channels (1 to {MAX_CHANNELS} are accepted)",
                fmt.channels
            ),
        );
    }
    if fmt.rate == 0 || fmt.rate > MAX_RATE {
        return bad("wav", format!("a sample rate of {} Hz", fmt.rate));
    }
    let codec = codec_of(&fmt)?;
    Ok(Wav {
        fmt,
        codec,
        data,
        fact,
    })
}

const IMA_STEPS: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449,
    494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493,
    10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];
const IMA_INDEX: [i32; 8] = [-1, -1, -1, -1, 2, 4, 6, 8];

impl Wav<'_> {
    fn ima_layout(&self) -> Result<(usize, usize)> {
        let channels = usize::from(self.fmt.channels);
        let block = usize::from(self.fmt.block_align);
        // A block is a 4-byte header per channel, then groups of 4 bytes (8 samples) per channel.
        if block < 4 * channels || !(block - 4 * channels).is_multiple_of(4 * channels) {
            return bad(
                "wav",
                format!("an IMA ADPCM block of {block} bytes for {channels} channels"),
            );
        }
        let computed = 1 + 2 * (block - 4 * channels) / channels;
        let stated = usize::from(self.fmt.samples_per_block);
        let per_block = if stated == 0 {
            computed
        } else {
            stated.min(computed)
        };
        Ok((block, per_block))
    }

    /// The number of frames the decoded sound has (what `fact` says for ADPCM when it is less than the blocks hold).
    fn frames(&self) -> Result<u64> {
        let channels = usize::from(self.fmt.channels);
        let frames = match self.codec {
            Codec::ImaAdpcm => {
                let (block, per_block) = self.ima_layout()?;
                let whole = self.data.len() / block;
                let rest = self.data.len() % block;
                let mut frames = whole * per_block;
                if rest >= 4 * channels {
                    frames += (1 + 8 * ((rest - 4 * channels) / (4 * channels))).min(per_block);
                }
                match self.fact {
                    Some(f) if (f as usize) < frames => f as usize,
                    _ => frames,
                }
            }
            Codec::Pcm8 => self.data.len() / channels,
            Codec::Pcm16 => self.data.len() / (2 * channels),
            Codec::Pcm24 => self.data.len() / (3 * channels),
            Codec::Pcm32 | Codec::Float32 => self.data.len() / (4 * channels),
            Codec::Float64 => self.data.len() / (8 * channels),
            Codec::Vorbis => 0,
        };
        Ok(frames as u64)
    }

    fn decode(&self) -> Result<Pcm> {
        let channels = usize::from(self.fmt.channels);
        let frames = self.frames()?;
        if frames > MAX_FRAMES {
            return bad(
                "wav",
                format!("{frames} frames is longer than the limit of {MAX_FRAMES}"),
            );
        }
        let frames = frames as usize;
        let samples: Vec<f32> = match self.codec {
            Codec::Pcm8 => self.data[..frames * channels]
                .iter()
                .map(|&b| (f32::from(b) - 128.0) / 128.0)
                .collect(),
            Codec::Pcm16 => self.data[..frames * channels * 2]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| f32::from(i16::from_le_bytes(*c)) / 32768.0)
                .collect(),
            Codec::Pcm24 => self.data[..frames * channels * 3]
                .as_chunks::<3>()
                .0
                .iter()
                .map(|c| {
                    // Sign-extend by placing the 24 bits in the top of an i32.
                    let v = i32::from_le_bytes([0, c[0], c[1], c[2]]) >> 8;
                    v as f32 / 8_388_608.0
                })
                .collect(),
            Codec::Pcm32 => self.data[..frames * channels * 4]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|c| (f64::from(i32::from_le_bytes(*c)) / 2_147_483_648.0) as f32)
                .collect(),
            Codec::Float32 => self.data[..frames * channels * 4]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|c| finite(f32::from_le_bytes(*c)))
                .collect(),
            Codec::Float64 => self.data[..frames * channels * 8]
                .as_chunks::<8>()
                .0
                .iter()
                .map(|c| finite(f64::from_le_bytes(*c) as f32))
                .collect(),
            Codec::ImaAdpcm => self.decode_ima(frames)?,
            Codec::Vorbis => unreachable!("a WAV never holds Vorbis"),
        };
        Ok(Pcm {
            rate: self.fmt.rate,
            channels: self.fmt.channels,
            samples,
        })
    }

    fn decode_ima(&self, frames: usize) -> Result<Vec<f32>> {
        let channels = usize::from(self.fmt.channels);
        let (block_size, per_block) = self.ima_layout()?;
        let mut out = vec![0.0f32; frames * channels];
        let mut frame0 = 0usize;
        for block in self.data.chunks(block_size) {
            if block.len() < 4 * channels || frame0 >= frames {
                break;
            }
            let groups = (block.len() - 4 * channels) / (4 * channels);
            let block_frames = (1 + 8 * groups).min(per_block).min(frames - frame0);
            for c in 0..channels {
                let mut predictor = i32::from(i16::from_le_bytes([block[4 * c], block[4 * c + 1]]));
                let mut index = i32::from(block[4 * c + 2]).min(88);
                out[frame0 * channels + c] = predictor as f32 / 32768.0;
                for n in 1..block_frames {
                    let k = n - 1;
                    let (group, within) = (k / 8, k % 8);
                    let byte = block[4 * channels + group * 4 * channels + c * 4 + within / 2];
                    let nibble = if within % 2 == 0 {
                        byte & 0x0F
                    } else {
                        byte >> 4
                    };
                    let step = IMA_STEPS[index as usize];
                    let mut diff = step >> 3;
                    if nibble & 1 != 0 {
                        diff += step >> 2;
                    }
                    if nibble & 2 != 0 {
                        diff += step >> 1;
                    }
                    if nibble & 4 != 0 {
                        diff += step;
                    }
                    predictor = if nibble & 8 != 0 {
                        predictor - diff
                    } else {
                        predictor + diff
                    };
                    predictor = predictor.clamp(-32768, 32767);
                    index = (index + IMA_INDEX[usize::from(nibble & 7)]).clamp(0, 88);
                    out[(frame0 + n) * channels + c] = predictor as f32 / 32768.0;
                }
            }
            frame0 += block_frames;
        }
        Ok(out)
    }
}

/// Not-a-number and infinity are silence, so that one bad sample of a damaged file cannot poison the mix.
fn finite(v: f32) -> f32 {
    if v.is_finite() { v } else { 0.0 }
}

/// Decode a `.wav` file.
pub fn decode_wav(bytes: &[u8]) -> Result<Pcm> {
    parse_wav(bytes)?.decode()
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Vorbis

fn open_vorbis(bytes: &[u8]) -> Result<OggStreamReader<Cursor<&[u8]>>> {
    let reader = catch_unwind(AssertUnwindSafe(|| {
        OggStreamReader::new(Cursor::new(bytes))
    }))
    .map_err(|_| Error::invalid("ogg", "the Vorbis decoder failed on the headers".to_owned()))?
    .map_err(|e| Error::invalid("ogg", format!("not a readable Ogg Vorbis stream: {e}")))?;
    let channels = u16::from(reader.ident_hdr.audio_channels);
    if channels == 0 || channels > MAX_CHANNELS {
        return bad(
            "ogg",
            format!("{channels} channels (1 to {MAX_CHANNELS} are accepted)"),
        );
    }
    let rate = reader.ident_hdr.audio_sample_rate;
    if rate == 0 || rate > MAX_RATE {
        return bad("ogg", format!("a sample rate of {rate} Hz"));
    }
    Ok(reader)
}

/// The granule position of the last Ogg page in `bytes` that has one: for Vorbis, the number of frames in the stream.
pub fn last_granule(bytes: &[u8]) -> Option<u64> {
    // A page header is 27 bytes; the granule position is the 8 bytes at offset 6. Search backwards for the last capture pattern that begins a
    // plausible header (version 0).
    let mut end = bytes.len();
    while let Some(i) = bytes[..end].windows(4).rposition(|w| w == b"OggS") {
        if i + 27 <= bytes.len() && bytes[i + 4] == 0 {
            let granule = i64::from_le_bytes(bytes[i + 6..i + 14].try_into().ok()?);
            if granule >= 0 {
                return Some(granule as u64);
            }
        }
        end = i;
    }
    None
}

/// Decode an Ogg Vorbis file.
pub fn decode_vorbis(bytes: &[u8]) -> Result<Pcm> {
    let mut stream = VorbisStream::new(bytes)?;
    let (rate, channels) = (stream.rate(), stream.channels());
    let mut samples = Vec::new();
    while let Some(chunk) = stream.read_chunk()? {
        samples.extend_from_slice(&chunk);
        if (samples.len() / usize::from(channels)) as u64 > MAX_FRAMES {
            return bad(
                "ogg",
                format!("longer than the limit of {MAX_FRAMES} frames"),
            );
        }
    }
    Ok(Pcm {
        rate,
        channels,
        samples,
    })
}

/// A Vorbis file decoded a piece at a time, for music that is not kept whole in memory.
pub struct VorbisStream<'a> {
    reader: OggStreamReader<Cursor<&'a [u8]>>,
    channels: u16,
    rate: u32,
}

impl<'a> VorbisStream<'a> {
    /// Read the headers of `bytes`.
    pub fn new(bytes: &'a [u8]) -> Result<Self> {
        let reader = open_vorbis(bytes)?;
        let channels = u16::from(reader.ident_hdr.audio_channels);
        let rate = reader.ident_hdr.audio_sample_rate;
        Ok(Self {
            reader,
            channels,
            rate,
        })
    }

    /// Frames per second.
    pub fn rate(&self) -> u32 {
        self.rate
    }

    /// Samples per frame.
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// The next piece of interleaved samples (a few hundred to a few thousand frames), or `None` at the end. An empty piece can occur.
    pub fn read_chunk(&mut self) -> Result<Option<Vec<f32>>> {
        let packet = catch_unwind(AssertUnwindSafe(|| {
            self.reader
                .read_dec_packet_generic::<InterleavedSamples<f32>>()
        }))
        .map_err(|_| Error::invalid("ogg", "the Vorbis decoder failed on a packet".to_owned()))?
        .map_err(|e| Error::invalid("ogg", format!("damaged Vorbis audio: {e}")))?;
        Ok(packet.map(|p| p.samples.into_iter().map(finite).collect()))
    }
}

// ---------------------------------------------------------------------------------------------------------------------------------
// What to keep whole and what to stream

/// How a sound is held by the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delivery {
    /// Decoded once into memory (an effect: short, played many times, started at once).
    Decoded,
    /// Decoded a piece at a time as it plays (music and long speech: too big to keep).
    Streamed,
}

/// The size limits that decide [`Delivery`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeliveryPolicy {
    /// Stream a sound whose 16-bit decoded size would be more than this many bytes (4 MiB: about 47 seconds of mono at 44.1 kHz).
    pub stream_above_decoded_bytes: u64,
    /// When the length is not known (an Ogg without a readable last page), stream a file larger than this many bytes (256 KiB).
    pub stream_above_file_bytes: u64,
}

impl Default for DeliveryPolicy {
    fn default() -> Self {
        Self {
            stream_above_decoded_bytes: 4 * 1024 * 1024,
            stream_above_file_bytes: 256 * 1024,
        }
    }
}

impl DeliveryPolicy {
    /// Decide for a file of `file_len` bytes that is `info`. Short sounds are decoded; one that would take more than
    /// [`stream_above_decoded_bytes`](Self::stream_above_decoded_bytes) once decoded is streamed (exactly at the limit is still decoded).
    pub fn delivery(&self, info: &StreamInfo, file_len: u64) -> Delivery {
        let streamed = match info.decoded_bytes() {
            Some(bytes) => bytes > self.stream_above_decoded_bytes,
            None => file_len > self.stream_above_file_bytes,
        };
        if streamed {
            Delivery::Streamed
        } else {
            Delivery::Decoded
        }
    }
}
