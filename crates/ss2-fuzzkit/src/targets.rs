//! The fuzz properties: one public function per target, each taking the raw input of the fuzzer.
//!
//! The rules every function here follows:
//!
//! - It looks at no more than [`MAX_INPUT`] bytes.
//! - It never returns a verdict: it **panics** when a property does not hold (or when the parser under test panics), which
//!   is what libFuzzer and the test harness both treat as a failure.
//! - After a successful parse it uses the result (walks it, samples it, writes it back) because a value that reads but
//!   cannot be used is a parser bug too.

use std::borrow::Cow;

use arbitrary::Unstructured;
use ss2_meta::Container;

use crate::MAX_INPUT;

fn cap(data: &[u8]) -> &[u8] {
    &data[..data.len().min(MAX_INPUT)]
}

fn text(data: &[u8]) -> Cow<'_, str> {
    String::from_utf8_lossy(cap(data))
}

// ---------------------------------------------------------------------------------------------------------------------
// ss2-meta and the format crates (all of them read CTSEMETA containers)
// ---------------------------------------------------------------------------------------------------------------------

/// `ss2-meta`: the container frame, the type system and the object decoder; what decodes and writes is read back
/// identically and written again to the same bytes.
pub fn meta_container(data: &[u8]) {
    let data = cap(data);
    let Ok(container) = Container::parse(data) else {
        return;
    };
    let _ = container.info_mismatches();
    for id in 0..container.types.len() as u32 + 2 {
        let _ = container.type_def(id);
        let _ = container.type_name(id);
        let _ = container.ident(id);
        let _ = container.external_type_name(id);
    }
    for section in &container.sections {
        let _ = container.section(&section.tag);
    }
    let Ok(decoded) = container.decode_objects() else {
        return;
    };
    let Ok(written) = ss2_meta::write(&container, &decoded) else {
        return;
    };
    let again = Container::parse(&written).expect("a file the writer wrote must parse");
    let decoded_again = again
        .decode_objects()
        .expect("a file the writer wrote must decode");
    // `Debug` rather than `==`: NaN is a legal float in a file and is not equal to itself.
    assert_eq!(
        format!("{:?}", decoded.objects),
        format!("{:?}", decoded_again.objects),
        "the objects changed through write and read"
    );
    assert_eq!(
        format!("{:?}", decoded.extra),
        format!("{:?}", decoded_again.extra),
        "the extra records changed through write and read"
    );
    let written_again = ss2_meta::write(&again, &decoded_again).expect("a file read back writes");
    assert_eq!(
        written, written_again,
        "writing is not stable: write(read(write(x))) differs from write(x)"
    );
}

/// `ss2-format-tex`: whatever reads has a pixel buffer that matches its size, format and mip chain exactly.
pub fn format_tex(data: &[u8]) {
    let Ok(t) = ss2_format_tex::Texture::from_bytes(cap(data)) else {
        return;
    };
    assert!(t.faces == 1 || t.faces == 6);
    assert_eq!(t.data.len() % t.faces as usize, 0);
    let mut total = 0usize;
    for face in 0..t.faces {
        for level in 0..t.mip_levels {
            let l = t
                .level(face, level)
                .expect("every level of a read texture exists");
            assert_eq!(l.data.len() as u64, t.format.level_bytes(l.width, l.height));
            assert!(l.width >= 1 && l.height >= 1);
            total += l.data.len();
        }
    }
    assert_eq!(
        total,
        t.data.len(),
        "the levels do not add up to the pixel buffer"
    );
    assert!(t.level(t.faces, 0).is_none());
    assert!(t.level(0, t.mip_levels).is_none());
}

/// `ss2-format-bmf`: whatever reads has every stream the length of its vertex count and every index inside the vertices.
pub fn format_bmf(data: &[u8]) {
    let Ok(mesh) = ss2_format_bmf::Mesh::from_bytes(cap(data)) else {
        return;
    };
    for lod in &mesh.lods {
        let _ = (lod.is_for_pc(), lod.triangle_count());
        for s in &lod.surfaces {
            let n = s.positions.len();
            assert_eq!(s.indices.len() % 3, 0, "indices are whole triangles");
            assert!(
                s.indices.iter().all(|&i| usize::from(i) < n),
                "an index outside the vertices"
            );
            assert_eq!(s.normals.len(), n);
            assert_eq!(s.uv_sets.len(), s.uv_map_ids.len());
            assert!(s.uv_sets.iter().all(|u| u.len() == n));
            if let Some(t) = &s.tangents {
                assert_eq!(t.len(), n);
            }
            assert_eq!(s.bone_weights.is_some(), s.bone_indices.is_some());
            if let Some(w) = &s.bone_weights {
                assert_eq!(w.len(), n);
            }
            assert_eq!(s.is_skinned(), s.bone_weights.is_some());
        }
    }
}

