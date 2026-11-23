use super::*;

fn words(w: &[&str]) -> RowKind {
    RowKind::Choice(w.iter().map(|s| (*s).to_owned()).collect())
}

fn range(min: f32, max: f32, step: f32) -> RowKind {
    RowKind::Range { min, max, step }
}

/// Three tabs: video (a choice, a range, a count, a toggle, a risky choice, a two-word choice), audio (a percentage), more (a suffix).
fn screen() -> SettingsScreen {
    let video = TabSpec::new(
        "Video",
        vec![
            RowSpec::new(
                "shadows",
                "Shadows",
                words(&["off", "low", "medium", "high"]),
                "high",
            ),
            RowSpec::new("scale", "Render scale", range(0.5, 1.0, 0.05), "1")
                .format(ValueFormat::Percent),
            RowSpec::new(
                "decals",
                "Decals",
                RowKind::Count {
                    min: 0,
                    max: 1000,
                    step: 50,
                },
                "400",
            ),
            RowSpec::new("bloom", "Bloom", RowKind::Toggle, "ON"),
            RowSpec::new(
                "mode",
                "Window mode",
                words(&["windowed", "borderless", "fullscreen"]),
                "windowed",
            )
            .risky(),
            RowSpec::new("crouch", "Crouch", words(&["hold", "toggle"]), "hold"),
        ],
    );
    let audio = TabSpec::new(
        "Audio",
        vec![
            RowSpec::new("volume", "Volume", range(0.0, 1.0, 0.1), "0.8")
                .format(ValueFormat::Percent),
        ],
    );
    let more = TabSpec::new(
        "More",
        vec![
            RowSpec::new("fov", "Field of view", range(30.0, 120.0, 5.0), "80")
                .format(ValueFormat::Suffix("°")),
            RowSpec::new(
                "aniso",
                "Anisotropy",
                words(&["1", "2", "4", "8", "16"]),
                "8",
            )
            .format(ValueFormat::Suffix("x")),
            RowSpec::new(
                "size",
                "Subtitle size",
                words(&["small", "extra_large"]),
                "small",
            ),
        ],
    );
    SettingsScreen::new(vec![video, audio, more], 15.0).unwrap()
}

fn build_error(tabs: Vec<TabSpec>, secs: f32) -> String {
    match SettingsScreen::new(tabs, secs) {
        Err(SettingsError::Build(m)) => m,
        other => panic!("expected a build error, got {other:?}"),
    }
}

fn one_row(row: RowSpec) -> Vec<TabSpec> {
    vec![TabSpec::new("T", vec![row])]
}

#[test]
fn a_new_screen_shows_the_defaults_in_canonical_spelling_and_has_nothing_pending() {
    let s = screen();
    assert_eq!(s.pending("bloom"), Some("on")); // "ON" in the description
    assert_eq!(s.applied("bloom"), Some("on"));
    assert_eq!(s.default_of("scale"), Some("1"));
    assert_eq!(s.pending("shadows"), Some("high"));
    assert!(!s.has_changes());
    assert!(s.changed().is_empty());
    assert!(!s.is_confirming());
    assert_eq!(s.tab_names(), ["Video", "Audio", "More"]);
    assert_eq!(s.rows_in(0).len(), 6);
    assert_eq!(s.rows_in(1).len(), 1);
    assert_eq!(s.rows_in(2).len(), 3);
    assert!(s.rows_in(3).is_empty());
    assert_eq!(s.tab(), 0);
    assert_eq!(s.focused(), "shadows");
    assert_eq!(s.row("fov").unwrap().label, "Field of view");
    assert!(s.row("nope").is_none());
    assert_eq!(s.pending("nope"), None);
    assert_eq!(s.applied("nope"), None);
    assert_eq!(s.default_of("nope"), None);
}

#[test]
fn a_description_that_is_wrong_is_refused_with_the_reason() {
    let r = |kind| RowSpec::new("x", "X", kind, "1");
    assert!(build_error(vec![], 5.0).contains("no tab"));
    assert!(build_error(vec![TabSpec::new("Empty", vec![])], 5.0).contains("no row"));
    // names
    assert!(
        build_error(one_row(RowSpec::new("", "X", RowKind::Toggle, "on")), 5.0)
            .contains("empty or has spaces")
    );
    assert!(
        build_error(one_row(RowSpec::new(" a", "X", RowKind::Toggle, "on")), 5.0)
            .contains("empty or has spaces")
    );
    let twice = vec![
        TabSpec::new("A", vec![RowSpec::new("x", "X", RowKind::Toggle, "on")]),
        TabSpec::new("B", vec![RowSpec::new("x", "Y", RowKind::Toggle, "on")]),
    ];
    assert!(build_error(twice, 5.0).contains("used twice"));
    // defaults
    let m = build_error(
        one_row(RowSpec::new("x", "X", RowKind::Toggle, "maybe")),
        5.0,
    );
    assert!(m.contains("default") && m.contains("on or off"), "{m}");
    assert!(
        build_error(
            one_row(RowSpec::new("x", "X", range(0.0, 1.0, 0.1), "2")),
            5.0
        )
        .contains("default")
    );
    // kinds
    assert!(build_error(one_row(r(words(&[]))), 5.0).contains("at least one word"));
    assert!(build_error(one_row(r(words(&["a", "A"]))), 5.0).contains("twice"));
    assert!(build_error(one_row(r(words(&["a", ""]))), 5.0).contains("empty"));
    assert!(build_error(one_row(r(words(&["a", " b"]))), 5.0).contains("empty or has spaces"));
    assert!(build_error(one_row(r(range(1.0, 1.0, 0.1))), 5.0).contains("min below max"));
    assert!(build_error(one_row(r(range(2.0, 1.0, 0.1))), 5.0).contains("min below max"));
    assert!(build_error(one_row(r(range(0.0, 1.0, 0.0))), 5.0).contains("step"));
    assert!(build_error(one_row(r(range(0.0, 1.0, -0.1))), 5.0).contains("step"));
    assert!(build_error(one_row(r(range(0.0, 1.0, 1.5))), 5.0).contains("step"));
    assert!(build_error(one_row(r(range(f32::NAN, 1.0, 0.1))), 5.0).contains("finite"));
    assert!(build_error(one_row(r(range(0.0, f32::INFINITY, 0.1))), 5.0).contains("finite"));
    assert!(build_error(one_row(r(range(0.0, 1.0, f32::NAN))), 5.0).contains("finite"));
    let count = |min, max, step| r(RowKind::Count { min, max, step });
    assert!(build_error(one_row(count(5, 5, 1)), 5.0).contains("min below max"));
    assert!(build_error(one_row(count(6, 5, 1)), 5.0).contains("min below max"));
    assert!(build_error(one_row(count(0, 5, 0)), 5.0).contains("step"));
    // the time
    let ok = || one_row(RowSpec::new("x", "X", RowKind::Toggle, "on"));
    for secs in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert!(
            build_error(ok(), secs).contains("confirmation time"),
            "{secs}"
        );
    }
    // the boundaries that are fine
    assert!(SettingsScreen::new(ok(), 0.5).is_ok());
    assert!(SettingsScreen::new(one_row(r(range(0.0, 1.0, 1.0))), 5.0).is_ok());
    assert!(SettingsScreen::new(one_row(r(words(&["1"]))), 5.0).is_ok());
    assert!(
        SettingsScreen::new(
            one_row(r(RowKind::Count {
                min: 0,
                max: 1,
                step: 1
            })),
            5.0
        )
        .is_ok()
    );
}

