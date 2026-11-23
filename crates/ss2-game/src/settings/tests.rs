use super::*;
use ss2_audio_api::NullAudio;
use ss2_ui::settings_model::{Nav, Step};

const SIZES: [(u32, u32); 4] = [(1280, 720), (1920, 1080), (2560, 1440), (3840, 2160)];

fn screen_for(s: &GameSettings) -> SettingsScreen {
    build_screen(s, &SIZES, 15.0).unwrap()
}

fn default_screen() -> SettingsScreen {
    screen_for(&GameSettings::default())
}

fn all_ids(screen: &SettingsScreen) -> Vec<String> {
    (0..4)
        .flat_map(|t| screen.rows_in(t).iter().map(|r| r.id.clone()))
        .collect()
}

/// A settings value that changes everything away from the defaults.
fn everything_different() -> GameSettings {
    let mut s = GameSettings::default();
    s.video.window_mode = WindowMode::Exclusive;
    s.video.resolution = (2560, 1440);
    s.video.vsync = false;
    s.video.fov = 105.0;
    s.video.ui = ScaleSettings {
        aspect: AspectMode::Letterbox,
        user_scale: 0.75,
    };
    s.quality = QualitySettings::preset(QualityTier::Low);
    s.quality.lod_bias = 1.25;
    s.quality.max_decals = 123;
    s.audio = AudioSettings {
        master: 0.3,
        music: 0.0,
        sfx: 1.0,
        voice: 0.55,
        ui: 0.1,
    };
    s.accessibility.set("text_scale", "1.5").unwrap();
    s.accessibility.set("colorblind", "tritanopia").unwrap();
    s.accessibility.set("subtitle_size", "extra_large").unwrap();
    s.accessibility
        .set("subtitle_background", "opaque")
        .unwrap();
    for id in ["reduce_flashing", "reduce_motion", "high_contrast"] {
        s.accessibility.set(id, "on").unwrap();
    }
    for id in ["crouch", "sprint", "aim"] {
        s.accessibility.set(id, "toggle").unwrap();
    }
    s
}

#[test]
fn the_screen_has_four_tabs_and_a_row_for_every_option_the_game_has() {
    let screen = default_screen();
    assert_eq!(
        screen.tab_names(),
        ["Video", "Graphics", "Audio", "Accessibility"]
    );
    assert_eq!(screen.rows_in(0).len(), 6);
    assert_eq!(screen.rows_in(1).len(), QualitySettings::options().len());
    assert_eq!(screen.rows_in(1).len(), 11);
    assert_eq!(screen.rows_in(2).len(), 5);
    assert_eq!(screen.rows_in(3).len(), OPTION_NAMES.len());
    let ids = all_ids(&screen);
    for id in VIDEO_ROWS.iter().chain(&AUDIO_ROWS).chain(&OPTION_NAMES) {
        assert!(ids.iter().any(|i| i == id), "no row {id}");
    }
    for desc in QualitySettings::options() {
        assert!(ids.iter().any(|i| i == desc.name), "no row {}", desc.name);
    }
    // one audio row per bus, in the order of Bus::ALL
    assert_eq!(AUDIO_ROWS.len(), Bus::ALL.len());
    // and every row belongs to a section, in the tab it is on
    for (tab, section) in [
        Section::Video,
        Section::Graphics,
        Section::Audio,
        Section::Accessibility,
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(screen.tab_names()[tab], section.title());
        for row in screen.rows_in(tab) {
            assert_eq!(section_of(&row.id), Some(section), "{}", row.id);
        }
    }
    assert_eq!(section_of("nope"), None);
}

#[test]
fn a_screen_shows_exactly_the_settings_it_was_given() {
    for original in [
        GameSettings::default(),
        everything_different(),
        GameSettings {
            quality: QualitySettings::preset(QualityTier::Ultra),
            ..GameSettings::default()
        },
        GameSettings {
            quality: QualitySettings::preset(QualityTier::Medium),
            ..GameSettings::default()
        },
        // a resolution the monitor list does not have
        GameSettings {
            video: VideoSettings {
                resolution: (1000, 700),
                ..VideoSettings::default()
            },
            ..GameSettings::default()
        },
    ] {
        let screen = screen_for(&original);
        assert!(!screen.has_changes());
        assert_eq!(applied_settings(&screen).unwrap(), original);
        assert_eq!(pending_settings(&screen).unwrap(), original);
    }
}