/// `ss2-format-mdl`: whatever reads has the materials its embedded surfaces name, each once and in id order, and
/// meshes that keep the `format_bmf` property.
pub fn format_mdl(data: &[u8]) {
    use ss2_format_bmf::MaterialRef;
    use ss2_format_mdl::{MeshSource, Model};
    let Ok(model) = Model::from_bytes(cap(data)) else {
        return;
    };
    assert!(
        model.materials.windows(2).all(|w| w[0].id < w[1].id),
        "materials are unique and sorted by id"
    );
    let _ = model.mesh();
    for source in &model.meshes {
        let MeshSource::Embedded(mesh) = source else {
            continue;
        };
        for s in mesh.lods.iter().flat_map(|l| &l.surfaces) {
            let n = s.positions.len();
            assert!(s.indices.iter().all(|&i| usize::from(i) < n));
            assert_eq!(s.normals.len(), n);
            if let MaterialRef::Embedded(id) = s.material {
                assert!(
                    model.material(id).is_some(),
                    "an embedded material is listed"
                );
                assert!(model.material_of(s).is_some());
            }
        }
    }
}

/// `ss2-format-ans`: whatever reads can be sampled at any time in both loop modes, and the poses are parallel to the tracks.
pub fn format_ans(data: &[u8]) {
    use ss2_format_ans::LoopMode;
    let Ok(set) = ss2_format_ans::AnimSet::from_bytes(cap(data)) else {
        return;
    };
    for a in &set.animations {
        let _ = set.animation(&a.name);
        let span = a.span_seconds();
        let times = [
            0.0,
            span * 0.5,
            span,
            span * 3.7,
            1.0e9,
            -1.0,
            f64::NAN,
            f64::INFINITY,
        ];
        for mode in [LoopMode::Once, LoopMode::Loop] {
            for &t in &times {
                let p = a.sample(t, mode);
                assert_eq!(p.translations.len(), a.translations.len());
                assert_eq!(p.rotations.len(), a.rotations.len());
                assert_eq!(p.scalars.len(), a.scalars.len());
            }
            let _ = a.events_between(0.0, span, mode);
            let _ = a.events_between(span * 0.25, 1.0e9, mode);
            for track in 0..=a.translations.len() {
                let _ = a.translation_delta(track, 0.0, span * 2.5, mode);
            }
        }
        let _ = a.sample_frame(a.first_frame as f32 + 0.5);
        let _ = (
            a.translation_index("x"),
            a.rotation_index("x"),
            a.scalar_index("x"),
        );
    }
}

/// `ss2-format-ep`: the generic parameter tree and the entity parameter file on top of it.
pub fn format_ep(data: &[u8]) {
    let data = cap(data);
    if let Ok(tree) = ss2_format_ep::ParamTree::from_bytes(data, ".ep fuzz") {
        let _ = tree.referenced_files();
        let _ = tree.root.find("x");
        let _ = tree.root.get("a", "b");
    }
    if let Ok(ep) = ss2_format_ep::EntityParams::from_bytes(data) {
        let _ = ep.root_class();
        let _ = ep.referenced_files();
        assert!(ep.class_chain.iter().any(|c| c == "CEntityParams"));
    }
}

/// `ss2-format-fnt`: whatever reads can lay out any text, with every glyph looked up inside the table.
pub fn format_fnt(data: &[u8]) {
    let Ok(font) = ss2_format_fnt::Font::from_bytes(cap(data)) else {
        return;
    };
    assert!(!font.glyphs.is_empty());
    for g in &font.glyphs {
        let _ = font.uv(g);
    }
    for code in [0u32, 32, 65, 66, 0x2122, u32::MAX] {
        let _ = font.glyph(code);
        let _ = font.has_glyph(code);
        assert!(font.glyph_index(code) < font.glyphs.len());
    }
    for s in ["", "AB A", "A\nB", "\u{2122}\u{0}z"] {
        let _ = font.layout(s);
        let _ = font.text_width(s);
        let _ = font.text_height(s);
    }
}