#[test]
fn what_a_row_accepts_and_how_it_spells_it() {
    let toggle = RowKind::Toggle;
    for on in ["on", "ON", " true ", "Yes", "1"] {
        assert_eq!(toggle.canonical(on).as_deref(), Some("on"), "{on}");
    }
    for off in ["off", "OFF", "false", " no", "0"] {
        assert_eq!(toggle.canonical(off).as_deref(), Some("off"), "{off}");
    }
    for bad in ["", "2", "maybe", "on off"] {
        assert_eq!(toggle.canonical(bad), None, "{bad}");
    }

    let choice = words(&["off", "extra_large"]);
    assert_eq!(
        choice.canonical("  EXTRA_LARGE ").as_deref(),
        Some("extra_large")
    );
    assert_eq!(choice.canonical("off").as_deref(), Some("off"));
    for bad in ["", "extra large", "of", "offf"] {
        assert_eq!(choice.canonical(bad), None, "{bad}");
    }

    let r = range(0.5, 1.0, 0.05);
    assert_eq!(r.canonical("0.50").as_deref(), Some("0.5"));
    assert_eq!(r.canonical(" 1 ").as_deref(), Some("1"));
    assert_eq!(r.canonical("1.0").as_deref(), Some("1"));
    assert_eq!(r.canonical("0.5").as_deref(), Some("0.5"));
    for bad in [
        "0.49", "1.01", "nan", "NaN", "inf", "-inf", "abc", "", "0,7", "1e9",
    ] {
        assert_eq!(r.canonical(bad), None, "{bad}");
    }
    // a minus zero reads as zero
    let z = range(-1.0, 1.0, 0.5);
    assert_eq!(z.canonical("-0").as_deref(), Some("0"));
    assert_eq!(z.canonical("-0.0").as_deref(), Some("0"));
    assert_eq!(z.canonical("-1").as_deref(), Some("-1"));
    assert_eq!(z.canonical("-1.01"), None);

    let c = RowKind::Count {
        min: 10,
        max: 1000,
        step: 10,
    };
    assert_eq!(c.canonical(" 10 ").as_deref(), Some("10"));
    assert_eq!(c.canonical("1000").as_deref(), Some("1000"));
    assert_eq!(c.canonical("0100").as_deref(), Some("100"));
    for bad in ["9", "1001", "-5", "1.5", "ten", "", "1e3"] {
        assert_eq!(c.canonical(bad), None, "{bad}");
    }
}

#[test]
fn the_refusal_names_the_row_and_what_it_takes() {
    assert_eq!(words(&["a", "b"]).expected(), "a or b");
    assert_eq!(words(&["a", "b", "c"]).expected(), "one of a, b, c");
    assert_eq!(words(&["a"]).expected(), "one of a");
    assert_eq!(range(0.5, 1.0, 0.1).expected(), "a number from 0.5 to 1");
    assert_eq!(
        RowKind::Count {
            min: 0,
            max: 1000,
            step: 1
        }
        .expected(),
        "a whole number from 0 to 1000"
    );
    assert_eq!(RowKind::Toggle.expected(), "on or off");

    let mut s = screen();
    let e = s.set_text("scale", "2").unwrap_err();
    assert_eq!(e.to_string(), "scale takes a number from 0.5 to 1");
    assert_eq!(
        e,
        SettingsError::Refused {
            row: "scale".into(),
            expected: "a number from 0.5 to 1".into()
        }
    );
    assert_eq!(
        s.set_text("shadows", "ultra").unwrap_err().to_string(),
        "shadows takes one of off, low, medium, high"
    );
    assert_eq!(
        s.set_text("crouch", "x").unwrap_err().to_string(),
        "crouch takes hold or toggle"
    );
    assert_eq!(
        s.set_text("nope", "1").unwrap_err(),
        SettingsError::UnknownRow("nope".into())
    );
    assert!(
        s.set_text("nope", "1")
            .unwrap_err()
            .to_string()
            .contains("nope")
    );
    assert!(
        SettingsError::Locked
            .to_string()
            .contains("keep these settings")
    );
    assert!(SettingsError::BadTime.to_string().contains("finite"));
    assert!(
        SettingsError::Build("why".into())
            .to_string()
            .contains("why")
    );
}