#[test]
fn edits_are_pending_until_apply_and_then_become_the_game_settings() {
    let original = GameSettings::default();
    let mut screen = screen_for(&original);
    screen.set_text("shadows", "low").unwrap();
    screen.set_text("render_scale", "0.7").unwrap();
    screen.set_text("music_volume", "0.2").unwrap();
    screen.set_text("text_scale", "1.3").unwrap();
    screen.set_text("fov", "100").unwrap();
    screen.set_text("vsync", "off").unwrap();
    screen.set_text("aspect", "letterbox").unwrap();
    screen.set_text("ui_scale", "0.8").unwrap();
    screen.set_text("window_mode", "borderless").unwrap();
    screen.set_text("resolution", "2560x1440").unwrap();

    assert_eq!(applied_settings(&screen).unwrap(), original);
    let pending = pending_settings(&screen).unwrap();
    assert_ne!(pending, original);
    assert_eq!(pending.quality.render_scale, 0.7);
    assert_eq!(pending.video.resolution, (2560, 1440));

    let out = screen.apply(10.0).unwrap();
    assert_eq!(out.changes.len(), 10);
    let now = applied_settings(&screen).unwrap();
    assert_eq!(now, pending);
    assert_eq!(now.audio.music, 0.2);
    assert_eq!(now.audio.master, original.audio.master);
    assert_eq!(now.accessibility.text_scale, 1.3);
    assert_eq!(now.video.window_mode, WindowMode::Borderless);
    assert_eq!(now.video.ui.aspect, AspectMode::Letterbox);
    assert_eq!(now.video.ui.user_scale, 0.8);
    assert_eq!(now.video.fov, 100.0);
    assert!(!now.video.vsync);
    now.quality.validate().unwrap();
}

#[test]
fn revert_throws_the_edits_away_and_reset_goes_to_the_defaults_but_only_after_apply() {
    let custom = everything_different();
    let mut screen = screen_for(&custom);
    screen.set_text("shadows", "high").unwrap();
    screen.revert();
    assert_eq!(pending_settings(&screen).unwrap(), custom);

    screen.reset_all().unwrap();
    assert_eq!(applied_settings(&screen).unwrap(), custom);
    let pending = pending_settings(&screen).unwrap();
    // the resolution default is the fresh install's 1920x1080 when the monitor offers it
    assert_eq!(pending, GameSettings::default());
    assert!(screen.has_changes());
    screen.apply(1.0).unwrap();
    assert_eq!(applied_settings(&screen).unwrap(), GameSettings::default());
}

#[test]
fn the_default_resolution_is_the_largest_when_the_monitor_does_not_offer_the_usual_one() {
    let mut s = GameSettings::default();
    s.video.resolution = (1366, 768);
    let mut screen = build_screen(&s, &[(1366, 768), (1600, 900)], 15.0).unwrap();
    screen.set_text("resolution", "1366x768").unwrap();
    screen.reset_row("resolution").unwrap();
    assert_eq!(screen.pending("resolution"), Some("1600x900"));
    // with no list at all the current resolution is the only one
    let only = build_screen(&s, &[], 15.0).unwrap();
    assert_eq!(
        only.row("resolution").unwrap().kind.expected(),
        "one of 1366x768"
    );
    assert_eq!(only.default_of("resolution"), Some("1366x768"));
}

