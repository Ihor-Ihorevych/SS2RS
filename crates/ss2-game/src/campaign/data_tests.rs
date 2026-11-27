//! Data-gated: the level list of the real install, built from the levels `ss2_vfs::levels::discover_levels` finds. Skips without `SS2_DATA_DIR`.
//!
//! `build_entries` is also the reference for how a caller turns discovered levels into a campaign list: the single-player levels, the
//! episode is the level's group (its folder above), the order is the numbering `N_M` of the id, and the last level ends the campaign.

use super::*;
use crate::difficulty::CampaignDifficulty;
use ss2_vfs::levels::discover_levels;
use ss2_vfs::{Nfo, Vfs, VfsPath};

/// `1_5_city_suburbs` -> (1, 5).
fn number(id: &str) -> (u32, u32) {
    let mut it = id.split('_').map(|s| s.parse::<u32>().unwrap());
    (it.next().unwrap(), it.next().unwrap())
}

struct Built {
    entries: Vec<LevelEntry>,
    /// Levels whose `.nfo` says `LEVEL` but not `SINGLE`.
    other_levels: usize,
}

fn build_entries(v: &dyn Vfs) -> Built {
    let mut singles: Vec<LevelEntry> = Vec::new();
    let mut other_levels = 0;
    for level in discover_levels(v).unwrap() {
        if !level.has_map_type("SINGLE") {
            other_levels += 1;
            continue;
        }
        let nfo = Nfo::parse(&v.read(&level.nfo).unwrap()).unwrap();
        let movie = |key| nfo.get(key).filter(|m| !m.is_empty()).map(str::to_string);
        let mut entry = LevelEntry::new(&level.id, &level.group, &level.name);
        entry.episode_movie = movie("EPISODE_MOVIE");
        entry.intro_movie = movie("INTRO_MOVIE");
        entry.outro_movie = movie("OUTRO_MOVIE");
        singles.push(entry);
    }
    singles.sort_by_key(|e| number(&e.id));
    if let Some(last) = singles.last_mut() {
        last.next = Next::End;
    }
    Built {
        entries: singles,
        other_levels,
    }
}

#[test]
fn the_levels_of_the_real_install_make_a_valid_list_in_the_order_the_specs_state() {
    let dir = ss2_testkit::require_game_data!();
    let v = ss2_vfs::mount_install(&dir).unwrap();
    let built = build_entries(&v);
    let list = LevelList::new(built.entries.clone()).expect("the real levels must validate");

    // game-modes.md: 42 single-player and co-op levels, 16 other levels (15 `DM;` and 1 `DM; CTF;`).
    assert_eq!(list.len(), 42);
    assert_eq!(built.other_levels, 16);

    // The order is the numbering 1_1 ... 7_7: seven episodes of 8, 6, 5, 4, 7, 5 and 7 levels.
    let per_episode = [8u32, 6, 5, 4, 7, 5, 7];
    let mut expected = Vec::new();
    for (i, n) in per_episode.iter().enumerate() {
        for m in 1..=*n {
            expected.push((u32::try_from(i).unwrap() + 1, m));
        }
    }
    let got: Vec<(u32, u32)> = list.entries().iter().map(|e| number(&e.id)).collect();
    assert_eq!(got, expected);
    assert_eq!(list.first().id, "1_1_jungle");
    assert_eq!(list.entries().last().unwrap().id, "7_7_mental");
    assert_eq!(list.main_path().len(), 42);
    let mut episodes: Vec<&str> = list.entries().iter().map(|e| e.episode.as_str()).collect();
    episodes.dedup();
    assert_eq!(
        episodes,
        [
            "01_mdigbo",
            "02_magnor",
            "03_chifang",
            "04_kleer",
            "05_ellenier",
            "06_kronor",
            "07_sirius"
        ]
    );

    // cutscenes.md: the episode movie belongs to the first level of an episode, every level has an intro, and three have no outro.
    for e in list.entries() {
        let first_of_episode = number(&e.id).1 == 1;
        assert_eq!(e.episode_movie.is_some(), first_of_episode, "{}", e.id);
        assert!(e.intro_movie.is_some(), "{}", e.id);
    }
    let without_outro: Vec<&str> = list
        .entries()
        .iter()
        .filter(|e| e.outro_movie.is_none())
        .map(|e| e.id.as_str())
        .collect();
    assert_eq!(
        without_outro,
        [
            "6_1_compound",
            "7_3_arenas_desertstadium",
            "7_4_arenas_junglestadium"
        ]
    );

    // Every movie the list names exists in the install (the VFS is case-insensitive).
    let mut movies = 0;
    for e in list.entries() {
        for m in [&e.episode_movie, &e.intro_movie, &e.outro_movie]
            .into_iter()
            .flatten()
        {
            assert!(
                v.exists(&VfsPath::new(m).unwrap()),
                "{m} is not in the install"
            );
            movies += 1;
        }
    }
    assert_eq!(movies, 7 + 42 + 39);
    eprintln!(
        "real install: {} levels, {} episodes, {} movies named, {} other levels left out",
        list.len(),
        per_episode.len(),
        movies,
        built.other_levels
    );
}