#[test]
fn an_edit_is_pending_and_nothing_is_applied_until_apply() {
    let mut s = screen();
    assert_eq!(s.set_text("shadows", " LOW ").unwrap(), "low");
    assert_eq!(s.set_text("scale", "0.80").unwrap(), "0.8");
    assert_eq!(s.pending("shadows"), Some("low"));
    assert_eq!(s.applied("shadows"), Some("high"));
    assert_eq!(s.applied("scale"), Some("1"));
    assert!(s.has_changes());
    assert!(s.is_changed("shadows"));
    assert!(!s.is_changed("bloom"));
    assert!(!s.is_changed("nope"));
    // The list of changes is in screen order and carries both values.
    s.set_text("bloom", "off").unwrap();
    s.set_text("fov", "90").unwrap();
    assert_eq!(
        s.changed(),
        vec![
            Change {
                id: "shadows".into(),
                from: "high".into(),
                to: "low".into()
            },
            Change {
                id: "scale".into(),
                from: "1".into(),
                to: "0.8".into()
            },
            Change {
                id: "bloom".into(),
                from: "on".into(),
                to: "off".into()
            },
            Change {
                id: "fov".into(),
                from: "80".into(),
                to: "90".into()
            },
        ]
    );
    assert_eq!(s.changed_in_tab(0), 3);
    assert_eq!(s.changed_in_tab(1), 0);
    assert_eq!(s.changed_in_tab(2), 1);
    assert_eq!(s.changed_in_tab(9), 0);
}

#[test]
fn setting_a_value_back_to_what_is_applied_is_not_a_change() {
    let mut s = screen();
    s.set_text("scale", "0.7").unwrap();
    assert!(s.has_changes());
    s.set_text("scale", "1.000").unwrap();
    assert!(!s.has_changes());
    assert!(s.changed().is_empty());
}

#[test]
fn text_that_is_not_a_value_never_changes_anything() {
    let mut s = screen();
    s.set_text("scale", "0.7").unwrap();
    let before = (s.pending("scale").map(str::to_owned), s.changed());
    for bad in ["", "abc", "NaN", "inf", "0.4", "1.1", "-1", "0,9", "1e9"] {
        assert!(s.set_text("scale", bad).is_err(), "{bad}");
    }
    assert!(s.set_text("shadows", "ultra").is_err());
    assert!(s.set_text("decals", "1001").is_err());
    assert!(s.set_text("decals", "-1").is_err());
    assert!(s.set_text("bloom", "maybe").is_err());
    assert_eq!(s.pending("scale"), before.0.as_deref());
    assert_eq!(s.changed(), before.1);
    assert_eq!(s.pending("shadows"), Some("high"));
    assert_eq!(s.pending("decals"), Some("400"));
    assert_eq!(s.pending("bloom"), Some("on"));
}

#[test]
fn a_choice_steps_around_and_wraps_both_ways() {
    let mut s = screen();
    let mut seen = Vec::new();
    for _ in 0..5 {
        seen.push(s.step("shadows", Step::Increase).unwrap());
    }
    assert_eq!(seen, ["off", "low", "medium", "high", "off"]);
    assert_eq!(s.step("shadows", Step::Decrease).unwrap(), "high");
    assert_eq!(s.step("shadows", Step::Decrease).unwrap(), "medium");
    // two words: both directions reach the other word
    assert_eq!(s.step("crouch", Step::Increase).unwrap(), "toggle");
    assert_eq!(s.step("crouch", Step::Increase).unwrap(), "hold");
    assert_eq!(s.step("crouch", Step::Decrease).unwrap(), "toggle");
    assert_eq!(s.step("crouch", Step::Decrease).unwrap(), "hold");
    assert_eq!(s.applied("shadows"), Some("high"));
    assert_eq!(
        s.step("nope", Step::Increase).unwrap_err(),
        SettingsError::UnknownRow("nope".into())
    );
}

#[test]
fn a_toggle_flips_whichever_way_it_is_stepped() {
    let mut s = screen();
    assert_eq!(s.step("bloom", Step::Increase).unwrap(), "off");
    assert_eq!(s.step("bloom", Step::Increase).unwrap(), "on");
    assert_eq!(s.step("bloom", Step::Decrease).unwrap(), "off");
    assert_eq!(s.step("bloom", Step::Decrease).unwrap(), "on");
}

#[test]
fn a_range_steps_on_its_grid_without_float_noise_and_stops_at_the_ends() {
    let mut s = screen();
    // from the top, down to the bottom: every text is a short decimal
    let mut down = Vec::new();
    for _ in 0..12 {
        down.push(s.step("scale", Step::Decrease).unwrap());
    }
    assert_eq!(
        down,
        [
            "0.95", "0.9", "0.85", "0.8", "0.75", "0.7", "0.65", "0.6", "0.55", "0.5", "0.5", "0.5"
        ]
    );
    let mut up = Vec::new();
    for _ in 0..11 {
        up.push(s.step("scale", Step::Increase).unwrap());
    }
    assert_eq!(
        up,
        [
            "0.55", "0.6", "0.65", "0.7", "0.75", "0.8", "0.85", "0.9", "0.95", "1", "1"
        ]
    );
    // a 0..1 range in tenths, walked 10 times
    for _ in 0..10 {
        s.step("volume", Step::Decrease).unwrap();
    }
    assert_eq!(s.pending("volume"), Some("0"));
    let mut texts = Vec::new();
    for _ in 0..10 {
        texts.push(s.step("volume", Step::Increase).unwrap());
    }
    assert_eq!(
        texts,
        [
            "0.1", "0.2", "0.3", "0.4", "0.5", "0.6", "0.7", "0.8", "0.9", "1"
        ]
    );
    // a range whose end is not on the grid ends on the end
    let tab = TabSpec::new("T", vec![RowSpec::new("r", "R", range(0.0, 1.0, 0.3), "0")]);
    let mut s2 = SettingsScreen::new(vec![tab], 5.0).unwrap();
    let up: Vec<_> = (0..5)
        .map(|_| s2.step("r", Step::Increase).unwrap())
        .collect();
    assert_eq!(up, ["0.3", "0.6", "0.9", "1", "1"]);
    // ... and from the end steps down the grid, not by a whole step from the end
    assert_eq!(s2.step("r", Step::Decrease).unwrap(), "0.9");
}