#[test]
fn the_resolution_list_is_sorted_smallest_first_and_has_each_size_once() {
    let list = resolution_words(&[(1920, 1080), (1280, 720), (1920, 1080)], (1280, 720));
    assert_eq!(list, [(1280, 720), (1920, 1080)]);
    let with_current = resolution_words(&[(1920, 1080), (1280, 720)], (1600, 900));
    assert_eq!(with_current, [(1280, 720), (1600, 900), (1920, 1080)]);
    // same area, different shape: the narrower one first, and neither is lost
    let same_area = resolution_words(&[(1000, 1000)], (500, 2000));
    assert_eq!(same_area, [(500, 2000), (1000, 1000)]);
    // stepping up a resolution goes to a larger one
    let mut screen = default_screen();
    assert_eq!(
        screen.step("resolution", Step::Increase).unwrap(),
        "2560x1440"
    );
    assert_eq!(
        screen.step("resolution", Step::Decrease).unwrap(),
        "1920x1080"
    );
    assert_eq!(
        screen.step("resolution", Step::Decrease).unwrap(),
        "1280x720"
    );
    assert_eq!(parse_size("1280x720"), Some((1280, 720)));
    assert_eq!(parse_size("1280"), None);
    assert_eq!(parse_size("ax720"), None);
    assert_eq!(parse_size("1280xb"), None);
}

#[test]
fn a_setting_that_is_out_of_range_is_refused_when_the_screen_is_built() {
    let mut s = GameSettings::default();
    s.quality.render_scale = 5.0;
    let e = build_screen(&s, &SIZES, 15.0).unwrap_err();
    assert_eq!(e.to_string(), "render_scale takes a number from 0.5 to 1");
    let mut s = GameSettings::default();
    s.video.fov = f32::NAN;
    assert!(build_screen(&s, &SIZES, 15.0).is_err());
    let mut s = GameSettings::default();
    s.video.fov = 200.0;
    assert_eq!(
        build_screen(&s, &SIZES, 15.0).unwrap_err().to_string(),
        "fov takes a number from 30 to 120"
    );
    let mut s = GameSettings::default();
    s.audio.sfx = 1.5;
    assert!(build_screen(&s, &SIZES, 15.0).is_err());
    let mut s = GameSettings::default();
    s.video.ui.user_scale = 2.0;
    assert!(build_screen(&s, &SIZES, 15.0).is_err());
    assert!(build_screen(&GameSettings::default(), &SIZES, 0.0).is_err());
}

#[test]
fn the_graphics_rows_say_no_in_the_same_words_and_for_the_same_values_as_the_settings_themselves() {
    let probes = [
        "", " ", "abc", "NaN", "nan", "inf", "-inf", "-1", "-0.5", "0", "0.0", "0.1", "0.25",
        "0.5", "0.50", "0.75", "1", "1.0", "1.5", "2", "2.0", "3", "3.5", "4", "8", "16", "17",
        "50", "100", "999", "1000", "1001", "1e3", "on", "ON", " off ", "true", "no", "yes", "1 ",
        "off", "low", "Medium", "HIGH", "ultra", "fxaa", "msaa2", "MSAA4", "quarter", "half",
        "full", "0,5",
    ];
    let screen = default_screen();
    let mut agreed = 0;
    for desc in QualitySettings::options() {
        for probe in probes {
            let mut real = QualitySettings::default();
            let real_result = real.set(desc.name, probe);
            let mut model = screen.clone();
            let model_result = model.set_text(desc.name, probe);
            match (real_result, model_result) {
                (Ok(()), Ok(text)) => {
                    assert_eq!(
                        real.get(desc.name).as_deref(),
                        Some(text.as_str()),
                        "{} = {probe:?}",
                        desc.name
                    );
                    agreed += 1;
                }
                (Err(a), Err(b)) => {
                    assert_eq!(a.to_string(), b.to_string(), "{} = {probe:?}", desc.name);
                    agreed += 1;
                }
                (a, b) => panic!(
                    "{} = {probe:?}: the setting says {a:?}, the screen {b:?}",
                    desc.name
                ),
            }
        }
    }
    assert_eq!(agreed, 11 * probes.len());
}

