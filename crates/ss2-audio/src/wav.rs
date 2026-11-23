//! WAV files for the offline renderer (roadmap T06-013): **writing** what the mixer rendered, and **reading** it back (for tests and golden
//! files). Only what the renderer produces: PCM 16-bit and IEEE float 32-bit, 1 or 2 channels. This is not the game's `.wav` decoder (T06-002,
//! which has to read what the original shipped).

use ss2_core::{Error, Result};

fn put_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn header(out: &mut Vec<u8>, format: u16, channels: u16, rate: u32, bits: u16, data_len: u32) {
    let block = channels * bits / 8;
    out.extend_from_slice(b"RIFF");
    put_u32(out, 36 + data_len);
    out.extend_from_slice(b"WAVEfmt ");
    put_u32(out, 16);
    put_u16(out, format);
    put_u16(out, channels);
    put_u32(out, rate);
    put_u32(out, rate * u32::from(block));
    put_u16(out, block);
    put_u16(out, bits);
    out.extend_from_slice(b"data");
    put_u32(out, data_len);
}

/// A 16-bit PCM WAV of interleaved `samples` (clamped to -1 to 1 and rounded). `Err` for a channel count other than 1 or 2, a rate of 0, or
/// a length that is not whole frames.
pub fn encode_pcm16(rate: u32, channels: u8, samples: &[f32]) -> Result<Vec<u8>> {
    check(rate, channels, samples.len())?;
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    header(
        &mut out,
        1,
        u16::from(channels),
        rate,
        16,
        (samples.len() * 2) as u32,
    );
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    Ok(out)
}

/// A 32-bit float WAV of interleaved `samples`, exactly as given.
pub fn encode_f32(rate: u32, channels: u8, samples: &[f32]) -> Result<Vec<u8>> {
    check(rate, channels, samples.len())?;
    let mut out = Vec::with_capacity(44 + samples.len() * 4);
    header(
        &mut out,
        3,
        u16::from(channels),
        rate,
        32,
        (samples.len() * 4) as u32,
    );
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    Ok(out)
}

fn check(rate: u32, channels: u8, len: usize) -> Result<()> {
    if rate == 0
        || !(1..=2).contains(&channels)
        || !len.is_multiple_of(usize::from(channels))
        || len > (u32::MAX as usize - 44) / 4
    {
        return Err(Error::invalid(
            "wav",
            format!("rate {rate}, {channels} channels, {len} samples"),
        ));
    }
    Ok(())
}

/// What [`decode`] read.
#[derive(Clone, Debug, PartialEq)]
pub struct Decoded {
    /// Frames per second.
    pub rate: u32,
    /// 1 or 2.
    pub channels: u8,
    /// Interleaved samples as `f32` (16-bit values scaled by 1/32768... see [`encode_pcm16`]: scaled by 1/32767).
    pub samples: Vec<f32>,
}

/// Read a WAV written by [`encode_pcm16`] or [`encode_f32`]. `Err` for anything else (other formats, truncated files).
pub fn decode(bytes: &[u8]) -> Result<Decoded> {
    let bad = |why: &str| Err(Error::invalid("wav", why.to_owned()));
    if bytes.len() < 44
        || &bytes[0..4] != b"RIFF"
        || &bytes[8..16] != b"WAVEfmt "
        || &bytes[36..40] != b"data"
    {
        return bad("not a WAV of the kind the renderer writes");
    }
    let u16_at = |i: usize| u16::from_le_bytes([bytes[i], bytes[i + 1]]);
    let u32_at =
        |i: usize| u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
    let (format, channels, rate, bits) = (u16_at(20), u16_at(22), u32_at(24), u16_at(34));
    let data_len = u32_at(40) as usize;
    if data_len > bytes.len() - 44 || !(1..=2).contains(&channels) || rate == 0 {
        return bad("truncated or invalid header");
    }
    let data = &bytes[44..44 + data_len];
    let samples = match (format, bits) {
        (1, 16) if data_len.is_multiple_of(2) => data
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| f32::from(i16::from_le_bytes(*c)) / 32767.0)
            .collect(),
        (3, 32) if data_len.is_multiple_of(4) => data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| f32::from_le_bytes(*c))
            .collect(),
        _ => return bad("only 16-bit PCM and 32-bit float are read"),
    };
    Ok(Decoded {
        rate,
        channels: channels as u8,
        samples,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_wav_round_trips_exactly_and_has_the_right_header() {
        let samples = vec![0.0, 0.25, -0.5, 1.0, f32::MIN_POSITIVE, -0.0];
        let bytes = encode_f32(44_100, 2, &samples).unwrap();
        assert_eq!(bytes.len(), 44 + 24);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 36 + 24);
        assert_eq!(
            u16::from_le_bytes(bytes[20..22].try_into().unwrap()),
            3,
            "IEEE float"
        );
        assert_eq!(
            u32::from_le_bytes(bytes[28..32].try_into().unwrap()),
            44_100 * 8,
            "byte rate"
        );
        assert_eq!(
            u16::from_le_bytes(bytes[32..34].try_into().unwrap()),
            8,
            "block align"
        );
        let d = decode(&bytes).unwrap();
        assert_eq!((d.rate, d.channels), (44_100, 2));
        assert!(
            d.samples
                .iter()
                .zip(&samples)
                .all(|(a, b)| a.to_bits() == b.to_bits())
        );
    }

    #[test]
    fn pcm16_wav_round_trips_within_a_quantisation_step_and_clamps() {
        let samples = vec![0.0, 0.5, -0.5, 1.0, -1.0, 2.0, -3.0, 0.123_456];
        let bytes = encode_pcm16(48_000, 1, &samples).unwrap();
        assert_eq!(bytes.len(), 44 + 16);
        let d = decode(&bytes).unwrap();
        for (got, want) in d.samples.iter().zip(&samples) {
            assert!(
                (got - want.clamp(-1.0, 1.0)).abs() <= 1.0 / 32767.0,
                "{got} vs {want}"
            );
        }
        assert_eq!(d.samples[5], 1.0);
        assert_eq!(d.samples[6], -1.0);
    }

    #[test]
    fn bad_input_is_refused_both_ways() {
        assert!(encode_pcm16(0, 1, &[0.0]).is_err());
        assert!(encode_pcm16(48_000, 3, &[0.0; 3]).is_err());
        assert!(encode_pcm16(48_000, 0, &[]).is_err());
        assert!(
            encode_f32(48_000, 2, &[0.0; 3]).is_err(),
            "not whole frames"
        );
        let good = encode_f32(48_000, 2, &[0.0; 4]).unwrap();
        for cut in 0..good.len() {
            assert!(decode(&good[..cut]).is_err(), "cut to {cut}");
        }
        let mut other = good.clone();
        other[20] = 7; // an unknown format
        assert!(decode(&other).is_err());
        let mut lying = good.clone();
        lying[40..44].copy_from_slice(&1000u32.to_le_bytes());
        assert!(
            decode(&lying).is_err(),
            "a data length that the file cannot hold"
        );
        assert!(
            decode(b"not a wav at all, but long enough to pass the length check........").is_err()
        );
        assert_eq!(
            decode(&encode_f32(48_000, 1, &[]).unwrap())
                .unwrap()
                .samples
                .len(),
            0,
            "an empty file is fine"
        );
    }
}