#[test]
fn a_range_value_between_grid_points_moves_to_the_next_point_in_the_direction_asked() {
    let mut s = screen();
    s.set_text("scale", "0.87").unwrap();
    assert_eq!(s.step("scale", Step::Increase).unwrap(), "0.9");
    s.set_text("scale", "0.87").unwrap();
    assert_eq!(s.step("scale", Step::Decrease).unwrap(), "0.85");
    // a value on the grid, written with a little noise, still moves one whole step
    s.set_text("scale", "0.8500001").unwrap();
    assert_eq!(s.step("scale", Step::Increase).unwrap(), "0.9");
    s.set_text("scale", "0.8499999").unwrap();
    assert_eq!(s.step("scale", Step::Decrease).unwrap(), "0.8");
    // the ends
    s.set_text("scale", "0.52").unwrap();
    assert_eq!(s.step("scale", Step::Decrease).unwrap(), "0.5");
    s.set_text("scale", "0.99").unwrap();
    assert_eq!(s.step("scale", Step::Increase).unwrap(), "1");
    // a range that does not start at zero: the grid starts at min
    s.set_text("fov", "82").unwrap();
    assert_eq!(s.step("fov", Step::Increase).unwrap(), "85");
    s.set_text("fov", "82").unwrap();
    assert_eq!(s.step("fov", Step::Decrease).unwrap(), "80");
    s.set_text("fov", "120").unwrap();
    assert_eq!(s.step("fov", Step::Increase).unwrap(), "120");
    s.set_text("fov", "30").unwrap();
    assert_eq!(s.step("fov", Step::Decrease).unwrap(), "30");
    assert_eq!(s.step("fov", Step::Increase).unwrap(), "35");
}

#[test]
fn a_count_steps_by_its_step_and_stops_at_the_ends() {
    let mut s = screen();
    assert_eq!(s.step("decals", Step::Increase).unwrap(), "450");
    assert_eq!(s.step("decals", Step::Decrease).unwrap(), "400");
    s.set_text("decals", "980").unwrap();
    assert_eq!(s.step("decals", Step::Increase).unwrap(), "1000");
    assert_eq!(s.step("decals", Step::Increase).unwrap(), "1000");
    s.set_text("decals", "30").unwrap();
    assert_eq!(s.step("decals", Step::Decrease).unwrap(), "0");
    assert_eq!(s.step("decals", Step::Decrease).unwrap(), "0");
    s.set_text("decals", "50").unwrap();
    assert_eq!(s.step("decals", Step::Decrease).unwrap(), "0");
    // a count that starts above zero does not go below its minimum
    let tab = TabSpec::new(
        "T",
        vec![RowSpec::new(
            "c",
            "C",
            RowKind::Count {
                min: 10,
                max: 40,
                step: 25,
            },
            "20",
        )],
    );
    let mut s2 = SettingsScreen::new(vec![tab], 5.0).unwrap();
    assert_eq!(s2.step("c", Step::Decrease).unwrap(), "10");
    assert_eq!(s2.step("c", Step::Increase).unwrap(), "35");
    assert_eq!(s2.step("c", Step::Increase).unwrap(), "40");
    // a huge count does not overflow
    let tab = TabSpec::new(
        "T",
        vec![RowSpec::new(
            "c",
            "C",
            RowKind::Count {
                min: 0,
                max: u32::MAX,
                step: u32::MAX,
            },
            "4294967295",
        )],
    );
    let mut s3 = SettingsScreen::new(vec![tab], 5.0).unwrap();
    assert_eq!(s3.step("c", Step::Increase).unwrap(), "4294967295");
    assert_eq!(s3.step("c", Step::Decrease).unwrap(), "0");
}

#[test]
fn revert_reset_and_load() {
    let mut s = screen();
    s.set_text("shadows", "off").unwrap();
    s.set_text("fov", "100").unwrap();
    s.revert();
    assert!(!s.has_changes());
    assert_eq!(s.pending("shadows"), Some("high"));
    assert_eq!(s.pending("fov"), Some("80"));

    // load: the game's current value is both applied and pending
    s.load("fov", "95").unwrap();
    assert_eq!(s.applied("fov"), Some("95"));
    assert_eq!(s.pending("fov"), Some("95"));
    assert!(!s.has_changes());
    assert!(s.load("fov", "500").is_err());
    assert_eq!(s.applied("fov"), Some("95"));
    assert!(s.load("nope", "1").is_err());

    // reset is an edit: it needs apply
    s.reset_row("fov").unwrap();
    assert_eq!(s.pending("fov"), Some("80"));
    assert_eq!(s.applied("fov"), Some("95"));
    assert!(s.is_changed("fov"));
    s.revert();
    assert_eq!(s.pending("fov"), Some("95"));

    s.set_text("shadows", "low").unwrap();
    s.set_text("scale", "0.6").unwrap();
    s.reset_all().unwrap();
    for id in [
        "shadows", "scale", "decals", "bloom", "mode", "volume", "fov",
    ] {
        assert_eq!(s.pending(id), s.default_of(id), "{id}");
    }
    assert_eq!(s.applied("fov"), Some("95"));
    assert!(s.is_changed("fov"));
    assert!(!s.is_changed("shadows")); // the default is also what is applied
    assert!(s.reset_row("nope").is_err());
}

