//! The synthetic seed corpus of every target.
//!
//! Where a seed comes from, in order of preference: a builder of the testkit or of the format crates' own tests (all from
//! the clean-room specs), a committed hand-made file of `testdata/` (each has a provenance note there), or a few lines of
//! text written by hand. **Nothing here is read from a game install.** `cargo xtask guard` checks the committed copies in
//! `fuzz/seeds/` like every other file.

use crate::builders;
use ss2_testkit::fixtures::ctsemeta::MetaBuilder;

/// One seed input: a name for the file in `fuzz/seeds/<target>/` and its bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seed {
    /// A short name; the committed file is `<name>.seed`.
    pub name: String,
    /// The bytes.
    pub bytes: Vec<u8>,
}

fn seed(name: &str, bytes: Vec<u8>) -> Seed {
    Seed {
        name: name.to_owned(),
        bytes,
    }
}

fn text(name: &str, text: &str) -> Seed {
    seed(name, text.as_bytes().to_vec())
}

// ---- the hand-made files of testdata/ ---------------------------------------------------------------------------------

const TEX_BGR: &[u8] = include_bytes!("../../../testdata/asset-export/bgr_8x4.tex");
const TEX_BGRA: &[u8] = include_bytes!("../../../testdata/asset-export/bgra_4x4.tex");
const TEX_DXT1: &[u8] = include_bytes!("../../../testdata/asset-export/dxt1_4x4.tex");
const TEX_CUBE: &[u8] = include_bytes!("../../../testdata/asset-export/cube_4x4.tex");
const BMF_QUAD: &[u8] = include_bytes!("../../../testdata/asset-export/quad.bmf");
const BMF_RICH: &[u8] = include_bytes!("../../../testdata/asset-export/rich.bmf");
const WAV_IMA_MONO: &[u8] = include_bytes!("../../../testdata/audio/ima_mono.wav");
const WAV_IMA_STEREO: &[u8] = include_bytes!("../../../testdata/audio/ima_stereo.wav");
const OGG_MONO: &[u8] = include_bytes!("../../../testdata/audio/vorbis_mono.ogg");
const OGG_STEREO: &[u8] = include_bytes!("../../../testdata/audio/vorbis_stereo.ogg");

// ---- the seeds of each target -----------------------------------------------------------------------------------------

/// The seeds of `meta_container`: the testkit's sample file, a small file with editor records, and a file of every
/// other format (they are all containers).
pub fn meta_container() -> Vec<Seed> {
    let mut out = vec![seed(
        "sample",
        MetaBuilder::sample().build().expect("the sample builds"),
    )];
    for (name, bytes) in [("tex_bgra", TEX_BGRA), ("bmf_quad", BMF_QUAD)] {
        out.push(seed(name, bytes.to_vec()));
    }
    out.push(seed(
        "ans",
        builders::ans::file(builders::ans::Flaws::default()),
    ));
    out.push(seed("ep", builders::ep::params_file(true)));
    out.push(seed(
        "rsc",
        builders::rsc::menu_screen_file("CMenuScreenLayout"),
    ));
    out
}

/// The seeds of `format_tex`: the four hand-made textures of `testdata/asset-export` (24 bit, 32 bit, DXT1 with a mip
/// chain, DXT1 cube map).
pub fn format_tex() -> Vec<Seed> {
    vec![
        seed("bgr_8x4", TEX_BGR.to_vec()),
        seed("bgra_4x4", TEX_BGRA.to_vec()),
        seed("dxt1_4x4", TEX_DXT1.to_vec()),
        seed("cube_4x4", TEX_CUBE.to_vec()),
    ]
}

/// The seeds of `format_bmf`: the two hand-made meshes of `testdata/asset-export` (one quad; two LODs with a skinned
/// surface), and the crate's quad with a collision mesh (on the render vertices with a tree; 32-bit indices on its own
/// positions).
pub fn format_bmf() -> Vec<Seed> {
    vec![
        seed("quad", BMF_QUAD.to_vec()),
        seed("rich", BMF_RICH.to_vec()),
        seed("collision", builders::bmf::quad_with_collision()),
        seed("collision_own", builders::bmf::quad_with_own_collision()),
    ]
}