#[test]
fn the_accessibility_rows_say_no_in_the_same_words_and_for_the_same_values_as_the_settings_themselves()
 {
    let probes = [
        "",
        "abc",
        "NaN",
        "inf",
        "-1",
        "0",
        "0.7",
        "0.8",
        "0.80",
        "1",
        "1.5",
        "2",
        "2.0",
        "2.1",
        "3",
        "on",
        "OFF",
        "true",
        "no",
        "1",
        "hold",
        "Toggle",
        " hold ",
        "holdtoggle",
        "off",
        "protanopia",
        "Deuteranopia",
        "tritanopia",
        "achromatopsia",
        "small",
        "medium",
        "large",
        "extra_large",
        "extra large",
        "none",
        "translucent",
        "opaque",
        "clear",
    ];
    let screen = default_screen();
    let mut agreed = 0;
    for id in OPTION_NAMES {
        for probe in probes {
            let mut real = AccessibilitySettings::default();
            let real_result = real.set(id, probe);
            let mut model = screen.clone();
            let model_result = model.set_text(id, probe);
            match (real_result, model_result) {
                (Ok(()), Ok(text)) => {
                    assert_eq!(
                        real.get(id).as_deref(),
                        Some(text.as_str()),
                        "{id} = {probe:?}"
                    );
                    agreed += 1;
                }
                (Err(a), Err(b)) => {
                    assert_eq!(a.to_string(), b.to_string(), "{id} = {probe:?}");
                    agreed += 1;
                }
                (a, b) => panic!("{id} = {probe:?}: the setting says {a:?}, the screen {b:?}"),
            }
        }
    }
    assert_eq!(agreed, 10 * probes.len());
}

#[test]
fn stepping_any_row_any_number_of_times_always_gives_a_value_the_game_accepts() {
    let mut screen = default_screen();
    for id in all_ids(&screen) {
        for dir in [Step::Increase, Step::Decrease, Step::Increase] {
            for _ in 0..45 {
                screen.step(&id, dir).unwrap();
                let pending =
                    pending_settings(&screen).unwrap_or_else(|e| panic!("{id} after a step: {e}"));
                pending.quality.validate().unwrap();
                assert!((FOV_RANGE.0..=FOV_RANGE.1).contains(&pending.video.fov));
                assert!((0.5..=1.0).contains(&pending.video.ui.user_scale));
                for v in [
                    pending.audio.master,
                    pending.audio.music,
                    pending.audio.sfx,
                    pending.audio.voice,
                    pending.audio.ui,
                ] {
                    assert!((0.0..=1.0).contains(&v), "{id}: {v}");
                }
                let (lo, hi) = (0.8, 2.0);
                assert!((lo..=hi).contains(&pending.accessibility.text_scale));
            }
        }
    }
}

#[test]
fn every_range_row_walks_its_whole_grid_with_short_numbers() {
    let screen = default_screen();
    for id in all_ids(&screen) {
        let RowKind::Range { min, max, step } = screen.row(&id).unwrap().kind else {
            continue;
        };
        let mut walk = screen.clone();
        for _ in 0..200 {
            walk.step(&id, Step::Decrease).unwrap();
        }
        assert_eq!(
            walk.pending(&id),
            Some(min.to_string().as_str()),
            "{id} floor"
        );
        let mut texts = vec![walk.pending(&id).unwrap().to_owned()];
        for _ in 0..200 {
            let t = walk.step(&id, Step::Increase).unwrap();
            if texts.last() != Some(&t) {
                texts.push(t);
            }
        }
        assert_eq!(texts.last().unwrap(), &max.to_string(), "{id} ceiling");
        // every number is at most three decimals: no float noise like 0.90000004
        for t in &texts {
            let decimals = t.split_once('.').map_or(0, |(_, d)| d.len());
            assert!(decimals <= 3, "{id}: {t}");
        }
        let expected = ((max - min) / step).ceil() as usize + 1;
        assert_eq!(texts.len(), expected, "{id}: {texts:?}");
    }
}

#[test]
fn changing_the_window_or_the_resolution_asks_first_and_other_changes_do_not() {
    for (id, text) in [("window_mode", "exclusive"), ("resolution", "1280x720")] {
        let mut screen = default_screen();
        screen.set_text(id, text).unwrap();
        let out = screen.apply(50.0).unwrap();
        assert_eq!(out.confirm_until, Some(65.0), "{id}");
    }
    for (id, text) in [
        ("vsync", "off"),
        ("fov", "90"),
        ("aspect", "letterbox"),
        ("ui_scale", "0.9"),
        ("shadows", "off"),
        ("render_scale", "0.6"),
        ("master_volume", "0.1"),
        ("text_scale", "1.5"),
        ("crouch", "toggle"),
    ] {
        let mut screen = default_screen();
        screen.set_text(id, text).unwrap();
        let out = screen.apply(50.0).unwrap();
        assert_eq!(out.changes.len(), 1, "{id}");
        assert_eq!(out.confirm_until, None, "{id}");
    }
}