#[test]
fn a_whole_campaign_of_the_real_levels_plays_through() {
    let dir = ss2_testkit::require_game_data!();
    let v = ss2_vfs::mount_install(&dir).unwrap();
    let list = LevelList::new(build_entries(&v).entries).unwrap();
    let ids: Vec<String> = list.entries().iter().map(|e| e.id.clone()).collect();
    let diff = CampaignDifficulty::new(
        crate::difficulty::Difficulty::Normal,
        crate::difficulty::ChangePolicy::BetweenLevels,
    );
    let mut c = Campaign::new(list, diff);
    let mut movies: Vec<String> = Vec::new();
    let drive = |c: &mut Campaign, movies: &mut Vec<String>| loop {
        if let Some(m) = c.current_movie() {
            movies.push(m.to_string());
        }
        match c.phase() {
            Phase::Playing { .. } | Phase::Finished { .. } => return,
            Phase::Ending { .. } => c.progress(Progress::MovieDone).unwrap(),
            Phase::Transition { stage, .. } => c
                .progress(match stage {
                    Stage::Load => Progress::Loaded,
                    Stage::Spawn => Progress::Spawned,
                    _ => Progress::MovieDone,
                })
                .unwrap(),
        };
    };
    drive(&mut c, &mut movies);
    for (i, id) in ids.iter().enumerate() {
        assert_eq!(c.current_level().id, *id);
        assert_eq!(c.phase(), Phase::Playing { level: i });
        c.advance(100);
        let mut gate = ExitGate::new(1, 0).unwrap();
        gate.set_status(0, PlayerStatus::AtExit).unwrap();
        let mut carry = CarryOver::new();
        carry.set(0, Slot::Carried(PlayerCarry::default())).unwrap();
        let rules = UnlockRules::none();
        let d = c
            .leave(LeaveRequest {
                gate: &gate,
                carry,
                result: LevelResult {
                    level: id.clone(),
                    exit: 0,
                    secrets_found: 0,
                    secrets_total: 0,
                },
                rules: &rules,
            })
            .unwrap();
        assert_eq!(d.campaign_complete, i + 1 == ids.len());
        assert_eq!(d.to.as_deref(), ids.get(i + 1).map(String::as_str));
        drive(&mut c, &mut movies);
    }
    assert_eq!(c.phase(), Phase::Finished { last: 41 });
    assert_eq!(c.elapsed_ticks(), 4_200);
    assert_eq!(movies.len(), 7 + 42 + 39);
    assert!(movies[0].to_lowercase().ends_with("episode_1.ogv"));
    assert!(movies[1].to_lowercase().ends_with("1_1_intro.ogv"));
    assert!(
        movies
            .last()
            .unwrap()
            .to_lowercase()
            .ends_with("7_7_outro.ogv")
    );
    for id in &ids {
        assert!(c.unlocks().level_unlocked(id), "{id}");
    }
    // The finished campaign's note survives a round trip.
    let note = c.note().unwrap();
    assert_eq!(CampaignNote::from_bytes(&note.to_bytes()).unwrap(), note);
}