/// The seeds of `format_ans`: one animation set with every kind of track and event.
pub fn format_ans() -> Vec<Seed> {
    use builders::ans::{Flaws, file};
    vec![seed("walk", file(Flaws::default()))]
}

/// The seeds of `format_ep`: a projectile parameter file, and the same with a root that is not an entity parameter.
pub fn format_ep() -> Vec<Seed> {
    vec![
        seed("projectile", builders::ep::params_file(true)),
        seed("not_entity", builders::ep::params_file(false)),
    ]
}

/// The seeds of `format_fnt`: a proportional font with a 32-bit atlas, and a font with a larger atlas.
pub fn format_fnt() -> Vec<Seed> {
    vec![
        seed("sample", builders::fnt::sample()),
        seed(
            "one_glyph",
            builders::fnt::font_file(8, 8, 9, &[(0, 0, 4, 8, 0, 0, 5, 0)], &[0; 33]),
        ),
    ]
}

/// The seeds of `format_fx`: a flare, a flare with two parts, a text effect, a vibration effect and a particle effect.
pub fn format_fx() -> Vec<Seed> {
    vec![
        seed("flare", builders::fx_lfx::flare_file(1)),
        seed("flare2", builders::fx_lfx::flare_file(2)),
        seed("text", builders::fx_tfx::text_file(false)),
        seed("vibro", builders::fx_vfx::vibro_file(true, true)),
        seed(
            "particles",
            builders::fx_pfx::pfx_file("CSphereEmitterShape"),
        ),
        seed(
            "particles_box",
            builders::fx_pfx::pfx_file("CBoxEmitterShape"),
        ),
    ]
}

/// The seeds of `format_mat`: a sound scheme and a physical material with its embedded schemes.
pub fn format_mat() -> Vec<Seed> {
    vec![
        seed("sound_scheme", builders::mat::sound_scheme_file()),
        seed("material", builders::mat::material_file()),
    ]
}

/// The seeds of `format_mtr`: a material preset file with an external texture, with the shader class changed.
pub fn format_mtr() -> Vec<Seed> {
    vec![
        seed("preset", builders::mtr::material_file(true, 1010)),
        seed("preset_1050", builders::mtr::material_file(true, 1050)),
        seed(
            "embedded_texture",
            builders::mtr::material_file(false, 1010),
        ),
    ]
}

/// The seeds of `format_wld`: a world with static models, a spawn marker and a chapter, and the same world with a
/// second root and with a dangling entity (both refused).
pub fn format_wld() -> Vec<Seed> {
    use builders::wld::{NORMAL, Variant, world_file};
    vec![
        seed("world", world_file(NORMAL)),
        seed(
            "two_roots",
            world_file(Variant {
                worlds: 2,
                ..NORMAL
            }),
        ),
        seed(
            "dangling_entity",
            world_file(Variant {
                entity_override: Some(77),
                ..NORMAL
            }),
        ),
    ]
}

/// The seeds of `format_mdl`: a model with an embedded quad mesh and its preset, one with an
/// external mesh and a null instance, and one with a helper configuration next to the main one.
pub fn format_mdl() -> Vec<Seed> {
    use builders::mdl::{Inst, Mat, Spec, model_file};
    let mut helper = Spec::new(&[Inst::Embedded]);
    helper.second = Some(-1);
    helper.material = Mat::External;
    vec![
        seed("embedded", model_file(&Spec::new(&[Inst::Embedded]))),
        seed(
            "external",
            model_file(&Spec::new(&[Inst::External, Inst::Null])),
        ),
        seed("helper", model_file(&helper)),
    ]
}