#[test]
fn apply_makes_the_edits_current_and_says_what_changed() {
    let mut s = screen();
    s.set_text("shadows", "low").unwrap();
    s.set_text("decals", "100").unwrap();
    let out = s.apply(10.0).unwrap();
    assert_eq!(
        out.changes
            .iter()
            .map(|c| (c.id.as_str(), c.from.as_str(), c.to.as_str()))
            .collect::<Vec<_>>(),
        [("shadows", "high", "low"), ("decals", "400", "100")]
    );
    assert_eq!(out.confirm_until, None);
    assert!(!s.is_confirming());
    assert_eq!(s.applied("shadows"), Some("low"));
    assert_eq!(s.applied("decals"), Some("100"));
    assert!(!s.has_changes());
    // nothing pending: nothing happens
    let again = s.apply(11.0).unwrap();
    assert!(again.changes.is_empty());
    assert_eq!(again.confirm_until, None);
    // revert after apply keeps what was applied
    s.revert();
    assert_eq!(s.pending("shadows"), Some("low"));
}

#[test]
fn what_you_edit_is_what_you_get_after_apply() {
    // Round trip over a value of every kind: set, apply, read back.
    let mut s = screen();
    let edits = [
        ("shadows", "Medium", "medium"),
        ("scale", "0.65", "0.65"),
        ("decals", "250", "250"),
        ("bloom", "no", "off"),
        ("mode", "borderless", "borderless"),
        ("crouch", "TOGGLE", "toggle"),
        ("volume", "0.3", "0.3"),
        ("fov", "110", "110"),
        ("aniso", "16", "16"),
        ("size", "extra_large", "extra_large"),
    ];
    for (id, text, _) in edits {
        s.set_text(id, text).unwrap();
    }
    let out = s.apply(0.0).unwrap();
    assert_eq!(out.changes.len(), edits.len());
    for (id, _, want) in edits {
        assert_eq!(s.applied(id), Some(want), "{id}");
        assert_eq!(s.pending(id), Some(want), "{id}");
    }
}

#[test]
fn a_risky_change_opens_a_confirmation_that_ends_at_now_plus_the_time() {
    let mut s = screen();
    s.set_text("mode", "fullscreen").unwrap();
    s.set_text("shadows", "low").unwrap();
    let out = s.apply(100.0).unwrap();
    assert_eq!(out.confirm_until, Some(115.0));
    assert!(s.is_confirming());
    assert_eq!(s.confirm_remaining(100.0), Some(15.0));
    assert_eq!(s.confirm_remaining(110.0), Some(5.0));
    assert_eq!(s.confirm_remaining(115.0), Some(0.0));
    assert_eq!(s.confirm_remaining(500.0), Some(0.0)); // never negative
    // the new values are in force while the question is open
    assert_eq!(s.applied("mode"), Some("fullscreen"));
    // only a change of a non-risky row does not ask
    let mut s = screen();
    s.set_text("shadows", "low").unwrap();
    assert_eq!(s.apply(100.0).unwrap().confirm_until, None);
    assert_eq!(s.confirm_remaining(100.0), None);
    assert_eq!(s.confirm_prompt(100.0), None);
    // setting a risky row to the value it has is not a change, so it does not ask
    let mut s = screen();
    s.set_text("mode", "windowed").unwrap();
    s.set_text("shadows", "low").unwrap();
    assert_eq!(s.apply(100.0).unwrap().confirm_until, None);
}

#[test]
fn the_player_keeps_the_settings() {
    let mut s = screen();
    s.set_text("mode", "fullscreen").unwrap();
    s.apply(0.0).unwrap();
    assert!(s.confirm());
    assert!(!s.is_confirming());
    assert!(!s.confirm()); // nothing to answer any more
    assert_eq!(s.applied("mode"), Some("fullscreen"));
    // the time running out afterwards does nothing
    assert_eq!(s.tick(1000.0), None);
    assert_eq!(s.applied("mode"), Some("fullscreen"));
    // editing works again
    s.set_text("mode", "windowed").unwrap();
    assert_eq!(s.apply(2000.0).unwrap().confirm_until, Some(2015.0));
}

#[test]
fn when_the_time_runs_out_the_previous_values_come_back_and_the_changes_to_undo_are_reported() {
    let mut s = screen();
    s.set_text("mode", "fullscreen").unwrap();
    s.set_text("shadows", "low").unwrap();
    s.set_text("scale", "0.7").unwrap();
    s.apply(100.0).unwrap();
    assert_eq!(s.tick(100.0), None);
    assert_eq!(s.tick(114.9), None);
    assert!(s.is_confirming());
    // exactly at the deadline it is over
    let undo = s.tick(115.0).expect("time is up");
    assert_eq!(
        undo,
        vec![
            Change {
                id: "shadows".into(),
                from: "low".into(),
                to: "high".into()
            },
            Change {
                id: "scale".into(),
                from: "0.7".into(),
                to: "1".into()
            },
            Change {
                id: "mode".into(),
                from: "fullscreen".into(),
                to: "windowed".into()
            },
        ]
    );
    assert!(!s.is_confirming());
    assert!(!s.has_changes());
    for id in ["mode", "shadows", "scale"] {
        assert_eq!(s.applied(id), s.default_of(id), "{id}");
        assert_eq!(s.pending(id), s.default_of(id), "{id}");
    }
    assert_eq!(s.tick(200.0), None); // only once
}