#[test]
fn if_the_player_does_not_keep_a_new_resolution_the_old_settings_come_back() {
    let before = GameSettings::default();
    let mut screen = screen_for(&before);
    screen.set_text("resolution", "3840x2160").unwrap();
    screen.set_text("window_mode", "exclusive").unwrap();
    screen.set_text("shadows", "off").unwrap();
    screen.apply(0.0).unwrap();
    assert_eq!(
        applied_settings(&screen).unwrap().video.resolution,
        (3840, 2160)
    );
    assert_eq!(
        screen.confirm_prompt(4.0).as_deref(),
        Some("Keep these settings? Reverting in 11 s")
    );
    assert_eq!(screen.tick(14.9), None);
    let undo = screen.tick(15.0).expect("time is up");
    assert_eq!(
        affected_sections(&undo),
        [Section::Video, Section::Graphics]
    );
    assert_eq!(applied_settings(&screen).unwrap(), before);
    assert_eq!(pending_settings(&screen).unwrap(), before);
}

#[test]
fn what_changed_says_which_parts_of_the_game_to_reapply() {
    let mut screen = default_screen();
    assert!(affected_sections(&screen.changed()).is_empty());
    screen.set_text("master_volume", "0.5").unwrap();
    screen.set_text("sfx_volume", "0.5").unwrap();
    assert_eq!(affected_sections(&screen.changed()), [Section::Audio]);
    screen.set_text("high_contrast", "on").unwrap();
    screen.set_text("bloom", "off").unwrap();
    screen.set_text("fov", "70").unwrap();
    assert_eq!(
        affected_sections(&screen.changed()),
        [
            Section::Video,
            Section::Graphics,
            Section::Audio,
            Section::Accessibility
        ]
    );
    let unknown = [Change {
        id: "nope".into(),
        from: "a".into(),
        to: "b".into(),
    }];
    assert!(affected_sections(&unknown).is_empty());
}

#[test]
fn a_quality_preset_sets_every_graphics_row_and_names_itself() {
    let mut screen = default_screen();
    // a fresh install is the default tier
    assert_eq!(quality_label_of(&screen), "high");
    for tier in QualityTier::ALL {
        set_quality_preset(&mut screen, tier).unwrap();
        assert_eq!(quality_label_of(&screen), tier.name());
        assert_eq!(
            pending_settings(&screen).unwrap().quality,
            QualitySettings::preset(tier)
        );
    }
    // the applied values did not move
    assert_eq!(applied_settings(&screen).unwrap(), GameSettings::default());
    // changing one value after a preset makes it Custom, and the preset's other values stay
    set_quality_preset(&mut screen, QualityTier::Medium).unwrap();
    screen.step("bloom", Step::Increase).unwrap();
    assert_eq!(quality_label_of(&screen), "Custom");
    assert_eq!(screen.pending("shadows"), Some("low"));
    screen.set_text("bloom", "on").unwrap();
    assert_eq!(quality_label_of(&screen), "medium");
    // the other tabs are untouched
    assert_eq!(screen.pending("fov"), Some("80"));
    // an edit to a non-graphics row does not change the label
    screen.set_text("fov", "100").unwrap();
    assert_eq!(quality_label_of(&screen), "medium");
}

#[test]
fn a_preset_cannot_be_set_while_a_confirmation_is_open() {
    let mut screen = default_screen();
    screen.set_text("window_mode", "borderless").unwrap();
    screen.apply(0.0).unwrap();
    assert_eq!(
        set_quality_preset(&mut screen, QualityTier::Low),
        Err(SettingsError::Locked)
    );
    assert_eq!(quality_label_of(&screen), "high");
    assert_eq!(screen.pending("shadows"), Some("medium")); // the default tier
}