/// The seeds of `format_rsc`: a menu screen and character tools.
pub fn format_rsc() -> Vec<Seed> {
    vec![
        seed("menu", builders::rsc::menu_screen_file("CMenuScreenLayout")),
        seed(
            "tool",
            builders::rsc::tool_file("CCharacterTool", true, None),
        ),
        seed(
            "tool_creator",
            builders::rsc::tool_file("CCharacterTool", true, Some("CDebrisToolEntityCreator")),
        ),
    ]
}

/// The seeds of `audio_decode`: two IMA-ADPCM WAV files, two Ogg Vorbis files (all hand-made in `testdata/audio`), and
/// WAV files written by the engine's own encoder (16-bit and float, mono and stereo).
pub fn audio_decode() -> Vec<Seed> {
    // A sine tone, precomputed instead of calling `f32::sin` here: libm's `sin` is not required to be
    // correctly-rounded, so the committed `float_stereo` seed (which stores these bits raw, unlike the quantised
    // `pcm16_mono` one) could differ by a ULP between platforms and fail the builder-matches-committed-file check.
    let tone: [f32; 64] = [
        0.0,
        0.14776011,
        0.28232124,
        0.39166346,
        0.46601954,
        0.4987475,
        0.4869238,
        0.43160465,
        0.33773157,
        0.21368992,
        0.07056,
        -0.07887294,
        -0.22126028,
        -0.34388313,
        -0.43578795,
        -0.48876506,
        -0.4980823,
        -0.46290725,
        -0.38638222,
        -0.27534264,
        -0.13970774,
        0.008407045,
        0.15577087,
        0.28921992,
        0.39683402,
        0.46900007,
        0.4992717,
        0.48494485,
        0.4272993,
        0.33148432,
        0.20605925,
        0.06222712,
        -0.087163575,
        -0.2287682,
        -0.34993762,
        -0.4398479,
        -0.49046814,
        -0.49727628,
        -0.45966414,
        -0.38099155,
        -0.26828647,
        -0.1316158,
        0.016811714,
        0.16373749,
        0.29603708,
        0.40189248,
        0.47184786,
        0.4996547,
        0.48282883,
        0.4228732,
        0.32514355,
        0.1983702,
        0.053876635,
        -0.09542957,
        -0.23621133,
        -0.35589266,
        -0.4437838,
        -0.49203253,
        -0.4963296,
        -0.45629108,
        -0.37549362,
        -0.26115382,
        -0.123486646,
        0.025212105,
    ];
    vec![
        seed("ima_mono", WAV_IMA_MONO.to_vec()),
        seed("ima_stereo", WAV_IMA_STEREO.to_vec()),
        seed("vorbis_mono", OGG_MONO.to_vec()),
        seed("vorbis_stereo", OGG_STEREO.to_vec()),
        seed(
            "pcm16_mono",
            ss2_audio::wav::encode_pcm16(22_050, 1, &tone).expect("a valid wav"),
        ),
        seed(
            "float_stereo",
            ss2_audio::wav::encode_f32(44_100, 2, &tone).expect("a valid wav"),
        ),
    ]
}

/// The seeds of `audio_event`: each of the three events in its wire form.
pub fn audio_event() -> Vec<Seed> {
    use ss2_audio_api::PlayParams;
    use ss2_audio_api::replication::{AudioEvent, EmitterId, SoundRef};
    let events = [
        (
            "one_shot",
            AudioEvent::OneShot {
                sound: SoundRef(3),
                params: PlayParams::sfx(),
            },
        ),
        (
            "loop_start",
            AudioEvent::LoopStart {
                emitter: EmitterId(9),
                sound: SoundRef(4),
                params: PlayParams {
                    looped: true,
                    ..PlayParams::sfx()
                },
            },
        ),
        (
            "loop_stop",
            AudioEvent::LoopStop {
                emitter: EmitterId(9),
                fade_ms: 250,
            },
        ),
    ];
    events
        .into_iter()
        .map(|(name, e)| {
            let mut bytes = Vec::new();
            e.encode(&mut bytes);
            seed(name, bytes)
        })
        .collect()
}