#[test]
fn the_player_can_revert_before_the_time_is_up() {
    let mut s = screen();
    s.set_text("mode", "borderless").unwrap();
    s.apply(0.0).unwrap();
    let undo = s.revert_applied();
    assert_eq!(undo.len(), 1);
    assert_eq!(undo[0].from, "borderless");
    assert_eq!(undo[0].to, "windowed");
    assert_eq!(s.applied("mode"), Some("windowed"));
    assert_eq!(s.pending("mode"), Some("windowed"));
    assert!(!s.is_confirming());
    assert!(s.revert_applied().is_empty()); // nothing open
}

#[test]
fn nothing_can_be_edited_while_a_question_is_open() {
    let mut s = screen();
    s.set_text("mode", "borderless").unwrap();
    s.apply(0.0).unwrap();
    assert_eq!(s.set_text("shadows", "low"), Err(SettingsError::Locked));
    assert_eq!(
        s.step("shadows", Step::Increase),
        Err(SettingsError::Locked)
    );
    assert_eq!(s.reset_row("mode"), Err(SettingsError::Locked));
    assert_eq!(s.reset_all(), Err(SettingsError::Locked));
    assert_eq!(s.load("fov", "90"), Err(SettingsError::Locked));
    assert_eq!(s.apply(1.0), Err(SettingsError::Locked));
    assert_eq!(s.press(NavInput::Right), Err(SettingsError::Locked));
    assert_eq!(s.press(NavInput::Activate), Err(SettingsError::Locked));
    assert_eq!(s.pending("shadows"), Some("high"));
    assert_eq!(s.applied("mode"), Some("borderless"));
    assert!(s.is_confirming());
    // moving around is fine
    assert_eq!(s.press(NavInput::Down), Ok(Nav::Moved));
    s.revert(); // nothing is pending, so this changes nothing
    assert_eq!(s.applied("mode"), Some("borderless"));
}

#[test]
fn a_time_that_is_not_a_number_applies_nothing_and_never_ends_the_question() {
    let mut s = screen();
    s.set_text("mode", "borderless").unwrap();
    for t in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(s.apply(t), Err(SettingsError::BadTime));
    }
    assert!(!s.is_confirming());
    assert_eq!(s.applied("mode"), Some("windowed"));
    assert!(s.is_changed("mode"));
    s.apply(0.0).unwrap();
    for t in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(s.tick(t), None);
    }
    assert!(s.is_confirming());
}

#[test]
fn the_prompt_counts_whole_seconds_up() {
    let mut s = screen();
    s.set_text("mode", "borderless").unwrap();
    s.apply(100.0).unwrap();
    let p = |t| s.confirm_prompt(t).unwrap();
    assert_eq!(p(100.0), "Keep these settings? Reverting in 15 s");
    assert_eq!(p(100.2), "Keep these settings? Reverting in 15 s");
    assert_eq!(p(103.5), "Keep these settings? Reverting in 12 s");
    assert_eq!(p(114.0), "Keep these settings? Reverting in 1 s");
    assert_eq!(p(114.9), "Keep these settings? Reverting in 1 s");
    assert_eq!(p(115.0), "Keep these settings? Reverting in 0 s");
    assert_eq!(p(900.0), "Keep these settings? Reverting in 0 s");
}

#[test]
fn values_are_shown_the_way_a_player_reads_them() {
    let mut s = screen();
    let show = |s: &SettingsScreen, id| s.display_value(id).unwrap();
    assert_eq!(show(&s, "bloom"), "On");
    s.set_text("bloom", "off").unwrap();
    assert_eq!(show(&s, "bloom"), "Off");
    assert_eq!(show(&s, "shadows"), "High");
    assert_eq!(show(&s, "mode"), "Windowed");
    assert_eq!(show(&s, "size"), "Small");
    s.set_text("size", "extra_large").unwrap();
    assert_eq!(show(&s, "size"), "Extra large");
    assert_eq!(show(&s, "scale"), "100%");
    s.set_text("scale", "0.85").unwrap();
    assert_eq!(show(&s, "scale"), "85%");
    s.set_text("scale", "0.555").unwrap();
    assert_eq!(show(&s, "scale"), "56%");
    assert_eq!(show(&s, "volume"), "80%");
    assert_eq!(show(&s, "fov"), "80°");
    assert_eq!(show(&s, "aniso"), "8x");
    assert_eq!(show(&s, "decals"), "400");
    assert_eq!(s.display_value("nope"), None);
    // Percent on a count, a suffix on a toggle and a suffix on a count
    let tab = TabSpec::new(
        "T",
        vec![
            RowSpec::new(
                "n",
                "N",
                RowKind::Count {
                    min: 0,
                    max: 9,
                    step: 1,
                },
                "5",
            )
            .format(ValueFormat::Percent),
            RowSpec::new("t", "T", RowKind::Toggle, "on").format(ValueFormat::Suffix("!")),
            RowSpec::new(
                "m",
                "M",
                RowKind::Count {
                    min: 0,
                    max: 9,
                    step: 1,
                },
                "5",
            )
            .format(ValueFormat::Suffix(" ms")),
            RowSpec::new("p", "P", range(0.0, 1.0, 0.5), "0.5"),
            RowSpec::new("q", "Q", range(0.0, 1.0, 0.5), "0.5").format(ValueFormat::Suffix("u")),
            RowSpec::new("w", "W", words(&["a_b"]), "a_b").format(ValueFormat::Percent),
        ],
    );
    let s = SettingsScreen::new(vec![tab], 5.0).unwrap();
    assert_eq!(show(&s, "n"), "5");
    assert_eq!(show(&s, "t"), "On");
    assert_eq!(show(&s, "m"), "5 ms");
    assert_eq!(show(&s, "p"), "0.5");
    assert_eq!(show(&s, "q"), "0.5u");
    assert_eq!(show(&s, "w"), "A b");
}

