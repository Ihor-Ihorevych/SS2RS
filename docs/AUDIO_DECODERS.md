# Sound decoders

`ss2_audio::decode` (T06-002) turns a `.wav` or an Ogg Vorbis file into interleaved `f32` samples (`Pcm { rate, channels, samples }`), says what a file is without decoding it (`probe`), decodes Vorbis a piece at a time (`VorbisStream`), and decides what to keep whole and what to stream (`DeliveryPolicy`). It is written from the public RIFF/WAVE, IMA ADPCM and Vorbis descriptions; the game's own sound files (`docs/re/audio.md`: every `.wav` is 16-bit PCM, every Ogg is 44.1 kHz Vorbis) are test cases, not the source.

## What it reads

**WAV** (RIFF, little-endian): PCM 8 bit (unsigned), 16, 24 and 32 bit, IEEE float 32 and 64 bit, and **IMA ADPCM** (4 bits a sample in blocks), each also in the `WAVE_FORMAT_EXTENSIBLE` form. Not read, and refused with a message that names the format tag: Microsoft ADPCM, A-law, µ-law, MP3 in a WAV, 12- or 20-bit "PCM". Chunks are walked, not assumed at fixed offsets: any order, unknown ones skipped, odd lengths padded; a `data` chunk that claims more than the file holds (a cut file, or `0xFFFFFFFF` from a writer that streams) is the rest of the file, and a half sample at the end is dropped. IMA's `fact` chunk trims the last block's padding (a decoder that ignores it returns 774 more samples than the 3,308 the sound has). Samples are scaled by 1/32768 (so 16-bit −32768 is exactly −1), and a float that is NaN or infinite is silence.

**Ogg Vorbis** through the `lewton` crate (pure Rust, safe, MIT or Apache-2.0; it is the one new dependency, and `cargo deny` accepts it). Mono and stereo to 8 channels; a panic inside the decoder is caught and becomes an error, as a file from a mod or a damaged archive must not crash the game.

**Limits** against a file that would fill memory or lie about itself: at most 8 channels, a rate of at most 384 kHz, and at most `MAX_FRAMES` (half an hour at 44.1 kHz) per sound, checked **before** anything is allocated for the samples.

## `probe`, `VorbisStream`, `DeliveryPolicy`

- `probe(bytes)` reads only the headers: container, codec, rate, channels and the **number of frames** (for a WAV from the chunk sizes and `fact`; for an Ogg from the granule position of its last page). It does not decode, so a loader can decide before it pays.
- `VorbisStream::new(bytes)` and `read_chunk()` give the samples a packet at a time (a few hundred to a few thousand frames), for music that must not be held whole: 75 of the game's 392 Ogg files are 1.4 to 2.4 MB, and decoded to `f32` each would be 25 to 40 MB. The pieces joined are exactly what `decode` gives (tested).
- `DeliveryPolicy::delivery(&info, file_len)`: **`Streamed`** when the sound decoded to 16 bits would be more than 4 MiB (about 47 seconds of mono at 44.1 kHz; exactly at the limit is still decoded), or, when the length is unknown, when the file is larger than 256 KiB; otherwise **`Decoded`**. Both limits are fields to change. This is only the decision: the code that streams while playing is the mixer's (T06-005, T06-009).

## How it was tested

Thirty-nine tests, four kinds:

