# T06 — Audio & Video (`ss2-audio`, `ss2-video`)

Install facts: 1,874 `.wav` (330 MB), 392 `.ogg` (164 MB), 99 `.ogv` Theora videos (1.3 GB), `.scs`
sound collections, original used `amp11lib`/OGG/WMX codecs. Stereo + 3D positional sound required.
PC first; mobile audio/memory work is Phase 2.

## Audio engine (Wave 0)
- [ ] **T06-001** · Output device + mixer thread (`cpal`): sample-rate conversion, buffer sizing, device change/loss recovery, headless null device for servers/tests · `L` · `P0`
- [x] **T06-002** · Decoders: PCM/IMA-ADPCM `.wav`, Vorbis `.ogg` (symphonia/lewton), streaming vs fully-decoded policy by size: `ss2_audio::decode` (`docs/AUDIO_DECODERS.md`): PCM 8/16/24/32, float 32/64 and IMA ADPCM WAV (also extensible), Ogg Vorbis through `lewton` (whole or a packet at a time), `probe` for the length without decoding, the decoded-or-streamed rule; agrees sample for sample with FFmpeg on IMA ADPCM (all 89 step indexes) and to 0.002 RMS on Vorbis, and decodes all 1,868 WAV and 392 Ogg of the real install; **the policy is only the decision (the streaming playback is the mixer's), no seeking, nothing calls it yet** (T06-002b) · `M` · `P0` · 🟢
- [ ] **T06-002b** · Use the decoders: a sound bank that loads by path through `decode` and applies `DeliveryPolicy` (T06-017b), a streaming voice in the mixer fed by `VorbisStream` (T06-005), Vorbis seeking for loop points and prefetch (T06-009), sample-rate conversion to the device rate (T06-001), Microsoft ADPCM and A-law if a mod needs them · `M` · `P0` · Deps: T06-002
- [x] **T06-013** · Offline renderer: run the mixer to a WAV buffer deterministically; golden-audio tests with tolerance: `ss2_audio::{render_offline, wav, compare}` (`docs/MIXER.md`): deterministic rendering, WAV writing, a peak and RMS tolerance, an analytic golden test; **no stored golden files yet** (T06-013b) · `M` · `P1`
- [ ] **T06-013b** · Stored golden-audio files for scenes without a closed form, and a cross-architecture check of the tolerance (with T07-026) · `S` · `P1` · Deps: T06-013
- [ ] **T06-003** · 3D positional audio: distance attenuation curves (from T01-034), panning, occlusion low-pass, Doppler, listener orientation · `L` · `P0`
- [x] **T06-006** · Voice management: priority, max voices per tier (mobile), virtualization/culling of inaudible voices, bus ducking: in the mixer core (`docs/MIXER.md`): priority, then loudness, then age; virtualized voices keep their place; critical voices never culled; music ducking under voice-overs; **the limit is a parameter, nothing sets it from a quality tier, and nothing was measured on a phone** (T06-006b) · `M` · `P1`
- [ ] **T06-006b** · Wire the voice limit to quality tiers (mobile), measure the mixer on phone CPUs (Phase 2), per-bus voice caps · `S` · `P1` · Deps: T06-006, T04-020
- [ ] **T06-007** · Environment reverb (EAX-style zones → simple FDN/Schroeder reverb), per-zone presets, mobile toggle · `L` · `P2`
- [ ] **T06-009** · Streaming from archives without full load; prefetch via `.preload` lists; seek support · `M` · `P1`

## Content behavior (needs specs)
- [ ] **T06-004** · Sound collections (`.scs`): random variants, pitch/volume jitter, material-based impact/footstep selection · `M` · `P1` · Deps: T01-013
- [ ] **T06-005** · Music system: streaming playback, crossfades, level-state-driven layers/cues, loop points · `L` · `P1`
- [ ] **T06-016** · Weapon/enemy audio event hooks from animation events (fire, reload, step) · `M` · `P1` · Deps: T02-016
- [x] **T06-015** · Replicated audio events API for co-op (who hears what; one-shots vs loops; late-joiner behavior): `ss2_audio_api::replication` (`docs/AUDIO_REPLICATION.md`): `AudioEvent` with a validated byte form, `AudioReplicator` (scopes, earshot, loops as state with hysteresis, late joiners get the loops in range and no one-shots) and `AudioEventPlayer`; **no transport sends it, nothing produces the events, no per-tick emitter updates** (T06-015b) · `M` · `P1` · Deps: T08-012
- [ ] **T06-015b** · Use the replicated audio events: send them over the transport (reliable and ordered for loops, unreliable for one-shots; a periodic refresh of each client's known loops), build the `SoundRef` table from the level's sound list, call the replicator from weapons, enemies and ambient emitters, per-tick emitter updates, occlusion and congestion priorities · `M` · `P1` · Deps: T06-015, T08-007
- [x] **T06-017** · Mixer buses & volume settings (master/music/sfx/voice/UI), mute on focus loss/Android pause: in the mixer core (`docs/MIXER.md`): bus volumes with master, clamping, ducking, a suspend that fades without dropping voices; **no `impl Audio` for the mixer, no settings screen or file, no Android pause wiring** (T06-017b) · `S` · `P1` · 🟢
- [ ] **T06-017b** · `impl Audio` for the mixer (sound resolution by path through the sound bank), wire `set_suspended` to the platform lifecycle and the volumes to the settings file and screen (T10-012) · `S` · `P1` · Deps: T06-017, T06-002
- [ ] **T06-018** · Audio API contract mocks and sound-test scene in asset viewer · `S` · `P1` · 🟢

## PHASE 2 — Mobile
- [ ] **T06-008** · Android: low-latency path (AAudio/Oboe via cpal), audio focus handling, Bluetooth latency compensation · `M` · `PHASE 2`
- [ ] **T06-019** · Memory-lean mode: compressed-in-RAM voices, streaming for long SFX · `M` · `PHASE 2`

## Video / cutscenes
- [ ] **T06-010** · `.ogv` Theora decode (pure-Rust or vetted binding) with frame queue → GPU texture upload (YUV→RGB in shader), A/V sync to audio clock · `L` · `P1`
- [ ] **T06-011** · Cutscene player: fullscreen/letterboxed playback, skip/pause rules, subtitle track, resume on Android lifecycle · `M` · `P2`
- [x] **T06-012** · Subtitles/captions overlay timing (source per T01-041): `ss2_ui::subtitles` (`docs/SUBTITLES.md`): a device-free cue queue with reading-time-based minimum durations, fades, a line limit with priority eviction and bounded waiting, late/out-of-order cues, pause, seek and skip; scope: timing model only, nothing draws it and no audio clock or subtitle data is connected (T06-012b) · `M` · `P2`
- [ ] **T06-012b** · Draw the subtitle overlay (text, outline, box and wrapping on the HUD skin using `AccessibilitySettings::subtitle_style`), drive it from the real audio/video clock (T06-010, T06-011), and read cues from the localization tables and cutscene data per T01-041 (T10-011); tune the defaults on a screen · `M` · `P2`
- [ ] **T06-020** · Large-video storage on Android (stream from SAF/content URI without copying) · `M` · `PHASE 2`
- [ ] **T06-021** · `.xmv` (Xbox) explicitly ignored; document + test that it's skipped · `S` · `P3` · 🟢
- [ ] **T06-014** · Voice chat for co-op (Opus, push-to-talk, VAD), optional · `L` · `P3`