/// `ss2-format-fx`: the four effect formats on the same input (`.pfx`, `.lfx`, `.tfx`, `.vfx`).
pub fn format_fx(data: &[u8]) {
    use ss2_format_fx::{lfx, pfx, tfx, vfx};
    let data = cap(data);
    if let Ok(e) = pfx::Effect::from_bytes(data) {
        let _ = e.referenced_files();
        let _ = e.graph_holders();
    }
    if let Ok(e) = lfx::FlareConfiguration::from_bytes(data) {
        let _ = e.referenced_files();
    }
    if let Ok(e) = tfx::TextEffectConfiguration::from_bytes(data) {
        let _ = e.referenced_files();
    }
    if let Ok(e) = vfx::VibrationEffect::from_bytes(data) {
        let _ = e.referenced_files();
    }
}

/// `ss2-format-mat`: physical materials, sound schemes and particle schemes on the same input.
pub fn format_mat(data: &[u8]) {
    use ss2_format_mat::{ParticleScheme, PhysicalMaterial, SoundScheme};
    let data = cap(data);
    if let Ok(s) = SoundScheme::from_bytes(data) {
        let _ = s.event("Fire");
    }
    if let Ok(s) = ParticleScheme::from_bytes(data) {
        let _ = s.event("BulletHit");
    }
    let _ = PhysicalMaterial::from_bytes(data);
}

/// `ss2-format-mtr`: whatever reads can be walked.
pub fn format_mtr(data: &[u8]) {
    let Ok(file) = ss2_format_mtr::MaterialFile::from_bytes(cap(data)) else {
        return;
    };
    for m in &file.presets {
        let _ = format!("{m:?}").len();
    }
}

/// `ss2-format-wld`: a world that reads has finite-sized lists and every model transform can be built.
pub fn format_wld(data: &[u8]) {
    let Ok(world) = ss2_format_wld::World::from_bytes(cap(data)) else {
        return;
    };
    assert!(world.static_models.len() <= world.entity_count);
    assert!(world.spawn_markers.len() + world.chapters.len() <= world.entity_count);
    for m in &world.static_models {
        let _ = m.transform();
        let _ = m.model.path();
    }
}