#[test]
fn rows_read_the_way_a_player_expects() {
    let screen = default_screen();
    let show = |id| screen.display_value(id).unwrap();
    assert_eq!(show("window_mode"), "Windowed");
    assert_eq!(show("resolution"), "1920x1080");
    assert_eq!(show("vsync"), "On");
    assert_eq!(show("fov"), "80°");
    assert_eq!(show("aspect"), "Expand");
    assert_eq!(show("ui_scale"), "100%");
    assert_eq!(show("shadows"), "Medium");
    assert_eq!(show("anisotropy"), "8x");
    assert_eq!(show("anti_aliasing"), "Msaa2");
    assert_eq!(show("render_scale"), "100%");
    assert_eq!(show("view_distance"), "100%");
    assert_eq!(show("particle_density"), "85%");
    assert_eq!(show("lod_bias"), "0");
    assert_eq!(show("max_decals"), "400");
    assert_eq!(show("dynamic_resolution"), "Off");
    assert_eq!(show("master_volume"), "80%");
    assert_eq!(show("music_volume"), "60%");
    assert_eq!(show("voice_volume"), "100%");
    assert_eq!(show("text_scale"), "100%");
    assert_eq!(show("subtitle_size"), "Medium");
    assert_eq!(show("colorblind"), "Off");
    assert_eq!(show("crouch"), "Hold");
    assert_eq!(show("reduce_motion"), "Off");
    let labels: Vec<_> = screen.rows_in(1).iter().map(|r| r.label.as_str()).collect();
    assert_eq!(labels[0], "Shadows");
    assert_eq!(labels[2], "Anisotropic filtering");
    assert_eq!(humanize_id("some_new_option"), "Some new option");
    assert_eq!(quality_label("some_new_option"), "Some new option");
}

#[test]
fn the_defaults_of_the_rows_are_the_defaults_of_the_settings() {
    let screen = default_screen();
    for desc in QualitySettings::options() {
        assert_eq!(
            screen.default_of(desc.name),
            QualitySettings::default().get(desc.name).as_deref(),
            "{}",
            desc.name
        );
    }
    for id in OPTION_NAMES {
        assert_eq!(
            screen.default_of(id),
            AccessibilitySettings::default().get(id).as_deref(),
            "{id}"
        );
    }
    let core = Settings::default();
    assert_eq!(
        screen.default_of("master_volume"),
        Some(core.master_volume.to_string().as_str())
    );
    assert_eq!(
        screen.default_of("music_volume"),
        Some(core.music_volume.to_string().as_str())
    );
    assert_eq!(
        screen.default_of("sfx_volume"),
        Some(core.sfx_volume.to_string().as_str())
    );
    assert_eq!(screen.default_of("voice_volume"), Some("1"));
    assert_eq!(screen.default_of("ui_volume"), Some("1"));
    assert_eq!(screen.default_of("vsync"), Some(on_off(core.vsync)));
    assert_eq!(screen.default_of("fov"), Some("80"));
    assert_eq!(DEFAULT_FOV, 80.0);
    // a fresh screen has nothing to apply and every row at its default
    assert!(!screen.has_changes());
    for tab in 0..4 {
        assert!(screen.view(tab).iter().all(|r| r.is_default));
    }
    // rows with nothing special: all of the pending values are the defaults
    assert_eq!(pending_settings(&screen).unwrap(), GameSettings::default());
    assert_eq!(GameSettings::default().video.fov, DEFAULT_FOV);
    assert_eq!(
        GameSettings::default().video.window_mode,
        WindowMode::Windowed
    );
    assert!(GameSettings::default().video.vsync);
    assert_eq!(GameSettings::default().video.resolution, (1920, 1080));
    assert_eq!(UI_SCALE_RANGE, (0.5, 1.0));
}

