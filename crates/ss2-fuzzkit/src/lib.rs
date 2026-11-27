//! The properties and the synthetic seeds behind the `cargo-fuzz` targets of `fuzz/` (T02-030).
//!
//! Every target of the fuzz crate is one line: `fuzz_target!(|data: &[u8]| ss2_fuzzkit::format_tex(data))`. The property
//! itself (what is called, what must hold afterwards) lives here, in a normal workspace crate, for two reasons:
//!
//! - **It also runs on stable.** `cargo-fuzz` needs nightly Rust and libFuzzer, so it cannot be a pull-request check.
//!   `tests/smoke.rs` feeds every target its seeds and a few thousand deterministic damaged and random inputs
//!   ([`mutate`]) in the ordinary test suite, so a parser that panics, hangs or breaks a property on a bad file is
//!   caught before the nightly run.
//! - **The seeds need the builders.** The corpus is built from the synthetic fixture writers of the repository (the
//!   testkit's [`MetaBuilder`](ss2_testkit::fixtures::ctsemeta::MetaBuilder), the byte layouts of each format crate's own
//!   tests, the committed hand-made files of `testdata/`), never from game files.
//!
//! The properties, for every target: the call returns (a value or an error, never a panic), it takes bounded time, what
//! it reads is bounded by the input (declared sizes are checked against the limits before they allocate), and where a
//! writer exists what was read can be written and read again unchanged. [`TARGETS`] lists them all.

mod builders;
pub mod mutate;
mod seeds;
mod targets;

pub use seeds::Seed;
pub use targets::*;

/// The largest input a target looks at: bytes beyond it are ignored, so one huge input cannot make a run slow.
pub const MAX_INPUT: usize = 64 * 1024;

/// One fuzz target: its name (the file in `fuzz/fuzz_targets/` and the `[[bin]]`), the property, and its synthetic seeds.
#[derive(Clone, Copy)]
pub struct Target {
    /// The name of the fuzz target.
    pub name: &'static str,
    /// The parsers it calls, for the table of `docs/FUZZING.md`.
    pub parsers: &'static str,
    /// The property: returns normally if it holds, panics if it does not.
    pub run: fn(&[u8]),
    /// The seed inputs, built from the synthetic fixtures.
    pub seeds: fn() -> Vec<Seed>,
}

/// Every target of this crate, in the order of the table of `docs/FUZZING.md`.
pub const TARGETS: &[Target] = &[
    Target {
        name: "meta_container",
        parsers: "ss2_meta::Container::parse, decode_objects, write",
        run: meta_container,
        seeds: seeds::meta_container,
    },
    Target {
        name: "format_tex",
        parsers: "ss2_format_tex::Texture::from_bytes",
        run: format_tex,
        seeds: seeds::format_tex,
    },
    Target {
        name: "format_bmf",
        parsers: "ss2_format_bmf::Mesh::from_bytes",
        run: format_bmf,
        seeds: seeds::format_bmf,
    },
    Target {
        name: "format_ans",
        parsers: "ss2_format_ans::AnimSet::from_bytes and the pose sampling",
        run: format_ans,
        seeds: seeds::format_ans,
    },
    Target {
        name: "format_ep",
        parsers: "ss2_format_ep::{ParamTree, EntityParams}::from_bytes",
        run: format_ep,
        seeds: seeds::format_ep,
    },
    Target {
        name: "format_fnt",
        parsers: "ss2_format_fnt::Font::from_bytes and the text layout",
        run: format_fnt,
        seeds: seeds::format_fnt,
    },
    Target {
        name: "format_fx",
        parsers: "ss2_format_fx::{Effect (pfx), FlareConfiguration (lfx), TextEffectConfiguration (tfx), VibrationEffect (vfx)}::from_bytes",
        run: format_fx,
        seeds: seeds::format_fx,
    },
    Target {
        name: "format_mat",
        parsers: "ss2_format_mat::{PhysicalMaterial, SoundScheme, ParticleScheme}::from_bytes",
        run: format_mat,
        seeds: seeds::format_mat,
    },
    Target {
        name: "format_mtr",
        parsers: "ss2_format_mtr::MaterialFile::from_bytes",
        run: format_mtr,
        seeds: seeds::format_mtr,
    },
    Target {
        name: "format_mdl",
        parsers: "ss2_format_mdl::Model::from_bytes",
        run: format_mdl,
        seeds: seeds::format_mdl,
    },
    Target {
        name: "format_rsc",
        parsers: "ss2_format_rsc::{ResourceDatabase, CharacterTool}::from_bytes",
        run: format_rsc,
        seeds: seeds::format_rsc,
    },
    Target {
        name: "format_wld",
        parsers: "ss2_format_wld::World::from_bytes",
        run: format_wld,
        seeds: seeds::format_wld,
    },
    Target {
        name: "audio_decode",
        parsers: "ss2_audio::decode::{sniff, probe, decode, decode_wav, decode_vorbis, VorbisStream, last_granule}, ss2_audio::wav::decode",
        run: audio_decode,
        seeds: seeds::audio_decode,
    },
    Target {
        name: "audio_event",
        parsers: "ss2_audio_api::replication::AudioEvent::decode",
        run: audio_event,
        seeds: seeds::audio_event,
    },
    Target {
        name: "game_state_bytes",
        parsers: "ss2_game::{campaign::CarryOver, campaign::UnlockSet, campaign::CampaignNote, difficulty::CampaignDifficulty, level_progress::LevelProgress}::from_bytes",
        run: game_state_bytes,
        seeds: seeds::game_state_bytes,
    },
    Target {
        name: "mods_text",
        parsers: "ss2_vfs::{ModManifest::parse, Nfo::parse, parse_current_mod}",
        run: mods_text,
        seeds: seeds::mods_text,
    },
    Target {
        name: "locate_text",
        parsers: "ss2_vfs::{parse_reg_query, parse_library_folders, load_choice}",
        run: locate_text,
        seeds: seeds::locate_text,
    },
    Target {
        name: "lobby_text",
        parsers: "ss2_ui::lobby::{parse_address, normalize_invite}",
        run: lobby_text,
        seeds: seeds::lobby_text,
    },
    Target {
        name: "net_chunks",
        parsers: "ss2_net::join::{ChunkSender, ChunkReceiver}",
        run: net_chunks,
        seeds: seeds::net_chunks,
    },
];
