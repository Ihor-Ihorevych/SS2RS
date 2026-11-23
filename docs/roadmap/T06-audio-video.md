# T06 — Audio & Video (`ss2-audio`, `ss2-video`)

Install facts: 1,874 `.wav` (330 MB), 392 `.ogg` (164 MB), 99 `.ogv` Theora videos (1.3 GB), `.scs`
sound collections, original used `amp11lib`/OGG/WMX codecs. Stereo + 3D positional sound required.
PC first; mobile audio/memory work is Phase 2.

## Audio engine (Wave 0)
- [ ] **T06-001** · Output device + mixer thread (`cpal`): sample-rate conversion, buffer sizing, device change/loss recovery, headless null device for servers/tests · `L` · `P0`
- [ ] **T06-002** · Decoders: PCM/IMA-ADPCM `.wav`, Vorbis `.ogg` (symphonia/lewton), streaming vs fully-decoded policy by size · `M` · `P0` · 🟢
- [~] **T06-013** [INPRG @Ihor-Ihorevych/agent · branch `T06-mixer-core` · 2026-11-23] · Offline renderer: run the mixer to a WAV buffer deterministically; golden-audio tests with tolerance · `M` · `P1`
- [ ] **T06-003** · 3D positional audio: distance attenuation curves (from T01-034), panning, occlusion low-pass, Doppler, listener orientation · `L` · `P0`
- [ ] **T06-006** · Voice management: priority, max voices per tier (mobile), virtualization/culling of inaudible voices, bus ducking · `M` · `P1`
- [ ] **T06-007** · Environment reverb (EAX-style zones → simple FDN/Schroeder reverb), per-zone presets, mobile toggle · `L` · `P2`
- [ ] **T06-009** · Streaming from archives without full load; prefetch via `.preload` lists; seek support · `M` · `P1`

## Content behavior (needs specs)
- [ ] **T06-004** · Sound collections (`.scs`): random variants, pitch/volume jitter, material-based impact/footstep selection · `M` · `P1` · Deps: T01-013
- [ ] **T06-005** · Music system: streaming playback, crossfades, level-state-driven layers/cues, loop points · `L` · `P1`
- [ ] **T06-016** · Weapon/enemy audio event hooks from animation events (fire, reload, step) · `M` · `P1` · Deps: T02-016
- [ ] **T06-015** · Replicated audio events API for co-op (who hears what; one-shots vs loops; late-joiner behavior) · `M` · `P1` · Deps: T08-012
- [ ] **T06-017** · Mixer buses & volume settings (master/music/sfx/voice/UI), mute on focus loss/Android pause · `S` · `P1` · 🟢
- [ ] **T06-018** · Audio API contract mocks and sound-test scene in asset viewer · `S` · `P1` · 🟢

## PHASE 2 — Mobile
- [ ] **T06-008** · Android: low-latency path (AAudio/Oboe via cpal), audio focus handling, Bluetooth latency compensation · `M` · `PHASE 2`
- [ ] **T06-019** · Memory-lean mode: compressed-in-RAM voices, streaming for long SFX · `M` · `PHASE 2`

## Video / cutscenes
- [ ] **T06-010** · `.ogv` Theora decode (pure-Rust or vetted binding) with frame queue → GPU texture upload (YUV→RGB in shader), A/V sync to audio clock · `L` · `P1`
- [ ] **T06-011** · Cutscene player: fullscreen/letterboxed playback, skip/pause rules, subtitle track, resume on Android lifecycle · `M` · `P2`
- [ ] **T06-012** · Subtitles/captions overlay timing (source per T01-041) · `M` · `P2`
- [ ] **T06-020** · Large-video storage on Android (stream from SAF/content URI without copying) · `M` · `PHASE 2`
- [ ] **T06-021** · `.xmv` (Xbox) explicitly ignored; document + test that it's skipped · `S` · `P3` · 🟢
- [ ] **T06-014** · Voice chat for co-op (Opus, push-to-talk, VAD), optional · `L` · `P3`
