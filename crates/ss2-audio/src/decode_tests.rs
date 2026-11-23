//! Tests of the decoders: hand-built files for every PCM variant and every way a file can be damaged, and small synthetic fixtures in
//! `testdata/audio/` whose expected samples come from another decoder (FFmpeg's), to which these must agree.

use std::path::PathBuf;

use crate::decode::*;

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/audio")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

// ---- building WAV files by hand -------------------------------------------------------------------------------------------------

fn chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out = id.to_vec();
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(body);
    if body.len() % 2 == 1 {
        out.push(0);
    }
    out
}

fn fmt_body(tag: u16, channels: u16, rate: u32, bits: u16) -> Vec<u8> {
    let block = channels * bits / 8;
    let mut b = Vec::new();
    b.extend_from_slice(&tag.to_le_bytes());
    b.extend_from_slice(&channels.to_le_bytes());
    b.extend_from_slice(&rate.to_le_bytes());
    b.extend_from_slice(&(rate * u32::from(block)).to_le_bytes());
    b.extend_from_slice(&block.to_le_bytes());
    b.extend_from_slice(&bits.to_le_bytes());
    b
}

fn riff(chunks: &[Vec<u8>]) -> Vec<u8> {
    let body: Vec<u8> = chunks.concat();
    let mut out = b"RIFF".to_vec();
    out.extend_from_slice(&((body.len() + 4) as u32).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(&body);
    out
}

fn wav(tag: u16, channels: u16, rate: u32, bits: u16, data: &[u8]) -> Vec<u8> {
    riff(&[
        chunk(b"fmt ", &fmt_body(tag, channels, rate, bits)),
        chunk(b"data", data),
    ])
}

fn extensible(tag: u16, channels: u16, rate: u32, bits: u16, data: &[u8]) -> Vec<u8> {
    let mut b = fmt_body(0xFFFE, channels, rate, bits);
    b.extend_from_slice(&22u16.to_le_bytes()); // cbSize
    b.extend_from_slice(&bits.to_le_bytes()); // valid bits
    b.extend_from_slice(&3u32.to_le_bytes()); // channel mask
    b.extend_from_slice(&tag.to_le_bytes());
    b.extend_from_slice(&[
        0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71,
    ]);
    riff(&[chunk(b"fmt ", &b), chunk(b"data", data)])
}

fn close(a: f32, b: f32, eps: f32) -> bool {
    (a - b).abs() <= eps
}

// ---- PCM ---------------------------------------------------------------------------------------------------------------------------

#[test]
fn pcm16_decodes_to_its_exact_fractions() {
    let data: Vec<u8> = [0i16, 16384, -16384, 32767, -32768]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    let pcm = decode(&wav(1, 1, 44_100, 16, &data)).unwrap();
    assert_eq!((pcm.rate, pcm.channels, pcm.frames()), (44_100, 1, 5));
    assert_eq!(pcm.samples, [0.0, 0.5, -0.5, 32767.0 / 32768.0, -1.0]);
}

#[test]
fn pcm8_is_unsigned_around_128() {
    let pcm = decode(&wav(1, 1, 11_025, 8, &[128, 255, 0, 192, 64])).unwrap();
    assert_eq!(pcm.samples, [0.0, 127.0 / 128.0, -1.0, 0.5, -0.5]);
}

#[test]
fn pcm24_is_sign_extended() {
    let data = [
        0x00, 0x00, 0x00, // 0
        0x00, 0x00, 0x40, // +0.5
        0x00, 0x00, 0xC0, // -0.5
        0xFF, 0xFF, 0x7F, // max
        0x00, 0x00, 0x80, // min
    ];
    let pcm = decode(&wav(1, 1, 48_000, 24, &data)).unwrap();
    assert_eq!(
        pcm.samples,
        [0.0, 0.5, -0.5, 8_388_607.0 / 8_388_608.0, -1.0]
    );
}

#[test]
fn pcm32_and_floats() {
    let data: Vec<u8> = [0i32, 1 << 30, -(1 << 30), i32::MIN]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    assert_eq!(
        decode(&wav(1, 1, 8000, 32, &data)).unwrap().samples,
        [0.0, 0.5, -0.5, -1.0]
    );
    let data: Vec<u8> = [0.25f32, -0.75, 1.5]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    assert_eq!(
        decode(&wav(3, 1, 8000, 32, &data)).unwrap().samples,
        [0.25, -0.75, 1.5],
        "floats beyond 1 are kept (the mixer clamps)"
    );
    let data: Vec<u8> = [0.125f64, -0.5]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    assert_eq!(
        decode(&wav(3, 1, 8000, 64, &data)).unwrap().samples,
        [0.125, -0.5]
    );
}

#[test]
fn a_float_that_is_not_a_number_is_silence() {
    let data: Vec<u8> = [f32::NAN, f32::INFINITY, -f32::INFINITY, 0.5]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    assert_eq!(
        decode(&wav(3, 1, 8000, 32, &data)).unwrap().samples,
        [0.0, 0.0, 0.0, 0.5]
    );
}

#[test]
fn stereo_stays_interleaved() {
    let data: Vec<u8> = [100i16, -100, 200, -200]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    let pcm = decode(&wav(1, 2, 22_050, 16, &data)).unwrap();
    assert_eq!((pcm.channels, pcm.frames()), (2, 2));
    assert!(
        close(pcm.samples[0], 100.0 / 32768.0, 1e-9)
            && close(pcm.samples[1], -100.0 / 32768.0, 1e-9)
    );
    assert!(
        close(pcm.samples[2], 200.0 / 32768.0, 1e-9)
            && close(pcm.samples[3], -200.0 / 32768.0, 1e-9)
    );
}

#[test]
fn the_extensible_form_gives_the_same_samples_as_the_plain_one() {
    let data: Vec<u8> = [1000i16, -2000, 3000, -4000]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    let plain = decode(&wav(1, 2, 44_100, 16, &data)).unwrap();
    let ext = decode(&extensible(1, 2, 44_100, 16, &data)).unwrap();
    assert_eq!(plain, ext);
    let data: Vec<u8> = [0.5f32, -0.5]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    assert_eq!(
        decode(&extensible(3, 1, 44_100, 32, &data))
            .unwrap()
            .samples,
        [0.5, -0.5]
    );
}

#[test]
fn an_extensible_file_with_an_unregistered_sub_format_is_refused() {
    let mut bad = extensible(1, 1, 44_100, 16, &[0, 0]);
    // Damage the GUID tail inside the fmt chunk (starts at offset 12 + 8 + 26).
    bad[12 + 8 + 30] ^= 0xFF;
    assert!(decode_wav(&bad).is_err());
}

// ---- the container -----------------------------------------------------------------------------------------------------------------

#[test]
fn chunks_are_found_in_any_order_with_padding_and_unknown_ones_between() {
    let data: Vec<u8> = [7i16, 8, 9].iter().flat_map(|v| v.to_le_bytes()).collect();
    let file = riff(&[
        chunk(b"LIST", b"INFOodd"), // odd length: padded
        chunk(b"fmt ", &fmt_body(1, 1, 22_050, 16)),
        chunk(b"junk", &[1, 2, 3]), // odd length: padded
        chunk(b"data", &data),
        chunk(b"cue ", &[0; 5]),
    ]);
    let pcm = decode(&file).unwrap();
    assert_eq!(pcm.samples.len(), 3);
    assert!(close(pcm.samples[2], 9.0 / 32768.0, 1e-9));
}

#[test]
fn a_data_chunk_that_claims_more_than_the_file_holds_is_the_rest_of_the_file() {
    let data: Vec<u8> = [1i16, 2, 3, 4]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    let mut file = wav(1, 1, 8000, 16, &data);
    let size_at = file.len() - data.len() - 4;
    file[size_at..size_at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(
        decode(&file).unwrap().samples.len(),
        4,
        "0xFFFFFFFF (a streaming writer)"
    );
    file[size_at..size_at + 4].copy_from_slice(&1_000_000u32.to_le_bytes());
    assert_eq!(decode(&file).unwrap().samples.len(), 4, "a file cut short");
    let cut = &file[..file.len() - 1];
    assert_eq!(
        decode(cut).unwrap().samples.len(),
        3,
        "and a half sample at the end is dropped"
    );
    let cut = &file[..file.len() - 3];
    assert_eq!(decode(cut).unwrap().samples.len(), 2);
}

#[test]
fn a_zero_length_data_chunk_is_silence_not_an_error() {
    let pcm = decode(&wav(1, 1, 8000, 16, &[])).unwrap();
    assert_eq!((pcm.frames(), pcm.rate), (0, 8000));
    assert_eq!(pcm.seconds(), 0.0);
}

#[test]
fn files_that_are_not_sounds_or_not_whole_are_refused_without_a_panic() {
    assert!(decode(&[]).is_err());
    assert!(decode(b"RIFF").is_err());
    assert!(decode(b"hello, this is not a sound file").is_err());
    let good = wav(1, 1, 8000, 16, &[0, 0, 1, 0]);
    for cut in 0..good.len() {
        // Every prefix either decodes (to fewer samples) or is an error; none panics.
        let _ = decode(&good[..cut]);
    }
    assert!(
        decode_wav(&riff(&[chunk(b"data", &[0, 0])])).is_err(),
        "no fmt chunk"
    );
    assert!(
        decode_wav(&riff(&[chunk(b"fmt ", &fmt_body(1, 1, 8000, 16))])).is_err(),
        "no data chunk"
    );
    assert!(
        decode_wav(&riff(&[chunk(b"fmt ", &[1, 0, 1, 0])])).is_err(),
        "a fmt chunk that is too short"
    );
}

#[test]
fn impossible_headers_are_refused() {
    let d = [0u8; 8];
    assert!(decode(&wav(1, 0, 8000, 16, &d)).is_err(), "no channels");
    assert!(decode(&wav(1, 9, 8000, 16, &d)).is_err(), "nine channels");
    assert!(decode(&wav(1, 1, 0, 16, &d)).is_err(), "rate 0");
    assert!(
        decode(&wav(1, 1, 400_000, 16, &d)).is_err(),
        "rate over the limit"
    );
    assert!(
        decode(&wav(1, 1, 384_000, 16, &d)).is_ok(),
        "the highest accepted rate"
    );
    assert!(decode(&wav(1, 1, 384_001, 16, &d)).is_err(), "one above it");
    assert!(decode(&wav(1, 1, 8000, 12, &d)).is_err(), "12-bit PCM");
    assert!(
        decode(&wav(2, 1, 8000, 4, &d)).is_err(),
        "Microsoft ADPCM is not read"
    );
    assert!(
        decode(&wav(6, 1, 8000, 8, &d)).is_err(),
        "A-law is not read"
    );
    assert!(
        decode(&wav(0x55, 1, 8000, 0, &d)).is_err(),
        "MP3 in a WAV is not read"
    );
    assert_eq!(
        decode(&wav(1, 8, 8000, 16, &[0; 32])).unwrap().channels,
        8,
        "eight channels are fine"
    );
}

// ---- IMA ADPCM -----------------------------------------------------------------------------------------------------------------------

#[test]
fn the_ima_step_table_is_the_standard_one() {
    // Checked through behaviour: a nibble of 7 from step index 0 adds step>>3 + step>>2 + step>>1 + step = 0 + 1 + 3 + 7 = 11, and the
    // index then moves by +8; a nibble of 0 from the top index leaves the sample and the index. The full table is compared with FFmpeg's
    // output by the fixture tests below.
    let mut block = vec![0u8; 8];
    block[0..2].copy_from_slice(&1000i16.to_le_bytes());
    block[2] = 0; // step index 0 (step 7)
    block[4] = 0x07; // two nibbles: 7 then 0
    let mut data = block.clone();
    data[4] = 0x07;
    let file = ima_file(1, 8, &data, Some(3));
    let pcm = decode(&file).unwrap();
    assert_eq!(pcm.samples.len(), 3);
    assert!(close(pcm.samples[0], 1000.0 / 32768.0, 1e-9));
    assert!(
        close(pcm.samples[1], 1011.0 / 32768.0, 1e-9),
        "{:?}",
        pcm.samples
    );
    // Second nibble 0 at index 8 (step 16): diff = 16>>3 = 2.
    assert!(
        close(pcm.samples[2], 1013.0 / 32768.0, 1e-9),
        "{:?}",
        pcm.samples
    );
}

fn ima_file(channels: u16, block_align: u16, data: &[u8], fact: Option<u32>) -> Vec<u8> {
    let spb = 1 + 2 * usize::from(block_align).saturating_sub(4 * usize::from(channels))
        / usize::from(channels);
    let mut f = fmt_body(0x11, channels, 22_050, 4);
    f[12..14].copy_from_slice(&block_align.to_le_bytes());
    f.extend_from_slice(&2u16.to_le_bytes());
    f.extend_from_slice(&(spb as u16).to_le_bytes());
    let mut chunks = vec![chunk(b"fmt ", &f)];
    if let Some(n) = fact {
        chunks.push(chunk(b"fact", &n.to_le_bytes()));
    }
    chunks.push(chunk(b"data", data));
    riff(&chunks)
}

#[test]
fn a_run_of_the_largest_nibble_climbs_monotonically_and_saturates_the_predictor() {
    // Nibble 7 only ever adds, and moves the step index up by 8 each time, so the predictor rises until it saturates at the 16-bit top.
    let mut data = vec![0u8; 4 + 4 * 16];
    data[0..2].copy_from_slice(&0i16.to_le_bytes());
    for b in &mut data[4..] {
        *b = 0x77;
    }
    let block = u16::try_from(data.len()).unwrap();
    let pcm = decode(&ima_file(1, block, &data, None)).unwrap();
    assert_eq!(pcm.samples.len(), 1 + 8 * 16);
    assert!(
        pcm.samples.windows(2).all(|w| w[1] >= w[0]),
        "nibble 7 only ever adds"
    );
    assert!(
        close(*pcm.samples.last().unwrap(), 32767.0 / 32768.0, 1e-9),
        "and the predictor saturates at the top"
    );
}

#[test]
fn ima_adpcm_agrees_with_ffmpeg_across_the_whole_step_table() {
    // `ima_sweep.wav` is noise from near silence to full scale: its 14 blocks visit all 89 step indexes (checked when it was made), so a wrong
    // entry anywhere in the step table changes some sample.
    let pcm = decode(&fixture("ima_sweep.wav")).unwrap();
    let reference = decode(&fixture("ima_sweep.ref.wav")).unwrap();
    assert_eq!((pcm.rate, pcm.channels), (22_050, 1));
    assert!(pcm.frames() >= 6000 && reference.frames() >= pcm.frames());
    assert_eq!(pcm.samples[..], reference.samples[..pcm.samples.len()]);
    assert!(
        pcm.samples.iter().any(|s| s.abs() > 0.8)
            && pcm.samples[..200].iter().all(|s| s.abs() < 0.01),
        "from quiet to loud"
    );
}

#[test]
fn ima_adpcm_agrees_with_ffmpeg_on_a_mono_file() {
    let pcm = decode(&fixture("ima_mono.wav")).unwrap();
    let reference = decode(&fixture("ima_mono.ref.wav")).unwrap();
    assert_eq!((pcm.rate, pcm.channels), (22_050, 1));
    assert_eq!(
        pcm.frames(),
        3308,
        "the `fact` chunk's frame count, not the whole blocks (4082)"
    );
    assert!(reference.frames() >= pcm.frames());
    assert_eq!(
        pcm.samples[..],
        reference.samples[..pcm.samples.len()],
        "sample for sample the same as FFmpeg decoded it"
    );
}

#[test]
fn ima_adpcm_agrees_with_ffmpeg_on_a_stereo_file() {
    let pcm = decode(&fixture("ima_stereo.wav")).unwrap();
    let reference = decode(&fixture("ima_stereo.ref.wav")).unwrap();
    assert_eq!((pcm.rate, pcm.channels), (22_050, 2));
    assert_eq!(pcm.frames(), 3308);
    assert!(reference.frames() >= pcm.frames());
    assert_eq!(pcm.samples[..], reference.samples[..pcm.samples.len()]);
    // The two channels really are different signals (a swapped or doubled channel would pass a mono comparison).
    let left: Vec<f32> = pcm.samples.iter().step_by(2).copied().collect();
    let right: Vec<f32> = pcm.samples.iter().skip(1).step_by(2).copied().collect();
    assert_ne!(left, right);
}

#[test]
fn a_samples_per_block_smaller_than_the_block_holds_limits_each_block() {
    // Block of 8 bytes, mono: 1 + 8 = 9 samples a block; the header says only 5 are used. Two blocks.
    let mut data = vec![0u8; 16];
    data[0..2].copy_from_slice(&500i16.to_le_bytes());
    data[8..10].copy_from_slice(&(-500i16).to_le_bytes());
    data[4..8].fill(0x33);
    data[12..16].fill(0x33);
    let mut file = ima_file(1, 8, &data, None);
    assert_eq!(decode(&file).unwrap().frames(), 18, "as the block holds");
    file[38..40].copy_from_slice(&5u16.to_le_bytes()); // samples per block in the fmt chunk
    let pcm = decode(&file).unwrap();
    assert_eq!(pcm.frames(), 10, "5 from each block");
    assert!(
        close(pcm.samples[5], -500.0 / 32768.0, 1e-9),
        "the second block starts after the fifth sample of the first"
    );
}

#[test]
fn a_short_last_ima_block_is_decoded_as_far_as_it_goes() {
    let whole = fixture("ima_mono.wav");
    let info = probe(&whole).unwrap();
    assert_eq!(info.frames, Some(3308));
    // Without the fact chunk's limit, cut the data inside the second block: header 4 bytes + 3 groups = 1 + 24 samples.
    let mut f = fmt_body(0x11, 1, 22_050, 4);
    f[12..14].copy_from_slice(&1024u16.to_le_bytes());
    f.extend_from_slice(&[2, 0, 0xF9, 0x07]);
    let data_start = whole.windows(4).position(|w| w == b"data").unwrap() + 8;
    let data = &whole[data_start..data_start + 1024 + 4 + 12];
    let file = riff(&[chunk(b"fmt ", &f), chunk(b"data", data)]);
    assert_eq!(decode(&file).unwrap().frames(), 2041 + 1 + 8 * 3);
}

#[test]
fn damaged_ima_blocks_are_refused_or_decoded_never_a_panic() {
    // A block size that cannot hold the headers, or is not whole groups.
    assert!(decode(&ima_file(1, 2, &[0; 8], None)).is_err());
    let mut odd = ima_file(1, 8, &[0; 8], None);
    odd[12 + 8 + 12..12 + 8 + 14].copy_from_slice(&9u16.to_le_bytes());
    assert!(decode(&odd).is_err());
    // A step index above 88 in the header is clamped, not an out-of-range read.
    let mut data = vec![0u8; 12];
    data[2] = 200;
    data[4..].fill(0xFF);
    assert!(decode(&ima_file(1, 12, &data, None)).is_ok());
    // Every prefix and every single-byte damage of a real file is survivable.
    let good = fixture("ima_stereo.wav");
    for cut in (0..good.len()).step_by(37) {
        let _ = decode(&good[..cut]);
    }
    for i in (0..good.len()).step_by(11) {
        let mut damaged = good.clone();
        damaged[i] ^= 0xA5;
        let _ = decode(&damaged);
    }
}

// ---- Vorbis ------------------------------------------------------------------------------------------------------------------------

fn rms_difference(a: &[f32], b: &[f32]) -> f64 {
    let n = a.len().min(b.len());
    (a[..n]
        .iter()
        .zip(&b[..n])
        .map(|(x, y)| f64::from(x - y).powi(2))
        .sum::<f64>()
        / n as f64)
        .sqrt()
}

#[test]
fn vorbis_decodes_to_what_ffmpeg_decodes_it_to_mono() {
    let pcm = decode(&fixture("vorbis_mono.ogg")).unwrap();
    let reference = decode(&fixture("vorbis_mono.ref.wav")).unwrap();
    assert_eq!((pcm.rate, pcm.channels), (44_100, 1));
    assert!(
        pcm.frames().abs_diff(reference.frames()) <= 2048,
        "{} against {}",
        pcm.frames(),
        reference.frames()
    );
    assert!(
        rms_difference(&pcm.samples, &reference.samples) < 0.002,
        "{}",
        rms_difference(&pcm.samples, &reference.samples)
    );
    assert!(
        pcm.samples.iter().any(|s| s.abs() > 0.3),
        "it is not silence"
    );
}

#[test]
fn vorbis_decodes_to_what_ffmpeg_decodes_it_to_stereo() {
    let pcm = decode(&fixture("vorbis_stereo.ogg")).unwrap();
    let reference = decode(&fixture("vorbis_stereo.ref.wav")).unwrap();
    assert_eq!((pcm.rate, pcm.channels), (44_100, 2));
    assert!(pcm.frames().abs_diff(reference.frames()) <= 2048);
    assert!(rms_difference(&pcm.samples, &reference.samples) < 0.002);
    let left: Vec<f32> = pcm.samples.iter().step_by(2).copied().collect();
    let right: Vec<f32> = pcm.samples.iter().skip(1).step_by(2).copied().collect();
    assert!(rms_difference(&left, &right) > 0.05, "the channels differ");
}

#[test]
fn the_stream_gives_the_same_samples_as_the_whole_file_in_pieces() {
    let bytes = fixture("vorbis_stereo.ogg");
    let whole = decode(&bytes).unwrap();
    let mut stream = VorbisStream::new(&bytes).unwrap();
    assert_eq!((stream.rate(), stream.channels()), (44_100, 2));
    let (mut all, mut pieces) = (Vec::new(), 0);
    while let Some(chunk) = stream.read_chunk().unwrap() {
        assert_eq!(chunk.len() % 2, 0, "whole frames");
        all.extend(chunk);
        pieces += 1;
    }
    assert_eq!(all, whole.samples);
    assert!(pieces > 3, "it really came in pieces: {pieces}");
    assert!(
        stream.read_chunk().unwrap().is_none(),
        "and stays at the end"
    );
}

#[test]
fn probe_tells_what_a_file_is_without_decoding_it() {
    let ogg = probe(&fixture("vorbis_stereo.ogg")).unwrap();
    assert_eq!(
        (ogg.container, ogg.codec, ogg.rate, ogg.channels),
        (Container::Ogg, Codec::Vorbis, 44_100, 2)
    );
    let decoded = decode(&fixture("vorbis_stereo.ogg")).unwrap();
    let frames = ogg.frames.expect("the last page says how long it is");
    assert!(
        frames.abs_diff(decoded.frames() as u64) <= 2048,
        "{frames} against {}",
        decoded.frames()
    );
    let ima = probe(&fixture("ima_stereo.wav")).unwrap();
    assert_eq!(
        (ima.container, ima.codec, ima.rate, ima.channels, ima.frames),
        (Container::Wav, Codec::ImaAdpcm, 22_050, 2, Some(3308))
    );
    let pcm = probe(&wav(1, 2, 44_100, 16, &[0; 400])).unwrap();
    assert_eq!((pcm.codec, pcm.frames), (Codec::Pcm16, Some(100)));
    assert_eq!(pcm.decoded_bytes(), Some(400));
    assert!(probe(b"nonsense").is_err());
}

#[test]
fn garbage_and_damage_never_panic_the_vorbis_path() {
    let good = fixture("vorbis_mono.ogg");
    assert!(decode(b"OggS").is_err());
    assert!(decode(b"OggS\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0").is_err());
    for cut in (0..good.len()).step_by(53) {
        let _ = decode(&good[..cut]);
    }
    for i in (0..good.len()).step_by(17) {
        let mut damaged = good.clone();
        damaged[i] ^= 0x5A;
        let _ = decode(&damaged);
    }
    // Headers alone give no audio rather than an error or a hang.
    let end_of_headers = good
        .windows(4)
        .enumerate()
        .filter(|(_, w)| *w == b"OggS")
        .nth(2)
        .map(|(i, _)| i)
        .unwrap();
    let headers_only = decode(&good[..end_of_headers]);
    assert!(headers_only.map_or(true, |p| p.frames() < 4096));
}

#[test]
fn last_granule_reads_the_last_page_and_nothing_from_nonsense() {
    assert_eq!(last_granule(b""), None);
    assert_eq!(last_granule(b"OggS"), None);
    let mut page = vec![0u8; 27];
    page[..4].copy_from_slice(b"OggS");
    page[6..14].copy_from_slice(&12_345i64.to_le_bytes());
    let mut two = page.clone();
    two.extend_from_slice(&page);
    let n = two.len();
    two[n - 27 + 6..n - 27 + 14].copy_from_slice(&99_999i64.to_le_bytes());
    assert_eq!(
        last_granule(&two),
        Some(99_999),
        "the last page, not the first"
    );
    two[n - 27 + 6..n - 27 + 14].copy_from_slice(&(-1i64).to_le_bytes());
    assert_eq!(
        last_granule(&two),
        Some(12_345),
        "a page without a granule position (-1) is skipped"
    );
    // A granule position of 0 is a position (the header pages have it); a page of another version is not a page of this format.
    let mut zero = page.clone();
    zero[6..14].copy_from_slice(&0i64.to_le_bytes());
    assert_eq!(last_granule(&zero), Some(0));
    let mut other_version = page.clone();
    other_version.extend_from_slice(&page);
    let n = other_version.len();
    other_version[n - 27 + 4] = 1;
    other_version[n - 27 + 6..n - 27 + 14].copy_from_slice(&777i64.to_le_bytes());
    assert_eq!(
        last_granule(&other_version),
        Some(12_345),
        "a page of version 1 is skipped"
    );
}

// ---- the choice between decoded and streamed -------------------------------------------------------------------------------------------

fn info(frames: Option<u64>, channels: u16) -> StreamInfo {
    StreamInfo {
        container: Container::Ogg,
        codec: Codec::Vorbis,
        rate: 44_100,
        channels,
        frames,
    }
}

#[test]
fn short_sounds_are_decoded_and_long_ones_streamed() {
    let policy = DeliveryPolicy::default();
    // 4 MiB of 16-bit mono is 2,097,152 frames.
    assert_eq!(
        policy.delivery(&info(Some(2_097_152), 1), 100_000),
        Delivery::Decoded,
        "exactly at the limit is still decoded"
    );
    assert_eq!(
        policy.delivery(&info(Some(2_097_153), 1), 100_000),
        Delivery::Streamed,
        "one frame over"
    );
    assert_eq!(
        policy.delivery(&info(Some(1_048_576), 2), 100_000),
        Delivery::Decoded,
        "the same size in stereo is half the frames"
    );
    assert_eq!(
        policy.delivery(&info(Some(1_048_577), 2), 100_000),
        Delivery::Streamed
    );
    assert_eq!(
        policy.delivery(&info(Some(44_100 * 2), 1), 9_000),
        Delivery::Decoded,
        "a gunshot"
    );
    assert_eq!(
        policy.delivery(&info(Some(44_100 * 120), 2), 2_000_000),
        Delivery::Streamed,
        "two minutes of music"
    );
}

#[test]
fn an_unknown_length_goes_by_the_size_of_the_file() {
    let policy = DeliveryPolicy::default();
    assert_eq!(
        policy.delivery(&info(None, 1), 256 * 1024),
        Delivery::Decoded,
        "exactly at the limit"
    );
    assert_eq!(
        policy.delivery(&info(None, 1), 256 * 1024 + 1),
        Delivery::Streamed
    );
    let tight = DeliveryPolicy {
        stream_above_decoded_bytes: 1000,
        stream_above_file_bytes: 10,
    };
    assert_eq!(
        tight.delivery(&info(Some(500), 1), 1_000_000),
        Delivery::Decoded,
        "1000 bytes decoded is at the limit"
    );
    assert_eq!(tight.delivery(&info(Some(501), 1), 1), Delivery::Streamed);
}

#[test]
fn the_games_own_sounds_fall_on_the_right_side() {
    // docs/re/audio.md: 317 of the 392 Ogg files are under 100 KB (effects and voices), the rest 1.4 to 2.4 MB (music, a minute or more).
    let policy = DeliveryPolicy::default();
    let effect = info(Some(44_100 * 5), 1);
    let music = StreamInfo {
        channels: 2,
        ..info(Some(44_100 * 90), 2)
    };
    assert_eq!(policy.delivery(&effect, 90_000), Delivery::Decoded);
    assert_eq!(policy.delivery(&music, 2_000_000), Delivery::Streamed);
}

#[test]
fn the_frame_limit_stops_a_file_that_would_fill_memory() {
    // 8-bit mono: one frame a byte, so a data chunk one byte over the limit is one frame over it. The file is 79 MB, the decode allocates nothing.
    let too_long = wav(1, 1, 44_100, 8, &vec![128u8; MAX_FRAMES as usize + 1]);
    let err = decode(&too_long).expect_err("longer than the limit");
    assert!(err.to_string().contains("limit"), "{err}");
    assert!(
        probe(&too_long).is_ok(),
        "probing reads no samples, so it can still say how long the file is"
    );
}

/// An Ogg file larger than this is music (docs/re/audio.md: effects and voices are under 100 KB, music 1.4 to 2.4 MB).
const MUSIC_BYTES: usize = 400_000;

// ---- the game's own files (needs the install) ---------------------------------------------------------------------------------------

#[test]
fn every_sound_of_the_real_install_decodes_and_is_what_the_spec_says() {
    use ss2_vfs::{EntryKind, Vfs, VfsPath};
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let mut stack = vec![VfsPath::root()];
    let (mut wavs, mut oggs, mut frames_total) = (0usize, 0usize, 0u64);
    let (mut stereo_wav, mut stereo_ogg, mut streamed, mut music) =
        (0usize, 0usize, 0usize, 0usize);
    let mut music_streamed = 0usize;
    let started = std::time::Instant::now();
    let mut failures = Vec::new();
    let policy = DeliveryPolicy::default();
    while let Some(d) = stack.pop() {
        for e in vfs.list(&d).unwrap() {
            let path = d.join(&e.name).unwrap();
            if e.kind == EntryKind::Dir {
                stack.push(path);
                continue;
            }
            let is_wav = matches!(path.extension(), Some("wav"));
            let is_ogg = matches!(path.extension(), Some("ogg"));
            if !is_wav && !is_ogg {
                continue;
            }
            let bytes = vfs.read(&path).unwrap();
            let probed = probe(&bytes);
            if is_ogg && bytes.len() > MUSIC_BYTES {
                // Music is 1.4 to 2.4 MB of Vorbis: reading the first part (its headers and a few seconds of audio) is enough in a debug build, and
                // the whole of it is decoded by the release-build check in docs/AUDIO_DECODERS.md.
                let info = probed.as_ref().unwrap();
                let mut stream = match VorbisStream::new(&bytes) {
                    Ok(stream) => stream,
                    Err(e) => {
                        failures.push(format!("{path} ({} bytes): {e}", bytes.len()));
                        continue;
                    }
                };
                let mut frames = 0usize;
                // SS2_AUDIO_FULL=1 reads to the end (a release build does the 75 files in under a minute).
                let full = std::env::var_os("SS2_AUDIO_FULL").is_some();
                for _ in 0..(if full { usize::MAX } else { 300 }) {
                    match stream.read_chunk() {
                        Ok(Some(chunk)) => frames += chunk.len() / usize::from(stream.channels()),
                        Ok(None) => break,
                        Err(e) => {
                            failures.push(format!("{path}: {e}"));
                            break;
                        }
                    }
                }
                assert!(
                    frames > 20_000,
                    "{path}: only {frames} frames from the first 300 packets"
                );
                if full {
                    let said = info.frames.expect("the last page says how long it is");
                    assert!(
                        said.abs_diff(frames as u64) <= 4096,
                        "{path}: the last page says {said} frames, {frames} were decoded"
                    );
                }
                assert_eq!(info.rate, 44_100);
                if policy.delivery(info, bytes.len() as u64) == Delivery::Streamed {
                    music_streamed += 1;
                }
                music += 1;
                continue;
            }
            match decode(&bytes) {
                Ok(pcm) => {
                    frames_total += pcm.frames() as u64;
                    let info = probed.unwrap();
                    if info.frames.is_some() && is_wav {
                        assert_eq!(info.frames, Some(pcm.frames() as u64), "{path}");
                    }
                    if policy.delivery(&info, bytes.len() as u64) == Delivery::Streamed {
                        streamed += 1;
                    }
                    if is_wav {
                        wavs += 1;
                        if pcm.channels == 2 {
                            stereo_wav += 1;
                        }
                        assert_eq!(
                            info.codec,
                            Codec::Pcm16,
                            "{path}: the spec says every WAV is 16-bit PCM"
                        );
                    } else {
                        oggs += 1;
                        if pcm.channels == 2 {
                            stereo_ogg += 1;
                        }
                        assert_eq!(
                            pcm.rate, 44_100,
                            "{path}: the spec says every Ogg is 44.1 kHz"
                        );
                    }
                    assert!(pcm.samples.iter().all(|s| s.is_finite()), "{path}");
                }
                Err(e) => failures.push(format!("{path}: {e}")),
            }
        }
    }
    eprintln!(
        "decoded {wavs} wav ({stereo_wav} stereo) and {oggs} small ogg ({stereo_ogg} stereo), {frames_total} frames in all; the start of {music} large ogg files ({music_streamed} of them would be streamed); {streamed} smaller ones would be streamed; {:.1} s",
        started.elapsed().as_secs_f64()
    );
    assert!(
        failures.is_empty(),
        "{} sounds failed, first few: {:?}",
        failures.len(),
        &failures[..failures.len().min(5)]
    );
    assert!(wavs > 0 && oggs > 0);
}