/// `ss2-format-rsc`: resource databases (`.rsc`) and character tools (`.ctl`).
pub fn format_rsc(data: &[u8]) {
    let data = cap(data);
    if let Ok(db) = ss2_format_rsc::ResourceDatabase::from_bytes(data) {
        let _ = db.root_class();
        let _ = db.referenced_files();
        let _ = db.cameras();
        if let Some(screen) = db.menu_screen() {
            let _ = screen.gadgets_of_class("CGadget").count();
        }
    }
    if let Ok(tool) = ss2_format_rsc::CharacterTool::from_bytes(data) {
        let _ = tool.referenced_files();
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Sound
// ---------------------------------------------------------------------------------------------------------------------

/// `ss2-audio`: sniff, probe and decode of WAV and Ogg Vorbis, the streaming decoder, and the engine's own WAV reader.
/// What decodes has whole frames, a nonzero rate and finite samples.
pub fn audio_decode(data: &[u8]) {
    use ss2_audio::decode;
    let data = cap(data);
    let _ = decode::sniff(data);
    let _ = decode::last_granule(data);
    let _ = decode::probe(data);
    if let Ok(pcm) = decode::decode(data) {
        assert!(pcm.channels >= 1 && pcm.rate >= 1);
        assert_eq!(pcm.samples.len() % usize::from(pcm.channels), 0);
        assert!(pcm.samples.iter().all(|s| s.is_finite()));
    }
    let _ = decode::decode_wav(data);
    if let Ok(mut stream) = decode::VorbisStream::new(data) {
        assert!(stream.channels() >= 1 && stream.rate() >= 1);
        // A damaged stream ends in an error; a bound keeps a stream that never ends from hanging the run.
        for _ in 0..4096 {
            match stream.read_chunk() {
                Ok(Some(_)) => {}
                Ok(None) | Err(_) => break,
            }
        }
    }
    if let Ok(w) = ss2_audio::wav::decode(data) {
        assert!((1..=2).contains(&w.channels) && w.rate >= 1);
        assert_eq!(w.samples.len() % usize::from(w.channels), 0);
    }
}

/// `ss2-audio-api`: the wire form of an audio event for co-op play. What decodes encodes again to bytes that decode to the
/// same event.
pub fn audio_event(data: &[u8]) {
    use ss2_audio_api::replication::AudioEvent;
    let Ok(event) = AudioEvent::decode(cap(data)) else {
        return;
    };
    let mut bytes = Vec::new();
    event.encode(&mut bytes);
    let again = AudioEvent::decode(&bytes).expect("an event that was encoded decodes");
    let mut bytes_again = Vec::new();
    again.encode(&mut bytes_again);
    assert_eq!(
        bytes, bytes_again,
        "the wire form of an event is not stable"
    );
}

// ---------------------------------------------------------------------------------------------------------------------
// Game state, text files, addresses, transfers
// ---------------------------------------------------------------------------------------------------------------------

/// `ss2-game`: the byte forms of the campaign carry-over, the unlocks, the campaign note, the difficulty and the level
/// progress. What reads writes again to bytes that read again to the same bytes. The level progress also gets a copy with
/// a correct trailing checksum, because a wrong one hides every other check.
pub fn game_state_bytes(data: &[u8]) {
    use ss2_game::campaign::{CampaignNote, CarryOver, UnlockSet};
    use ss2_game::difficulty::CampaignDifficulty;
    use ss2_game::level_progress::LevelProgress;
    let data = cap(data);
    if let Ok(v) = CarryOver::from_bytes(data) {
        let bytes = v.to_bytes();
        let again = CarryOver::from_bytes(&bytes).expect("what was written reads");
        assert_eq!(bytes, again.to_bytes(), "carry-over is not stable");
    }
    if let Ok(v) = UnlockSet::from_bytes(data) {
        let bytes = v.to_bytes();
        let again = UnlockSet::from_bytes(&bytes).expect("what was written reads");
        assert_eq!(bytes, again.to_bytes(), "unlocks are not stable");
    }
    if let Ok(v) = CampaignNote::from_bytes(data) {
        let bytes = v.to_bytes();
        let again = CampaignNote::from_bytes(&bytes).expect("what was written reads");
        assert_eq!(bytes, again.to_bytes(), "the campaign note is not stable");
    }
    if let Ok(v) = CampaignDifficulty::from_bytes(data) {
        assert_eq!(
            CampaignDifficulty::from_bytes(&v.to_bytes()).map(|d| d.to_bytes()),
            Ok(v.to_bytes())
        );
    }
    let check_progress = |bytes: &[u8]| {
        if let Ok(v) = LevelProgress::from_bytes(bytes) {
            let written = v.to_bytes();
            let again = LevelProgress::from_bytes(&written).expect("what was written reads");
            assert_eq!(written, again.to_bytes(), "level progress is not stable");
        }
    };
    check_progress(data);
    if data.len() >= 4 {
        let (body, _) = data.split_at(data.len() - 4);
        let mut sealed = body.to_vec();
        sealed.extend_from_slice(&ss2_testkit::fixtures::crc32(body).to_le_bytes());
        check_progress(&sealed);
    }
}

/// `ss2-vfs`: the `.mod`, `.nfo` and `CurrentMod.txt` text formats.
pub fn mods_text(data: &[u8]) {
    let t = text(data);
    if let Ok(m) = ss2_vfs::ModManifest::parse(&t) {
        assert!(!m.modules.is_empty());
        assert!(m.game_modules().count() <= m.modules.len());
    }
    if let Ok(n) = ss2_vfs::Nfo::parse(cap(data)) {
        for (key, _) in &n.entries {
            assert!(n.get(key).is_some(), "a key that was read cannot be found");
        }
    }
    let _ = ss2_vfs::parse_current_mod(&t);
}

/// `ss2-vfs`: the text the install locator reads (`reg query` output, Steam's `libraryfolders.vdf`, the remembered choice file).
/// The first line of the input names the registry value.
pub fn locate_text(data: &[u8]) {
    let t = text(data);
    let (value, rest) = t.split_once('\n').unwrap_or((&t, ""));
    let _ = ss2_vfs::parse_reg_query(rest, value.trim());
    let _ = ss2_vfs::parse_library_folders(&t);
    let _ = ss2_vfs::parse_reg_query(&t, "InstallPath");
    // `load_choice` reads a file: one per thread, rewritten each time.
    let dir = std::env::temp_dir();
    let name = format!(
        "ss2-fuzzkit-choice-{}-{:?}.toml",
        std::process::id(),
        std::thread::current().id()
    )
    .replace(['(', ')'], "");
    let file = dir.join(name);
    if std::fs::write(&file, cap(data)).is_ok() {
        let _ = ss2_vfs::load_choice(&file);
        let _ = std::fs::remove_file(&file);
    }
}

/// `ss2-ui`: the address a player types to join a game and the invite code. A normalized invite normalizes to itself.
pub fn lobby_text(data: &[u8]) {
    use ss2_ui::lobby::{normalize_invite, parse_address};
    let t = text(data);
    if let Ok(a) = parse_address(&t) {
        assert!(a.port >= 1);
    }
    if let Ok(code) = normalize_invite(&t) {
        assert_eq!(
            normalize_invite(&code),
            Ok(code.clone()),
            "normalizing an invite twice changes it"
        );
    }
}

/// `ss2-net`: the chunked transfer of a level or snapshot to a joining player. The input drives an honest transfer
/// (random order, repeats and drops: the blob must arrive intact) and then a receiver that is fed chunks with arbitrary
/// headers (nothing may panic or grow without bound).
pub fn net_chunks(data: &[u8]) {
    use ss2_net::join::{Chunk, ChunkAck, ChunkConfig, ChunkReceiver, ChunkSender, checksum};
    let mut u = Unstructured::new(cap(data));
    let cfg = ChunkConfig {
        chunk_size: u.int_in_range(1..=64usize).unwrap_or(16),
        max_bytes: 2048,
    };
    let len = u.int_in_range(0..=2048usize).unwrap_or(0).min(u.len());
    let blob: Vec<u8> = u.bytes(len).map(<[u8]>::to_vec).unwrap_or_default();

    if let Ok(mut tx) = ChunkSender::new(7, &blob, cfg) {
        let mut rx = ChunkReceiver::new(cfg);
        let total = tx.total();
        for _ in 0..total * 8 + 8 {
            if tx.is_complete() {
                break;
            }
            let batch = u.int_in_range(1..=4usize).unwrap_or(1);
            let mut chunks = tx.next_chunks(batch);
            if u.arbitrary::<bool>().unwrap_or(false) {
                chunks.reverse();
            }
            for c in chunks {
                // A dropped chunk is sent again by the next round; a repeated one is counted.
                if !u.arbitrary::<u8>().unwrap_or(0).is_multiple_of(4) {
                    let _ = rx.receive(&c);
                    if u.arbitrary::<u8>().unwrap_or(0).is_multiple_of(4) {
                        let _ = rx.receive(&c);
                    }
                }
            }
            if let Some(ack) = rx.ack() {
                tx.on_ack(&ack);
            }
        }
        // Finish what was left with no loss so the check below always has a complete transfer.
        while !tx.is_complete() {
            for c in tx.next_chunks(8) {
                let _ = rx.receive(&c);
            }
            if let Some(ack) = rx.ack() {
                tx.on_ack(&ack);
            }
        }
        assert!(rx.is_complete());
        assert_eq!(rx.finish().as_deref(), Ok(blob.as_slice()));
        assert_eq!(checksum(&blob), checksum(&blob));
    }

    // Hostile chunks: any header, any data.
    let mut rx = ChunkReceiver::new(cfg);
    for _ in 0..32 {
        if u.is_empty() {
            break;
        }
        let data_len = u.int_in_range(0..=80usize).unwrap_or(0).min(u.len());
        let chunk = Chunk {
            transfer: u.int_in_range(0..=2u32).unwrap_or(0),
            index: u.int_in_range(0..=40u32).unwrap_or(0),
            total: u.int_in_range(0..=40u32).unwrap_or(0),
            total_len: u.int_in_range(0..=3000u32).unwrap_or(0),
            checksum: u.int_in_range(0..=1u64).unwrap_or(0),
            data: u.bytes(data_len).map(<[u8]>::to_vec).unwrap_or_default(),
        };
        let _ = rx.receive(&chunk);
        if let Some(ack) = rx.ack() {
            assert!(ack.count() as usize <= rx.received().max(ack.total as usize));
        }
        assert!(rx.received() <= 2048);
        if rx.is_complete() {
            let _ = rx.finish();
        }
    }
    let ack = ChunkAck {
        transfer: 0,
        total: u.int_in_range(0..=70u32).unwrap_or(0),
        received: vec![u.arbitrary::<u64>().unwrap_or(0)],
    };
    let _ = (ack.count(), ack.has(u.arbitrary::<u32>().unwrap_or(0)));
}