- **Built by hand, one for each way a file differs:** every PCM width and the floats with exact expected values (including the sign extension of 24 bit and NaN), stereo interleaving, the extensible form giving the same samples as the plain one and an unregistered sub-format refused, chunks in any order with padding, the over-long and cut `data` chunk, a zero-length one, impossible headers (0 or 9 channels, rate 0 or too high, 12-bit, Microsoft ADPCM, A-law, MP3), a file one frame over the limit (79 MB, refused before allocating), and **every prefix and a single-byte damage of real files** (never a panic).
- **Against another decoder.** Small synthetic fixtures in `testdata/audio/` (tones and noise made with FFmpeg, so there is no one else's sound in the repository; the commands are below) were decoded by FFmpeg too, and `decode` must agree: **IMA ADPCM sample for sample** (mono, stereo, and a noise sweep from near silence to full scale whose blocks visit **all 89 step indexes**, so a wrong entry anywhere in the step table changes a sample), and Vorbis to within an RMS difference of 0.002 (the two are float decoders, so not bit-exact), for mono and stereo.
- **The game's own files** (data-gated, `SS2_DATA_DIR`): all **1,868 `.wav`** (all 16-bit PCM, 7 stereo: the numbers of the spec) decode whole, and `probe` says the length that the decode finds; all **317 small and 75 large `.ogg`** (392: the spec's count) open at 44.1 kHz. A debug build decodes the small ones whole and the first 300 packets of the large ones (in about 140 seconds); `SS2_AUDIO_FULL=1 cargo test --release -p ss2-audio --lib every_sound` decodes the large ones to the end and checks the length against the last page. **The release run was done once, here: all of it, the 75 large files to the end with the length checked against the last page, in 67 seconds (most of it reading the archives).**
- **Mutation:** twenty-nine deliberate breakages (a scale, a sign extension, the chunk padding, the data length, the GUID check, the channel and rate limits, every part of the IMA step, the nibble order, the channel layout, the clamps, the `fact` trim and the partial last block, the granule scan, the policy edges, the frame limit). **Twenty-four failed a test at once; four survived my first tests**: the rate limit's exact edge, a granule position of 0, an Ogg page of another version, and a 32-bit scale of 2^31 against 2^31−1 (**equivalent**: the two differ by less than an `f32` can hold, so I left it); the first three are covered now, and a `samples_per_block` smaller than the block holds, which I had not tested, has a test too. Every one of the others was killed by the final tests.

## Making the fixtures

```
# tone and noise sources, 0.15 s, 22.05 kHz (the Vorbis ones 44.1 kHz); the reference decodes are s16 WAVs
ffmpeg -f lavfi -i "aevalsrc=0.45*sin(2*PI*440*t)+0.25*sin(2*PI*1320*t)*sin(2*PI*3*t):s=22050:d=0.15" -c:a adpcm_ima_wav ima_mono.wav
ffmpeg -f lavfi -i "aevalsrc='(random(0)*2-1)*0.95*exp(-11*(1-t/0.3)):s=22050:d=0.3'" -c:a adpcm_ima_wav -block_size 256 ima_sweep.wav
ffmpeg -i ima_mono.wav -c:a pcm_s16le ima_mono.ref.wav
ffmpeg -f lavfi -i "<the same source>" -ar 44100 -c:a libvorbis -q:a 3 vorbis_mono.ogg ; ffmpeg -i vorbis_mono.ogg -c:a pcm_s16le vorbis_mono.ref.wav
```

(FFmpeg 9.0.1; the stereo ones use `|` between two channel expressions. The files are under 30 KB each, 100 KB in all, and pass `cargo xtask guard`.)

## Not covered (follow-up T06-002b)

- **Streaming WAV** and **seeking** in Vorbis (T06-009's prefetch and loop points need `seek_absgp_pg`); only Vorbis has a piece-at-a-time reader today.
- **Nothing calls it yet**: the mixer's sample buffers are filled by hand and the sound bank (T06-017b) will load by path through this.
- **Other codecs** (Microsoft ADPCM, A-law, µ-law) are refused; the game does not use them, a mod might.
- **Sample-rate conversion** (a 22.05 kHz sound to a 48 kHz device) belongs to the mixer and the output device (T06-001).
- **Vorbis quality**: `lewton` is not bit-exact with libvorbis (about 1e-3 RMS in the fixtures) and is slower than a C decoder; a release build decodes the largest music file in a fraction of a second, a debug build many times slower.
- **Chained Ogg streams and non-Vorbis Ogg** (Opus, FLAC) are not read.