#[test]
fn the_volumes_reach_the_mixer_and_come_back() {
    let mut audio = NullAudio::new();
    assert_eq!(
        AudioSettings::read(&audio),
        AudioSettings {
            master: 1.0,
            music: 1.0,
            sfx: 1.0,
            voice: 1.0,
            ui: 1.0
        }
    );
    let v = AudioSettings {
        master: 0.3,
        music: 0.0,
        sfx: 0.6,
        voice: 0.9,
        ui: 0.1,
    };
    v.apply(&mut audio);
    assert_eq!(AudioSettings::read(&audio), v);
    assert_eq!(audio.bus_volume(Bus::Master), 0.3);
    assert_eq!(audio.bus_volume(Bus::Music), 0.0);
    assert_eq!(audio.bus_volume(Bus::Sfx), 0.6);
    assert_eq!(audio.bus_volume(Bus::Voice), 0.9);
    assert_eq!(audio.bus_volume(Bus::Ui), 0.1);
    for bus in Bus::ALL {
        assert!(v.volume(bus).is_some());
    }
    // the screen's applied volumes go straight to the mixer
    let mut screen = default_screen();
    screen.set_text("voice_volume", "0.4").unwrap();
    screen.apply(0.0).unwrap();
    applied_settings(&screen).unwrap().audio.apply(&mut audio);
    assert_eq!(audio.bus_volume(Bus::Voice), 0.4);
    assert_eq!(
        audio.bus_volume(Bus::Master),
        AudioSettings::default().master
    );
}

#[test]
fn the_settings_file_values_go_in_and_out() {
    let core = Settings {
        vsync: false,
        fullscreen: true,
        master_volume: 0.25,
        music_volume: 0.5,
        sfx_volume: 0.75,
    };
    let s = GameSettings::from_core(&core);
    assert!(!s.video.vsync);
    assert_eq!(s.video.window_mode, WindowMode::Borderless);
    assert_eq!(
        (s.audio.master, s.audio.music, s.audio.sfx),
        (0.25, 0.5, 0.75)
    );
    // the rest is the defaults
    assert_eq!(s.quality, QualitySettings::default());
    assert_eq!(s.audio.voice, 1.0);
    let windowed = GameSettings::from_core(&Settings::default());
    assert_eq!(windowed.video.window_mode, WindowMode::Windowed);
    assert!(windowed.video.vsync);
    assert_eq!(windowed, GameSettings::default());

    let mut out = Settings::default();
    s.write_core(&mut out);
    assert_eq!(out, core);
    let mut exclusive = s.clone();
    exclusive.video.window_mode = WindowMode::Exclusive;
    exclusive.write_core(&mut out);
    assert!(out.fullscreen);
    exclusive.video.window_mode = WindowMode::Windowed;
    exclusive.write_core(&mut out);
    assert!(!out.fullscreen);
}

#[test]
fn the_window_request_carries_the_mode_and_the_resolution() {
    let v = VideoSettings {
        window_mode: WindowMode::Exclusive,
        resolution: (2560, 1440),
        ..VideoSettings::default()
    };
    let r = v.display_request();
    assert_eq!(r.mode, WindowMode::Exclusive);
    assert_eq!(r.resolution, Some((2560, 1440)));
    assert_eq!(r.monitor, None);
    assert_eq!(r.refresh_hz, None);
}

#[test]
fn the_screen_can_be_driven_with_navigation_inputs_alone() {
    use ss2_core::input::NavInput;
    let mut screen = default_screen();
    // Video tab: focus the vsync row (third) and flip it, then change the tab and lower the music volume
    screen.press(NavInput::Down).unwrap();
    screen.press(NavInput::Down).unwrap();
    assert_eq!(screen.focused(), "vsync");
    assert_eq!(screen.press(NavInput::Activate), Ok(Nav::Changed));
    assert_eq!(screen.pending("vsync"), Some("off"));
    screen.next_tab();
    screen.next_tab();
    assert_eq!(screen.focused(), "master_volume");
    screen.press(NavInput::Down).unwrap();
    assert_eq!(screen.press(NavInput::Left), Ok(Nav::Changed));
    assert_eq!(screen.pending("music_volume"), Some("0.5"));
    assert_eq!(screen.changed().len(), 2);
    let out = screen.apply(0.0).unwrap();
    assert_eq!(out.changes.len(), 2);
    let now = applied_settings(&screen).unwrap();
    assert!(!now.video.vsync);
    assert_eq!(now.audio.music, 0.5);
}