/// The seeds of `game_state_bytes`: the empty carry-over and unlocks, a difficulty, a level progress, a campaign note.
pub fn game_state_bytes() -> Vec<Seed> {
    use ss2_game::campaign::{
        Campaign, CarryOver, LevelEntry, LevelList, Next, Progress, UnlockSet,
    };
    use ss2_game::difficulty::CampaignDifficulty;
    use ss2_game::level_progress::{LevelProgress, LevelSetup};
    let mut out = vec![
        seed("carry_empty", CarryOver::new().to_bytes()),
        seed("unlocks_empty", UnlockSet::new().to_bytes()),
    ];
    if let Ok(p) = LevelProgress::new(LevelSetup {
        players: 2,
        secrets: 3,
        checkpoints: 2,
        ..LevelSetup::default()
    }) {
        out.push(seed("progress", p.to_bytes()));
    }
    if let Ok(d) = CampaignDifficulty::from_bytes(&[1, 1, 0, 0]) {
        out.push(seed("difficulty", d.to_bytes().to_vec()));
        if let Ok(list) = LevelList::new(vec![
            LevelEntry::new("a", "e", "Level A"),
            LevelEntry::new("b", "e", "Level B").with_next(Next::End),
        ]) {
            let mut campaign = Campaign::new(list, d);
            // Load, then spawn: the campaign is playing the first level and can be saved.
            let _ = campaign.progress(Progress::Loaded);
            let _ = campaign.progress(Progress::Spawned);
            if let Ok(note) = campaign.note() {
                out.push(seed("note", note.to_bytes()));
            }
        }
    }
    out
}

/// The seeds of `mods_text`: a `.mod`, a `.nfo` for a level and for a mod, and a `CurrentMod.txt`.
pub fn mods_text() -> Vec<Seed> {
    vec![
        text("mod", "MODULES = Base <TOOLS>Editor Patch1 ;\r\n"),
        text(
            "nfo_level",
            "LEVEL\r\nNAME = \"Test Level\";\r\nEPISODE = \"One\"\r\nMOVIE = \"Content/Test/Intro.xmv\";\r\n",
        ),
        text(
            "nfo_mod",
            "NAME=\"A mod\"\nVERSION = 1.2 ;\nAUTHOR = \"Nobody\"\n",
        ),
        text(
            "current",
            "# the mod that is loaded\nCURRENTMOD = SeriousSam2.mod ;\n",
        ),
    ]
}

/// The seeds of `locate_text`: `reg query` output, a Steam library list and a remembered-choice file.
pub fn locate_text() -> Vec<Seed> {
    vec![
        text(
            "reg_query",
            "InstallPath\n\nHKEY_LOCAL_MACHINE\\SOFTWARE\\Example\n    InstallPath    REG_SZ    C:\\Games\\Example\n\n",
        ),
        text(
            "library_folders",
            "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"C:\\\\Program Files (x86)\\\\Steam\"\n\t\t\"apps\"\n\t\t{\n\t\t\t\"1\"\t\"100\"\n\t\t}\n\t}\n\t\"1\"\n\t{\n\t\t\"path\"\t\t\"D:\\\\Library\"\n\t}\n}\n",
        ),
        text(
            "choice",
            "path = \"C:\\\\Games\\\\Example\"\nvalidated_at = 1700000000\nfuture_key = true\n",
        ),
    ]
}

/// The seeds of `lobby_text`: addresses of each kind and invite codes.
pub fn lobby_text() -> Vec<Seed> {
    vec![
        text("v4_port", "192.168.1.20:27015"),
        text("v6_port", "[::1]:27015"),
        text("name", "example.com"),
        text("invite", "ss2-abcd-efgh-2345"),
        text("bad", "256.1.1.1:99999"),
    ]
}

/// The seeds of `net_chunks`: bytes that drive a transfer of a few chunks and then a few damaged ones.
pub fn net_chunks() -> Vec<Seed> {
    let mut rng = crate::mutate::Rng::new(0x5EED);
    vec![
        seed("short", crate::mutate::random_bytes(&mut rng, 96)),
        seed("long", crate::mutate::random_bytes(&mut rng, 700)),
    ]
}