#[test]
fn the_view_has_everything_a_screen_draws() {
    let mut s = screen();
    s.set_text("scale", "0.5").unwrap();
    s.set_text("decals", "1000").unwrap();
    s.set_text("bloom", "off").unwrap();
    let v = s.view(0);
    assert_eq!(v.len(), 6);
    assert_eq!(
        v.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        ["shadows", "scale", "decals", "bloom", "mode", "crouch"]
    );
    assert_eq!(v[0].label, "Shadows");
    assert_eq!(v[0].value, "High");
    assert!(v[0].focused);
    assert!(!v[0].changed && v[0].is_default);
    assert!(v[0].can_decrease && v[0].can_increase);
    assert!(!v[0].risky);
    // scale at its minimum
    assert_eq!(v[1].value, "50%");
    assert!(v[1].changed && !v[1].is_default);
    assert!(!v[1].can_decrease && v[1].can_increase);
    assert!(!v[1].focused);
    // decals at its maximum
    assert!(v[2].can_decrease && !v[2].can_increase);
    // a toggle can always step
    assert!(v[3].changed);
    assert!(v[3].can_decrease && v[3].can_increase);
    assert!(v[4].risky);
    // another tab: no focus there, and a tab that does not exist is empty
    assert!(s.view(1).iter().all(|r| !r.focused));
    assert!(s.view(7).is_empty());
    // an interior count / range can step both ways
    s.set_text("scale", "0.75").unwrap();
    s.set_text("decals", "500").unwrap();
    let v = s.view(0);
    assert!(v[1].can_decrease && v[1].can_increase);
    assert!(v[2].can_decrease && v[2].can_increase);
    // a count at zero cannot decrease
    s.set_text("decals", "0").unwrap();
    assert!(!s.view(0)[2].can_decrease);
    assert!(s.view(0)[2].can_increase);
    // a range at its maximum
    s.set_text("scale", "1").unwrap();
    assert!(!s.view(0)[1].can_increase);
}

#[test]
fn tabs_and_focus_move_and_wrap() {
    let mut s = screen();
    assert_eq!(s.press(NavInput::Down), Ok(Nav::Moved));
    assert_eq!(s.focused(), "scale");
    assert_eq!(s.press(NavInput::Up), Ok(Nav::Moved));
    assert_eq!(s.press(NavInput::Up), Ok(Nav::Moved)); // wraps to the last row of the tab
    assert_eq!(s.focused(), "crouch");
    assert_eq!(s.press(NavInput::Down), Ok(Nav::Moved)); // and back to the first
    assert_eq!(s.focused(), "shadows");
    // a tab with one row stays on it
    assert!(s.select_tab(1));
    assert_eq!(s.focused(), "volume");
    s.press(NavInput::Down).unwrap();
    assert_eq!(s.focused(), "volume");
    s.press(NavInput::Up).unwrap();
    assert_eq!(s.focused(), "volume");
    // tabs
    assert!(!s.select_tab(1)); // already there
    assert!(!s.select_tab(3)); // out of range
    assert_eq!(s.tab(), 1);
    s.next_tab();
    assert_eq!(s.tab(), 2);
    assert_eq!(s.focused(), "fov"); // first row of the new tab
    s.press(NavInput::Down).unwrap();
    assert_eq!(s.focused(), "aniso");
    s.next_tab();
    assert_eq!(s.tab(), 0);
    assert_eq!(s.focused(), "shadows");
    s.prev_tab();
    assert_eq!(s.tab(), 2);
    s.prev_tab();
    assert_eq!(s.tab(), 1);
    // the focused row is marked in the current tab's view only
    assert!(s.view(1)[0].focused);
    assert!(s.view(0).iter().all(|r| !r.focused));
    // focus_row works inside the tab and refuses other tabs and unknown names
    assert!(!s.focus_row("shadows"));
    assert!(!s.focus_row("nope"));
    assert!(s.select_tab(0));
    assert!(s.focus_row("mode"));
    assert_eq!(s.focused(), "mode");
    assert!(s.view(0)[4].focused);
    assert!(!s.focus_row("fov"));
    assert_eq!(s.focused(), "mode");
}

#[test]
fn left_right_activate_and_back() {
    let mut s = screen();
    assert_eq!(s.press(NavInput::Right), Ok(Nav::Changed));
    assert_eq!(s.pending("shadows"), Some("off")); // high -> wraps to off
    assert_eq!(s.press(NavInput::Left), Ok(Nav::Changed));
    assert_eq!(s.pending("shadows"), Some("high"));
    assert_eq!(s.press(NavInput::Activate), Ok(Nav::Changed));
    assert_eq!(s.pending("shadows"), Some("off"));
    assert!(s.focus_row("bloom"));
    assert_eq!(s.press(NavInput::Activate), Ok(Nav::Changed));
    assert_eq!(s.pending("bloom"), Some("off"));
    assert_eq!(s.press(NavInput::Activate), Ok(Nav::Changed));
    assert_eq!(s.pending("bloom"), Some("on"));
    assert!(s.focus_row("scale"));
    s.press(NavInput::Left).unwrap();
    assert_eq!(s.pending("scale"), Some("0.95"));
    assert_eq!(s.press(NavInput::Back), Ok(Nav::Back));
    assert_eq!(s.pending("scale"), Some("0.95")); // back is for the screen to act on
    // at the end of a range the input is reported as a limit, and nothing changes
    assert!(s.focus_row("decals"));
    s.set_text("decals", "1000").unwrap();
    assert_eq!(s.press(NavInput::Right), Ok(Nav::Limit));
    assert_eq!(s.pending("decals"), Some("1000"));
    assert_eq!(s.press(NavInput::Left), Ok(Nav::Changed));
    s.set_text("decals", "0").unwrap();
    assert_eq!(s.press(NavInput::Left), Ok(Nav::Limit));
    assert_eq!(s.press(NavInput::Right), Ok(Nav::Changed));
}

#[test]
fn a_long_run_of_edits_keeps_every_value_valid_and_applies_only_on_apply() {
    // A small deterministic generator: no clock, no randomness from outside.
    let mut seed = 0x2545_F491_4F6C_DD1Du64;
    let mut next = move |n: usize| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % n as u64) as usize
    };
    let ids = [
        "shadows", "scale", "decals", "bloom", "mode", "crouch", "volume", "fov", "aniso", "size",
    ];
    let junk = [
        "", "abc", "NaN", "-1", "999999", "0.123456", "on", "high", "1", "0.5",
    ];
    let mut s = screen();
    let mut now = 0.0f32;
    for _ in 0..4000 {
        let id = ids[next(ids.len())];
        let before_applied: Vec<_> = ids
            .iter()
            .map(|i| s.applied(i).unwrap().to_owned())
            .collect();
        let before_pending: Vec<_> = ids
            .iter()
            .map(|i| s.pending(i).unwrap().to_owned())
            .collect();
        let op = next(9);
        match op {
            0 | 1 => {
                let r = s.set_text(id, junk[next(junk.len())]);
                if r.is_err() {
                    let after: Vec<_> = ids
                        .iter()
                        .map(|i| s.pending(i).unwrap().to_owned())
                        .collect();
                    assert_eq!(before_pending, after, "a refused edit changed something");
                }
            }
            2..=4 => {
                let dir = if next(2) == 0 {
                    Step::Decrease
                } else {
                    Step::Increase
                };
                let _ = s.step(id, dir);
            }
            5 => {
                let _ = s.reset_row(id);
            }
            6 => {
                s.revert();
                for i in ids {
                    assert_eq!(s.pending(i), s.applied(i));
                }
            }
            7 => {
                now += 3.0;
                let _ = s.tick(now);
                if next(3) == 0 {
                    s.confirm();
                }
            }
            _ => {
                now += 1.0;
                let _ = s.apply(now);
            }
        }
        // Whatever happened, every value is in canonical, valid spelling ...
        for i in ids {
            let kind = &s.row(i).unwrap().kind;
            for text in [s.pending(i).unwrap(), s.applied(i).unwrap()] {
                assert_eq!(kind.canonical(text).as_deref(), Some(text), "{i}={text}");
            }
        }
        // ... and an edit never reaches the applied values.
        let after_applied: Vec<_> = ids
            .iter()
            .map(|i| s.applied(i).unwrap().to_owned())
            .collect();
        if op < 7 {
            assert_eq!(
                before_applied, after_applied,
                "op {op} reached the applied values"
            );
        }
        assert_eq!(s.has_changes(), !s.changed().is_empty());
    }
}

#[test]
fn a_fine_grid_keeps_four_decimals_and_a_walk_through_zero_never_shows_minus_zero() {
    let tab = TabSpec::new(
        "T",
        vec![RowSpec::new("fine", "F", range(0.0, 0.1, 0.0125), "0")],
    );
    let mut s = SettingsScreen::new(vec![tab], 5.0).unwrap();
    let up: Vec<_> = (0..9)
        .map(|_| s.step("fine", Step::Increase).unwrap())
        .collect();
    assert_eq!(
        up,
        [
            "0.0125", "0.025", "0.0375", "0.05", "0.0625", "0.075", "0.0875", "0.1", "0.1"
        ]
    );
    // A grid that starts below zero passes through zero; the text there is `0`, never `-0`, whatever the float error.
    let mut seen_zero = 0;
    for min in [-1.0, -0.9, -0.8, -0.7, -0.6, -0.5, -0.4, -0.3, -0.2, -0.1] {
        for step in [0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.7, 0.01, 0.03] {
            let tab = TabSpec::new(
                "T",
                vec![RowSpec::new(
                    "r",
                    "R",
                    range(min, 1.0, step),
                    &min.to_string(),
                )],
            );
            let mut s = SettingsScreen::new(vec![tab], 5.0).unwrap();
            for dir in [Step::Increase, Step::Decrease] {
                for _ in 0..120 {
                    let t = s.step("r", dir).unwrap();
                    assert_ne!(t, "-0", "min {min} step {step}");
                    if t == "0" {
                        seen_zero += 1;
                    }
                }
            }
        }
    }
    assert!(seen_zero > 20);
}

#[test]
fn a_row_that_equals_its_default_is_marked_even_when_it_differs_from_what_is_applied() {
    let mut s = screen();
    s.load("shadows", "low").unwrap(); // applied and pending are `low`, the default is `high`
    let v = s.view(0);
    assert!(!v[0].is_default && !v[0].changed);
    s.set_text("shadows", "high").unwrap(); // pending is the default, applied is not
    let v = s.view(0);
    assert!(v[0].is_default && v[0].changed);
}

#[test]
fn the_last_row_of_a_tab_counts_in_its_badge() {
    let mut s = screen();
    s.set_text("crouch", "toggle").unwrap(); // the last row of the first tab
    assert_eq!(s.changed_in_tab(0), 1);
    s.set_text("size", "extra_large").unwrap(); // the last row of the last tab
    assert_eq!(s.changed_in_tab(2), 1);
    assert_eq!(s.view(0).iter().filter(|r| r.changed).count(), 1);
}
