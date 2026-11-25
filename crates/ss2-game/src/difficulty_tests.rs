use super::*;

fn m(thousandths: u32) -> Milli {
    Milli::from_thousandths(thousandths)
}

fn raw(h: f64, s: f64, t: f64, d: f64, r: i64, i: f64, a: f64) -> RawModifiers {
    RawModifiers {
        enemy_health: h,
        enemy_speed: s,
        think_frequency: t,
        player_damage: d,
        health_recharging: r,
        initial_delay: i,
        auto_aim: a,
    }
}

fn normal_raw() -> RawModifiers {
    raw(1.0, 1.0, 1.0, 1.0, 0, 1.0, 1.0)
}

fn bad_value(level: Option<Difficulty>, field: Field, problem: ValueProblem) -> DifficultyError {
    DifficultyError::BadValue {
        level,
        field,
        problem,
    }
}

// ---- the levels ---------------------------------------------------------------------------------------------

#[test]
fn the_five_levels_are_in_order_of_difficulty() {
    use Difficulty::*;
    assert_eq!(Difficulty::ALL, [Tourist, Easy, Normal, Hard, Serious]);
    for (i, level) in Difficulty::ALL.into_iter().enumerate() {
        assert_eq!(usize::from(level.index()), i);
        assert_eq!(Difficulty::from_index(i as u8), Some(level));
    }
    assert!(Tourist < Easy && Easy < Normal && Normal < Hard && Hard < Serious);
    assert_eq!(Difficulty::from_index(5), None);
    assert_eq!(Difficulty::from_index(255), None);
    assert_eq!(Difficulty::default(), Normal);
}

#[test]
fn names_round_trip_and_are_matched_without_regard_to_case_but_not_trimmed() {
    for level in Difficulty::ALL {
        assert_eq!(Difficulty::from_name(level.name()), Some(level));
        assert_eq!(level.to_string(), level.name());
    }
    assert_eq!(Difficulty::Tourist.name(), "tourist");
    assert_eq!(Difficulty::Easy.name(), "easy");
    assert_eq!(Difficulty::Normal.name(), "normal");
    assert_eq!(Difficulty::Hard.name(), "hard");
    assert_eq!(Difficulty::Serious.name(), "serious");
    assert_eq!(Difficulty::from_name("SERIOUS"), Some(Difficulty::Serious));
    assert_eq!(Difficulty::from_name("Tourist"), Some(Difficulty::Tourist));
    assert_eq!(Difficulty::from_name(" hard"), None);
    assert_eq!(Difficulty::from_name("hard "), None);
    assert_eq!(Difficulty::from_name(""), None);
    assert_eq!(Difficulty::from_name("impossible"), None);
    // The key the save module's own test already uses.
    assert_eq!(NOTE_KEY, "difficulty");
}

#[test]
fn harder_and_easier_walk_the_ladder_and_stop_at_the_ends() {
    assert_eq!(Difficulty::Tourist.easier(), None);
    assert_eq!(Difficulty::Serious.harder(), None);
    assert_eq!(Difficulty::Tourist.harder(), Some(Difficulty::Easy));
    assert_eq!(Difficulty::Easy.harder(), Some(Difficulty::Normal));
    assert_eq!(Difficulty::Normal.harder(), Some(Difficulty::Hard));
    assert_eq!(Difficulty::Hard.harder(), Some(Difficulty::Serious));
    assert_eq!(Difficulty::Easy.easier(), Some(Difficulty::Tourist));
    assert_eq!(Difficulty::Normal.easier(), Some(Difficulty::Easy));
    assert_eq!(Difficulty::Hard.easier(), Some(Difficulty::Normal));
    assert_eq!(Difficulty::Serious.easier(), Some(Difficulty::Hard));
}

// ---- Milli: reading numbers ----------------------------------------------------------------------------------

#[test]
fn from_f64_rounds_to_the_nearest_thousandth() {
    assert_eq!(Milli::from_f64(0.0), Ok(m(0)));
    assert_eq!(Milli::from_f64(-0.0), Ok(m(0)));
    assert_eq!(Milli::from_f64(1.0), Ok(Milli::ONE));
    assert_eq!(Milli::from_f64(1.125), Ok(m(1125)));
    assert_eq!(Milli::from_f64(0.2), Ok(m(200)));
    // The f32 values of the original's file, widened.
    assert_eq!(Milli::from_f64(f64::from(0.9_f32)), Ok(m(900)));
    assert_eq!(Milli::from_f64(f64::from(0.8_f32)), Ok(m(800)));
    assert_eq!(Milli::from_f64(f64::from(1.15_f32)), Ok(m(1150)));
    assert_eq!(Milli::from_f64(f64::from(0.2_f32)), Ok(m(200)));
    // Round to nearest, either side of the half.
    assert_eq!(Milli::from_f64(1.2504), Ok(m(1250)));
    assert_eq!(Milli::from_f64(1.2506), Ok(m(1251)));
    assert_eq!(Milli::from_f64(0.0004), Ok(m(0)));
    assert_eq!(Milli::from_f64(0.0006), Ok(m(1)));
}

#[test]
fn from_f64_refuses_what_is_not_a_sane_multiplier() {
    assert_eq!(Milli::from_f64(f64::NAN), Err(ValueProblem::NotANumber));
    assert_eq!(Milli::from_f64(f64::INFINITY), Err(ValueProblem::Infinite));
    assert_eq!(
        Milli::from_f64(f64::NEG_INFINITY),
        Err(ValueProblem::Infinite)
    );
    assert_eq!(Milli::from_f64(-0.001), Err(ValueProblem::Negative));
    assert_eq!(Milli::from_f64(-1.0), Err(ValueProblem::Negative));
    assert_eq!(Milli::from_f64(f64::MIN), Err(ValueProblem::Negative));
    // 100 is the largest value; just past it is too large.
    assert_eq!(Milli::from_f64(100.0), Ok(Milli::MAX_TABLE));
    assert_eq!(Milli::from_f64(100.0004), Ok(Milli::MAX_TABLE));
    assert_eq!(Milli::from_f64(100.0006), Err(ValueProblem::TooLarge));
    assert_eq!(Milli::from_f64(1e300), Err(ValueProblem::TooLarge));
    assert_eq!(Milli::from_f64(f64::MAX), Err(ValueProblem::TooLarge));
    assert_eq!(Milli::MAX_TABLE.thousandths(), 100_000);
}

#[test]
fn milli_prints_the_shortest_exact_decimal() {
    for (t, text) in [
        (0, "0"),
        (1000, "1"),
        (1125, "1.125"),
        (200, "0.2"),
        (50, "0.05"),
        (1, "0.001"),
        (1010, "1.01"),
        (2000, "2"),
        (100_000, "100"),
        (750, "0.75"),
    ] {
        assert_eq!(m(t).to_string(), text, "{t}");
    }
}

// ---- Milli: arithmetic -----------------------------------------------------------------------------------------

#[test]
fn rounding_boundaries_for_unsigned_values() {
    let half = m(500);
    // value, down, nearest, up
    for (v, down, nearest, up) in [
        (0, 0, 0, 0),
        (1, 0, 1, 1),
        (2, 1, 1, 1),
        (3, 1, 2, 2),
        (4, 2, 2, 2),
    ] {
        assert_eq!(half.apply_u32(v, Rounding::Down), down, "down {v}");
        assert_eq!(half.apply_u32(v, Rounding::Nearest), nearest, "nearest {v}");
        assert_eq!(half.apply_u32(v, Rounding::Up), up, "up {v}");
    }
    let fifth = m(200);
    for (v, down, nearest, up) in [
        (5, 1, 1, 1),
        (6, 1, 1, 2),
        (7, 1, 1, 2),
        (8, 1, 2, 2),
        (9, 1, 2, 2),
        (10, 2, 2, 2),
        (2, 0, 0, 1),
    ] {
        assert_eq!(fifth.apply_u32(v, Rounding::Down), down, "down {v}");
        assert_eq!(
            fifth.apply_u32(v, Rounding::Nearest),
            nearest,
            "nearest {v}"
        );
        assert_eq!(fifth.apply_u32(v, Rounding::Up), up, "up {v}");
    }
    // Exactly one half below and one third-of-a-thousandth above the half.
    // A remainder of a single thousandth already counts for Up.
    assert_eq!(m(1).apply_u32(1, Rounding::Up), 1);
    assert_eq!(m(1).apply_u32(1, Rounding::Down), 0);
    assert_eq!(m(1999).apply_u32(1, Rounding::Up), 2);
    assert_eq!(m(1001).apply_u32(1, Rounding::Up), 2);
    assert_eq!(m(1499).apply_u32(1, Rounding::Nearest), 1);
    assert_eq!(m(1500).apply_u32(1, Rounding::Nearest), 2);
    assert_eq!(m(499).apply_u32(1, Rounding::Nearest), 0);
    assert_eq!(m(500).apply_u32(1, Rounding::Nearest), 1);
}

#[test]
fn one_changes_nothing_and_zero_gives_zero() {
    for v in [0, 1, 7, 1000, u32::MAX] {
        for r in [Rounding::Down, Rounding::Nearest, Rounding::Up] {
            assert_eq!(Milli::ONE.apply_u32(v, r), v);
            assert_eq!(Milli::ZERO.apply_u32(v, r), 0);
        }
    }
    for v in [i64::MIN, -5, 0, 5, i64::MAX] {
        for r in [Rounding::Down, Rounding::Nearest, Rounding::Up] {
            assert_eq!(Milli::ONE.apply_i64(v, r), v);
            assert_eq!(Milli::ZERO.apply_i64(v, r), 0);
        }
    }
    for v in [0, 5, u64::MAX] {
        assert_eq!(Milli::ONE.apply_u64(v, Rounding::Nearest), v);
        assert_eq!(Milli::ZERO.apply_u64(v, Rounding::Nearest), 0);
    }
}

#[test]
fn u32_saturates_at_the_limit() {
    assert_eq!(m(2000).apply_u32(u32::MAX, Rounding::Nearest), u32::MAX);
    assert_eq!(m(1001).apply_u32(u32::MAX, Rounding::Down), u32::MAX);
    assert_eq!(Milli::MAX_TABLE.apply_u32(u32::MAX, Rounding::Up), u32::MAX);
    assert_eq!(m(u32::MAX).apply_u32(u32::MAX, Rounding::Nearest), u32::MAX);
    // Just below the limit is still exact.
    assert_eq!(
        m(1000).apply_u32(u32::MAX - 1, Rounding::Nearest),
        u32::MAX - 1
    );
    assert_eq!(m(500).apply_u32(u32::MAX, Rounding::Down), 2_147_483_647);
    assert_eq!(m(500).apply_u32(u32::MAX, Rounding::Nearest), 2_147_483_648);
    assert_eq!(m(500).apply_u32(u32::MAX, Rounding::Up), 2_147_483_648);
    // 2^31 * 2 is exactly 2^32, one past u32::MAX.
    assert_eq!(m(2000).apply_u32(1 << 31, Rounding::Down), u32::MAX);
    assert_eq!(m(1999).apply_u32(1 << 31, Rounding::Down), 4_292_819_812);
}

#[test]
fn u64_saturates_at_the_limit() {
    assert_eq!(m(2000).apply_u64(u64::MAX, Rounding::Nearest), u64::MAX);
    assert_eq!(
        m(500).apply_u64(u64::MAX, Rounding::Down),
        9_223_372_036_854_775_807
    );
    assert_eq!(
        m(500).apply_u64(u64::MAX, Rounding::Nearest),
        9_223_372_036_854_775_808
    );
    assert_eq!(
        m(1500).apply_u64(1 << 63, Rounding::Nearest),
        13_835_058_055_282_163_712
    );
    assert_eq!(Milli::MAX_TABLE.apply_u64(u64::MAX, Rounding::Up), u64::MAX);
    assert_eq!(m(1000).apply_u64(u64::MAX, Rounding::Down), u64::MAX);
    assert_eq!(m(250).apply_u64(7, Rounding::Nearest), 2);
}

#[test]
fn i64_rounds_by_size_and_saturates_at_both_limits() {
    let half = m(500);
    for (v, down, nearest, up) in [
        (3, 1, 2, 2),
        (-3, -1, -2, -2),
        (1, 0, 1, 1),
        (-1, 0, -1, -1),
        (2, 1, 1, 1),
        (-2, -1, -1, -1),
    ] {
        assert_eq!(half.apply_i64(v, Rounding::Down), down, "down {v}");
        assert_eq!(half.apply_i64(v, Rounding::Nearest), nearest, "nearest {v}");
        assert_eq!(half.apply_i64(v, Rounding::Up), up, "up {v}");
    }
    assert_eq!(m(2000).apply_i64(i64::MAX, Rounding::Nearest), i64::MAX);
    assert_eq!(m(2000).apply_i64(i64::MIN, Rounding::Nearest), i64::MIN);
    assert_eq!(m(2000).apply_i64(i64::MIN / 2, Rounding::Nearest), i64::MIN);
    assert_eq!(
        m(2000).apply_i64(i64::MAX / 2, Rounding::Nearest),
        i64::MAX - 1
    );
    assert_eq!(m(500).apply_i64(i64::MIN, Rounding::Nearest), i64::MIN / 2);
    assert_eq!(
        m(500).apply_i64(i64::MAX, Rounding::Down),
        4_611_686_018_427_387_903
    );
    assert_eq!(
        m(500).apply_i64(i64::MAX, Rounding::Nearest),
        4_611_686_018_427_387_904
    );
    assert_eq!(
        m(500).apply_i64(-i64::MAX, Rounding::Nearest),
        -4_611_686_018_427_387_904
    );
    assert_eq!(
        Milli::MAX_TABLE.apply_i64(i64::MIN, Rounding::Down),
        i64::MIN
    );
    assert_eq!(Milli::MAX_TABLE.apply_i64(-1, Rounding::Down), -100);
    assert_eq!(
        Milli::MAX_TABLE.apply_i64(i64::MAX, Rounding::Down),
        i64::MAX
    );
}

#[test]
fn times_multiplies_two_multipliers() {
    assert_eq!(m(1500).times(m(1500)), m(2250));
    assert_eq!(m(1000).times(m(1125)), m(1125));
    assert_eq!(m(0).times(m(1125)), m(0));
    assert_eq!(m(1125).times(m(0)), m(0));
    // 0.001 * 0.5 = 0.0005: half goes up; 0.001 * 0.499 does not.
    assert_eq!(m(1).times(m(500)), m(1));
    assert_eq!(m(1).times(m(499)), m(0));
    assert_eq!(m(1001).times(m(1001)), m(1002));
    assert_eq!(m(u32::MAX).times(m(u32::MAX)), m(u32::MAX));
    assert_eq!(m(2000).times(m(u32::MAX)), m(u32::MAX));
}

#[test]
fn divide_turns_a_rate_into_a_period() {
    // x2 the rate is half the period.
    assert_eq!(m(2000).divide_u32(20, Rounding::Nearest), 10);
    assert_eq!(m(1000).divide_u32(20, Rounding::Nearest), 20);
    assert_eq!(m(500).divide_u32(20, Rounding::Nearest), 40);
    // 20 / 1.75 = 11.428...
    assert_eq!(m(1750).divide_u32(20, Rounding::Down), 11);
    assert_eq!(m(1750).divide_u32(20, Rounding::Nearest), 11);
    assert_eq!(m(1750).divide_u32(20, Rounding::Up), 12);
    // 20 / 1.125 = 17.77...
    assert_eq!(m(1125).divide_u32(20, Rounding::Nearest), 18);
    // exactly 2.5: half away from zero
    assert_eq!(m(2000).divide_u32(5, Rounding::Nearest), 3);
    assert_eq!(m(2000).divide_u32(5, Rounding::Down), 2);
    assert_eq!(m(2000).divide_u32(5, Rounding::Up), 3);
    // zero
    assert_eq!(m(0).divide_u32(0, Rounding::Nearest), 0);
    assert_eq!(m(0).divide_u32(1, Rounding::Nearest), u32::MAX);
    assert_eq!(m(0).divide_u32(u32::MAX, Rounding::Down), u32::MAX);
    assert_eq!(m(1000).divide_u32(0, Rounding::Up), 0);
    // saturation: dividing by a tiny rate
    assert_eq!(m(1).divide_u32(u32::MAX, Rounding::Nearest), u32::MAX);
    assert_eq!(m(1).divide_u32(1, Rounding::Nearest), 1000);
    assert_eq!(m(1000).divide_u32(u32::MAX, Rounding::Nearest), u32::MAX);
}

// ---- the modifiers ---------------------------------------------------------------------------------------------

#[test]
fn modifiers_new_accepts_the_limits_and_refuses_beyond() {
    let ok = Modifiers::new(m(1), m(1), m(1), m(0), false, m(0), m(0));
    assert!(ok.is_ok());
    let top = Milli::MAX_TABLE;
    assert!(Modifiers::new(top, top, top, top, true, top, top).is_ok());
    let over = m(100_001);
    let one = Milli::ONE;
    assert_eq!(
        Modifiers::new(over, one, one, one, false, one, one),
        Err(bad_value(None, Field::EnemyHealth, ValueProblem::TooLarge))
    );
    assert_eq!(
        Modifiers::new(one, over, one, one, false, one, one),
        Err(bad_value(None, Field::EnemySpeed, ValueProblem::TooLarge))
    );
    assert_eq!(
        Modifiers::new(one, one, over, one, false, one, one),
        Err(bad_value(
            None,
            Field::ThinkFrequency,
            ValueProblem::TooLarge
        ))
    );
    assert_eq!(
        Modifiers::new(one, one, one, over, false, one, one),
        Err(bad_value(None, Field::PlayerDamage, ValueProblem::TooLarge))
    );
    assert_eq!(
        Modifiers::new(one, one, one, one, false, over, one),
        Err(bad_value(None, Field::InitialDelay, ValueProblem::TooLarge))
    );
    assert_eq!(
        Modifiers::new(one, one, one, one, false, one, over),
        Err(bad_value(None, Field::AutoAim, ValueProblem::TooLarge))
    );
    let zero = Milli::ZERO;
    assert_eq!(
        Modifiers::new(zero, one, one, one, false, one, one),
        Err(bad_value(None, Field::EnemyHealth, ValueProblem::Zero))
    );
    assert_eq!(
        Modifiers::new(one, zero, one, one, false, one, one),
        Err(bad_value(None, Field::EnemySpeed, ValueProblem::Zero))
    );
    assert_eq!(
        Modifiers::new(one, one, zero, one, false, one, one),
        Err(bad_value(None, Field::ThinkFrequency, ValueProblem::Zero))
    );
    // Zero is fine for the other four.
    assert!(Modifiers::new(one, one, one, zero, false, zero, zero).is_ok());
}

#[test]
fn modifiers_new_keeps_every_value_in_its_own_place() {
    let x = Modifiers::new(m(11), m(22), m(33), m(44), true, m(55), m(66)).unwrap();
    assert_eq!(x.enemy_health(), m(11));
    assert_eq!(x.enemy_speed(), m(22));
    assert_eq!(x.think_frequency(), m(33));
    assert_eq!(x.player_damage(), m(44));
    assert!(x.health_recharging());
    assert_eq!(x.initial_delay(), m(55));
    assert_eq!(x.auto_aim(), m(66));
    let y = Modifiers::from_raw(raw(0.011, 0.022, 0.033, 0.044, 1, 0.055, 0.066)).unwrap();
    assert_eq!(x, y);
    let z = Modifiers::from_raw(raw(0.011, 0.022, 0.033, 0.044, 0, 0.055, 0.066)).unwrap();
    assert!(!z.health_recharging());
}

#[test]
fn from_raw_names_the_bad_member() {
    let n = normal_raw();
    assert!(Modifiers::from_raw(n).is_ok());
    let cases: [(RawModifiers, Field, ValueProblem); 14] = [
        (
            RawModifiers {
                enemy_health: f64::NAN,
                ..n
            },
            Field::EnemyHealth,
            ValueProblem::NotANumber,
        ),
        (
            RawModifiers {
                enemy_speed: f64::NAN,
                ..n
            },
            Field::EnemySpeed,
            ValueProblem::NotANumber,
        ),
        (
            RawModifiers {
                think_frequency: f64::INFINITY,
                ..n
            },
            Field::ThinkFrequency,
            ValueProblem::Infinite,
        ),
        (
            RawModifiers {
                player_damage: -0.5,
                ..n
            },
            Field::PlayerDamage,
            ValueProblem::Negative,
        ),
        (
            RawModifiers {
                initial_delay: f64::NAN,
                ..n
            },
            Field::InitialDelay,
            ValueProblem::NotANumber,
        ),
        (
            RawModifiers { auto_aim: 1e9, ..n },
            Field::AutoAim,
            ValueProblem::TooLarge,
        ),
        (
            RawModifiers {
                enemy_health: 0.0,
                ..n
            },
            Field::EnemyHealth,
            ValueProblem::Zero,
        ),
        (
            RawModifiers {
                enemy_speed: 0.0,
                ..n
            },
            Field::EnemySpeed,
            ValueProblem::Zero,
        ),
        (
            RawModifiers {
                think_frequency: 0.0004,
                ..n
            },
            Field::ThinkFrequency,
            ValueProblem::Zero,
        ),
        (
            RawModifiers {
                health_recharging: 2,
                ..n
            },
            Field::HealthRecharging,
            ValueProblem::NotAFlag,
        ),
        (
            RawModifiers {
                health_recharging: -1,
                ..n
            },
            Field::HealthRecharging,
            ValueProblem::NotAFlag,
        ),
        (
            RawModifiers {
                enemy_health: 100.1,
                ..n
            },
            Field::EnemyHealth,
            ValueProblem::TooLarge,
        ),
        (
            RawModifiers {
                player_damage: f64::NEG_INFINITY,
                ..n
            },
            Field::PlayerDamage,
            ValueProblem::Infinite,
        ),
        (
            RawModifiers {
                initial_delay: -0.001,
                ..n
            },
            Field::InitialDelay,
            ValueProblem::Negative,
        ),
    ];
    for (r, field, problem) in cases {
        assert_eq!(
            Modifiers::from_raw(r),
            Err(bad_value(None, field, problem)),
            "{field:?}"
        );
    }
    // Zero damage, delay and auto-aim are allowed.
    let zeros = RawModifiers {
        player_damage: 0.0,
        initial_delay: 0.0,
        auto_aim: 0.0,
        ..n
    };
    assert!(Modifiers::from_raw(zeros).is_ok());
    // The first bad member in Field order is the one reported.
    let two = RawModifiers {
        auto_aim: f64::NAN,
        enemy_speed: -1.0,
        ..n
    };
    assert_eq!(
        Modifiers::from_raw(two),
        Err(bad_value(None, Field::EnemySpeed, ValueProblem::Negative))
    );
}

#[test]
fn the_fields_know_their_member_and_provenance() {
    assert_eq!(Field::ALL.len(), 7);
    let names: Vec<_> = Field::ALL.iter().map(|f| f.member()).collect();
    assert_eq!(
        names,
        [
            "dif_fEnemyHealthMultiplier",
            "dif_fEnemySpeedMultiplier",
            "dif_fThinkFrequencyMultiplier",
            "dif_fPlayerDamageMultiplier",
            "dif_bPlayerHealthRecharging",
            "dif_fInitialDelayMultiplier",
            "dif_fAutoAimingFactor",
        ]
    );
    for f in Field::ALL {
        assert_eq!(f.provenance(), Provenance::Spec);
    }
    let inferred: Vec<_> = Field::ALL
        .into_iter()
        .filter(|f| f.role_is_inferred())
        .collect();
    assert_eq!(
        inferred,
        [
            Field::ThinkFrequency,
            Field::PlayerDamage,
            Field::InitialDelay,
            Field::AutoAim
        ]
    );
}

#[test]
fn errors_print_readable_text() {
    assert_eq!(
        bad_value(None, Field::EnemyHealth, ValueProblem::NotANumber).to_string(),
        "dif_fEnemyHealthMultiplier is not a number"
    );
    assert_eq!(
        bad_value(
            Some(Difficulty::Hard),
            Field::AutoAim,
            ValueProblem::TooLarge
        )
        .to_string(),
        "hard: dif_fAutoAimingFactor is too large"
    );
    for (problem, text) in [
        (ValueProblem::NotANumber, "not a number"),
        (ValueProblem::Infinite, "infinite"),
        (ValueProblem::Negative, "negative"),
        (ValueProblem::Zero, "zero, but it must be above zero"),
        (ValueProblem::TooLarge, "too large"),
        (ValueProblem::NotAFlag, "not 0 or 1"),
    ] {
        assert_eq!(problem.to_string(), text);
    }
    assert_eq!(
        DifficultyError::NoPlayers.to_string(),
        "a game needs at least one player"
    );
    assert_eq!(
        DifficultyError::TooManyPlayers { players: 65 }.to_string(),
        "65 players is over the limit of 64"
    );
    assert_eq!(
        DifficultyError::BadScaling.to_string(),
        "a co-op scaling step is over the limit"
    );
    assert_eq!(
        DifficultyError::ChangeRefused(ChangeRefusal::Locked).to_string(),
        "the difficulty cannot be changed: it is locked for this campaign"
    );
    assert_eq!(
        DifficultyError::ChangeRefused(ChangeRefusal::LevelInProgress).to_string(),
        "the difficulty cannot be changed: a level is being played"
    );
    assert_eq!(
        DifficultyError::BadBytes("x").to_string(),
        "cannot read the saved difficulty: x"
    );
}

// ---- the tables ------------------------------------------------------------------------------------------------

/// `(health, speed, think, damage, recharging, delay, auto-aim)` in thousandths, as the spec prints them.
type Row = (u32, u32, u32, u32, bool, u32, u32);

fn row_of(x: &Modifiers) -> Row {
    (
        x.enemy_health().thousandths(),
        x.enemy_speed().thousandths(),
        x.think_frequency().thousandths(),
        x.player_damage().thousandths(),
        x.health_recharging(),
        x.initial_delay().thousandths(),
        x.auto_aim().thousandths(),
    )
}

#[test]
fn the_pc_table_is_the_spec_table() {
    // docs/re/player-rules.md, "Difficulty tables", the PC rows.
    let want: [Row; 5] = [
        (500, 750, 750, 200, true, 1500, 1250),
        (1000, 750, 750, 500, false, 1250, 1150),
        (1000, 1000, 1000, 1000, false, 1000, 1000),
        (1000, 1125, 1125, 1500, false, 500, 900),
        (1000, 1250, 1750, 2000, false, 250, 800),
    ];
    for (level, row) in Difficulty::ALL.into_iter().zip(want) {
        assert_eq!(row_of(DifficultyTable::PC.get(level)), row, "{level}");
    }
    assert_eq!(DifficultyTable::default(), DifficultyTable::PC);
}

#[test]
fn the_xbox_table_is_the_spec_table() {
    // The same page, the parenthesis about the Xbox table.
    let want: [Row; 5] = [
        (500, 750, 750, 200, true, 1500, 1200),
        (500, 750, 750, 500, false, 1500, 1100),
        (750, 900, 750, 750, false, 1250, 1000),
        (1000, 1000, 1000, 1500, false, 1000, 900),
        (1000, 1125, 1125, 2000, false, 500, 800),
    ];
    for (level, row) in Difficulty::ALL.into_iter().zip(want) {
        assert_eq!(row_of(DifficultyTable::XBOX.get(level)), row, "{level}");
    }
    assert_ne!(DifficultyTable::XBOX, DifficultyTable::PC);
}

#[test]
fn every_shipped_entry_passes_the_validation_it_would_get_from_data() {
    for table in [DifficultyTable::PC, DifficultyTable::XBOX] {
        for level in Difficulty::ALL {
            let x = table.get(level);
            let again = Modifiers::new(
                x.enemy_health(),
                x.enemy_speed(),
                x.think_frequency(),
                x.player_damage(),
                x.health_recharging(),
                x.initial_delay(),
                x.auto_aim(),
            );
            assert_eq!(again, Ok(*x), "{level}");
        }
    }
}

#[test]
fn the_pc_table_has_the_properties_the_spec_states() {
    // Health recharges on Tourist only; Normal changes nothing; harder levels never make enemies slower.
    for level in Difficulty::ALL {
        let x = DifficultyTable::PC.get(level);
        assert_eq!(x.health_recharging(), level == Difficulty::Tourist);
    }
    let normal = DifficultyTable::PC.get(Difficulty::Normal);
    assert_eq!(normal.enemy_health(), Milli::ONE);
    assert_eq!(normal.enemy_speed(), Milli::ONE);
    assert_eq!(normal.think_frequency(), Milli::ONE);
    assert_eq!(normal.player_damage(), Milli::ONE);
    assert_eq!(normal.initial_delay(), Milli::ONE);
    assert_eq!(normal.auto_aim(), Milli::ONE);
    let mut last = DifficultyTable::PC.get(Difficulty::Tourist);
    for level in &Difficulty::ALL[1..] {
        let x = DifficultyTable::PC.get(*level);
        assert!(x.enemy_health() >= last.enemy_health());
        assert!(x.enemy_speed() >= last.enemy_speed());
        assert!(x.think_frequency() >= last.think_frequency());
        assert!(x.player_damage() >= last.player_damage());
        assert!(x.initial_delay() <= last.initial_delay());
        assert!(x.auto_aim() <= last.auto_aim());
        last = x;
    }
}

#[test]
fn a_table_from_raw_numbers_builds_or_names_the_level_and_member() {
    let rows = [
        raw(0.5, 0.75, 0.75, 0.2, 1, 1.5, 1.25),
        raw(1.0, 0.75, 0.75, 0.5, 0, 1.25, 1.15),
        normal_raw(),
        raw(1.0, 1.125, 1.125, 1.5, 0, 0.5, 0.9),
        raw(1.0, 1.25, 1.75, 2.0, 0, 0.25, 0.8),
    ];
    assert_eq!(DifficultyTable::from_raw(rows), Ok(DifficultyTable::PC));
    // The error names the entry, wherever it is.
    for (i, level) in Difficulty::ALL.into_iter().enumerate() {
        let mut broken = rows;
        broken[i].enemy_speed = f64::NAN;
        assert_eq!(
            DifficultyTable::from_raw(broken),
            Err(bad_value(
                Some(level),
                Field::EnemySpeed,
                ValueProblem::NotANumber
            )),
            "{level}"
        );
    }
    // The first bad entry wins.
    let mut two = rows;
    two[3].auto_aim = -1.0;
    two[1].health_recharging = 7;
    assert_eq!(
        DifficultyTable::from_raw(two),
        Err(bad_value(
            Some(Difficulty::Easy),
            Field::HealthRecharging,
            ValueProblem::NotAFlag
        ))
    );
}

#[test]
fn a_table_from_valid_entries_returns_them_by_level() {
    let a = *DifficultyTable::PC.get(Difficulty::Serious);
    let b = *DifficultyTable::PC.get(Difficulty::Tourist);
    let c = *DifficultyTable::PC.get(Difficulty::Normal);
    let t = DifficultyTable::new([a, b, c, a, b]);
    assert_eq!(*t.get(Difficulty::Tourist), a);
    assert_eq!(*t.get(Difficulty::Easy), b);
    assert_eq!(*t.get(Difficulty::Normal), c);
    assert_eq!(*t.get(Difficulty::Hard), a);
    assert_eq!(*t.get(Difficulty::Serious), b);
}

// ---- applying the modifiers --------------------------------------------------------------------------------------

#[test]
fn enemy_health_is_scaled_rounded_and_never_zero() {
    let t = &DifficultyTable::PC;
    let tourist = t.get(Difficulty::Tourist);
    assert_eq!(tourist.scale_enemy_health(100), 50);
    assert_eq!(tourist.scale_enemy_health(5000), 2500);
    assert_eq!(tourist.scale_enemy_health(3), 2); // 1.5 rounds half up
    assert_eq!(tourist.scale_enemy_health(2), 1);
    assert_eq!(tourist.scale_enemy_health(1), 1); // 0.5 rounds to 1 (half up)
    assert_eq!(tourist.scale_enemy_health(0), 0);
    assert_eq!(tourist.scale_enemy_health(u32::MAX), 2_147_483_648);
    // A tiny multiplier still leaves a living enemy.
    let tiny = Modifiers::new(m(1), m(1), m(1), m(1), false, m(1), m(1)).unwrap();
    assert_eq!(tiny.scale_enemy_health(100), 1);
    assert_eq!(tiny.scale_enemy_health(0), 0);
    assert_eq!(tiny.scale_enemy_health(1), 1);
    for level in [
        Difficulty::Easy,
        Difficulty::Normal,
        Difficulty::Hard,
        Difficulty::Serious,
    ] {
        assert_eq!(t.get(level).scale_enemy_health(1234), 1234);
    }
    let big = Modifiers::new(Milli::MAX_TABLE, m(1), m(1), m(1), false, m(1), m(1)).unwrap();
    assert_eq!(big.scale_enemy_health(u32::MAX), u32::MAX);
    assert_eq!(big.scale_enemy_health(3), 300);
}

#[test]
fn damage_to_the_player_follows_the_table_and_rounds_to_nearest() {
    let t = &DifficultyTable::PC;
    assert_eq!(t.get(Difficulty::Tourist).scale_damage_to_player(100), 20);
    assert_eq!(t.get(Difficulty::Tourist).scale_damage_to_player(15), 3);
    assert_eq!(t.get(Difficulty::Tourist).scale_damage_to_player(2), 0);
    assert_eq!(t.get(Difficulty::Tourist).scale_damage_to_player(3), 1); // 0.6
    assert_eq!(t.get(Difficulty::Easy).scale_damage_to_player(15), 8); // 7.5 half up
    assert_eq!(t.get(Difficulty::Normal).scale_damage_to_player(15), 15);
    assert_eq!(t.get(Difficulty::Hard).scale_damage_to_player(15), 23); // 22.5 half up
    assert_eq!(t.get(Difficulty::Serious).scale_damage_to_player(15), 30);
    assert_eq!(
        t.get(Difficulty::Serious).scale_damage_to_player(u32::MAX),
        u32::MAX
    );
    assert_eq!(t.get(Difficulty::Serious).scale_damage_to_player(0), 0);
}

#[test]
fn enemy_speed_scales_a_raw_fixed_point_number_in_both_directions() {
    let t = &DifficultyTable::PC;
    // Q16.16: 10.0 is 655_360.
    assert_eq!(t.get(Difficulty::Easy).scale_enemy_speed(655_360), 491_520);
    assert_eq!(
        t.get(Difficulty::Serious).scale_enemy_speed(655_360),
        819_200
    );
    assert_eq!(
        t.get(Difficulty::Hard).scale_enemy_speed(-655_360),
        -737_280
    );
    assert_eq!(t.get(Difficulty::Normal).scale_enemy_speed(-7), -7);
    assert_eq!(t.get(Difficulty::Hard).scale_enemy_speed(3), 3); // 3.375 rounds to 3
    assert_eq!(t.get(Difficulty::Hard).scale_enemy_speed(4), 5); // 4.5 half away from zero
    assert_eq!(t.get(Difficulty::Hard).scale_enemy_speed(-4), -5);
    assert_eq!(
        t.get(Difficulty::Serious).scale_enemy_speed(i64::MAX),
        i64::MAX
    );
    assert_eq!(
        t.get(Difficulty::Serious).scale_enemy_speed(i64::MIN),
        i64::MIN
    );
}

#[test]
fn the_think_period_is_the_inverse_of_the_think_frequency() {
    let t = &DifficultyTable::PC;
    // 20 ticks between thoughts at x1.
    assert_eq!(t.get(Difficulty::Normal).scale_think_period(20), 20);
    assert_eq!(t.get(Difficulty::Tourist).scale_think_period(20), 27); // 26.67
    assert_eq!(t.get(Difficulty::Easy).scale_think_period(20), 27);
    assert_eq!(t.get(Difficulty::Hard).scale_think_period(20), 18); // 17.78
    assert_eq!(t.get(Difficulty::Serious).scale_think_period(20), 11); // 11.43
    // Never below one tick for a real period, zero stays zero.
    assert_eq!(t.get(Difficulty::Serious).scale_think_period(1), 1); // 0.57 rounds to 1
    let fast = Modifiers::new(
        Milli::MAX_TABLE,
        m(1),
        Milli::MAX_TABLE,
        m(1),
        false,
        m(1),
        m(1),
    )
    .unwrap();
    assert_eq!(fast.scale_think_period(5), 1);
    assert_eq!(fast.scale_think_period(0), 0);
    let slow = Modifiers::new(m(1), m(1), m(1), m(1), false, m(1), m(1)).unwrap();
    assert_eq!(slow.scale_think_period(u32::MAX), u32::MAX);
}

#[test]
fn the_initial_delay_is_scaled_and_may_reach_zero() {
    let t = &DifficultyTable::PC;
    assert_eq!(t.get(Difficulty::Tourist).scale_initial_delay(20), 30);
    assert_eq!(t.get(Difficulty::Easy).scale_initial_delay(20), 25);
    assert_eq!(t.get(Difficulty::Normal).scale_initial_delay(20), 20);
    assert_eq!(t.get(Difficulty::Hard).scale_initial_delay(20), 10);
    assert_eq!(t.get(Difficulty::Serious).scale_initial_delay(20), 5);
    assert_eq!(t.get(Difficulty::Serious).scale_initial_delay(1), 0); // 0.25
    assert_eq!(t.get(Difficulty::Serious).scale_initial_delay(2), 1); // 0.5 half up
    assert_eq!(
        t.get(Difficulty::Tourist).scale_initial_delay(u32::MAX),
        u32::MAX
    );
}

#[test]
fn auto_aim_scales_a_raw_quantity() {
    let t = &DifficultyTable::PC;
    assert_eq!(t.get(Difficulty::Tourist).scale_auto_aim(1000), 1250);
    assert_eq!(t.get(Difficulty::Easy).scale_auto_aim(1000), 1150);
    assert_eq!(t.get(Difficulty::Normal).scale_auto_aim(1000), 1000);
    assert_eq!(t.get(Difficulty::Hard).scale_auto_aim(1000), 900);
    assert_eq!(t.get(Difficulty::Serious).scale_auto_aim(1000), 800);
    assert_eq!(t.get(Difficulty::Serious).scale_auto_aim(-1000), -800);
    // Rounded to nearest, not down: 0.9 and 0.8 of 1, and 0.9 of 5.
    assert_eq!(t.get(Difficulty::Hard).scale_auto_aim(1), 1);
    assert_eq!(t.get(Difficulty::Serious).scale_auto_aim(1), 1);
    assert_eq!(t.get(Difficulty::Hard).scale_auto_aim(5), 5); // 4.5 half away from zero
    assert_eq!(t.get(Difficulty::Hard).scale_auto_aim(-5), -5);
    assert_eq!(t.get(Difficulty::Serious).scale_auto_aim(2), 2); // 1.6
    assert_eq!(
        t.get(Difficulty::Tourist).scale_auto_aim(i64::MAX),
        i64::MAX
    );
    assert_eq!(
        t.get(Difficulty::Tourist).scale_auto_aim(i64::MIN),
        i64::MIN
    );
}

// ---- co-op ----------------------------------------------------------------------------------------------------------

#[test]
fn no_scaling_gives_the_same_numbers_for_any_number_of_players() {
    assert_eq!(CoopScaling::default(), CoopScaling::NONE);
    assert_eq!(CoopScaling::NONE.health_step(), Milli::ZERO);
    assert_eq!(CoopScaling::NONE.count_step(), Milli::ZERO);
    for players in [1, 2, 4, MAX_PLAYERS] {
        assert_eq!(CoopScaling::NONE.health_factor(players), Ok(Milli::ONE));
        assert_eq!(CoopScaling::NONE.count_factor(players), Ok(Milli::ONE));
        for level in Difficulty::ALL {
            let e = Effective::new(DifficultyTable::PC.get(level), &CoopScaling::NONE, players)
                .unwrap();
            assert_eq!(
                e.enemy_health(1234),
                DifficultyTable::PC.get(level).scale_enemy_health(1234)
            );
            assert_eq!(e.spawn_count(7), 7);
        }
    }
}

#[test]
fn the_factor_is_linear_in_the_extra_players() {
    let c = CoopScaling::new(m(500), m(250)).unwrap();
    assert_eq!(c.health_step(), m(500));
    assert_eq!(c.count_step(), m(250));
    // One player is exactly x1 whatever the step.
    assert_eq!(c.health_factor(1), Ok(m(1000)));
    assert_eq!(c.count_factor(1), Ok(m(1000)));
    assert_eq!(c.health_factor(2), Ok(m(1500)));
    assert_eq!(c.health_factor(3), Ok(m(2000)));
    assert_eq!(c.health_factor(4), Ok(m(2500)));
    assert_eq!(c.count_factor(2), Ok(m(1250)));
    assert_eq!(c.count_factor(5), Ok(m(2000)));
    assert_eq!(c.health_factor(MAX_PLAYERS), Ok(m(1000 + 500 * 63)));
    // The extreme: the largest step with the most players still fits.
    let top = CoopScaling::new(Milli::MAX_TABLE, Milli::MAX_TABLE).unwrap();
    assert_eq!(top.health_factor(MAX_PLAYERS), Ok(m(1000 + 100_000 * 63)));
    assert_eq!(top.count_factor(MAX_PLAYERS), Ok(m(1000 + 100_000 * 63)));
}

#[test]
fn zero_and_too_many_players_are_refused() {
    let c = CoopScaling::new(m(500), m(500)).unwrap();
    assert_eq!(c.health_factor(0), Err(DifficultyError::NoPlayers));
    assert_eq!(c.count_factor(0), Err(DifficultyError::NoPlayers));
    assert_eq!(
        c.health_factor(MAX_PLAYERS + 1),
        Err(DifficultyError::TooManyPlayers {
            players: MAX_PLAYERS + 1
        })
    );
    assert_eq!(
        c.count_factor(255),
        Err(DifficultyError::TooManyPlayers { players: 255 })
    );
    assert!(c.health_factor(MAX_PLAYERS).is_ok());
    let n = DifficultyTable::PC.get(Difficulty::Normal);
    assert_eq!(Effective::new(n, &c, 0), Err(DifficultyError::NoPlayers));
    assert_eq!(
        Effective::new(n, &c, 65),
        Err(DifficultyError::TooManyPlayers { players: 65 })
    );
    // The no-scaling rule still refuses them: the player count is part of the contract.
    assert_eq!(
        CoopScaling::NONE.health_factor(0),
        Err(DifficultyError::NoPlayers)
    );
    assert_eq!(
        Effective::new(n, &CoopScaling::NONE, 200),
        Err(DifficultyError::TooManyPlayers { players: 200 })
    );
}

#[test]
fn a_scaling_step_over_the_limit_is_refused() {
    assert!(CoopScaling::new(Milli::MAX_TABLE, Milli::MAX_TABLE).is_ok());
    assert_eq!(
        CoopScaling::new(m(100_001), m(0)),
        Err(DifficultyError::BadScaling)
    );
    assert_eq!(
        CoopScaling::new(m(0), m(100_001)),
        Err(DifficultyError::BadScaling)
    );
    assert!(CoopScaling::new(m(0), m(0)).is_ok());
}

#[test]
fn effective_health_rounds_once_and_combines_both_factors() {
    // Serious x1, with +50% per extra player.
    let c = CoopScaling::new(m(500), m(1000)).unwrap();
    let serious = DifficultyTable::PC.get(Difficulty::Serious);
    let e1 = Effective::new(serious, &c, 1).unwrap();
    let e2 = Effective::new(serious, &c, 2).unwrap();
    let e4 = Effective::new(serious, &c, 4).unwrap();
    assert_eq!(e1.enemy_health(100), 100);
    assert_eq!(e2.enemy_health(100), 150);
    assert_eq!(e4.enemy_health(100), 250);
    assert_eq!(e2.players(), 2);
    assert_eq!(e1.players(), 1);
    // Tourist x0.5 with x1.5: 0.75 in one rounding: 3 * 0.75 = 2.25 -> 2, 2 * 0.75 = 1.5 -> 2.
    let tourist = DifficultyTable::PC.get(Difficulty::Tourist);
    let t2 = Effective::new(tourist, &c, 2).unwrap();
    assert_eq!(t2.enemy_health(3), 2);
    assert_eq!(t2.enemy_health(2), 2);
    assert_eq!(t2.enemy_health(1), 1); // 0.75 rounds to 1
    assert_eq!(t2.enemy_health(0), 0);
    // Rounding twice (0.5 then 1.5) would give a different answer for 3: round(1.5)=2, round(3.0)=3.
    assert_eq!(t2.enemy_health(3), 2);
    // Never zero for a positive base.
    let tiny = Modifiers::new(m(1), m(1), m(1), m(1), false, m(1), m(1)).unwrap();
    assert_eq!(
        Effective::new(&tiny, &CoopScaling::NONE, 1)
            .unwrap()
            .enemy_health(10),
        1
    );
    // Saturation.
    let top = CoopScaling::new(Milli::MAX_TABLE, Milli::MAX_TABLE).unwrap();
    let big = Modifiers::new(Milli::MAX_TABLE, m(1), m(1), m(1), false, m(1), m(1)).unwrap();
    let e = Effective::new(&big, &top, MAX_PLAYERS).unwrap();
    assert_eq!(e.enemy_health(u32::MAX), u32::MAX);
    assert_eq!(e.enemy_health(1), 630_100);
}

#[test]
fn the_modifiers_do_not_change_with_the_player_count() {
    let c = CoopScaling::new(m(900), m(900)).unwrap();
    for level in Difficulty::ALL {
        let base = DifficultyTable::PC.get(level);
        for players in [1, 2, 8, MAX_PLAYERS] {
            let e = Effective::new(base, &c, players).unwrap();
            assert_eq!(e.modifiers(), base);
        }
    }
}

#[test]
fn spawn_counts_scale_with_players_and_never_vanish() {
    let c = CoopScaling::new(m(0), m(500)).unwrap();
    let n = DifficultyTable::PC.get(Difficulty::Normal);
    let one = Effective::new(n, &c, 1).unwrap();
    let two = Effective::new(n, &c, 2).unwrap();
    let three = Effective::new(n, &c, 3).unwrap();
    assert_eq!(one.spawn_count(4), 4);
    assert_eq!(two.spawn_count(4), 6);
    assert_eq!(three.spawn_count(4), 8);
    assert_eq!(two.spawn_count(1), 2); // 1.5 half up
    assert_eq!(two.spawn_count(0), 0);
    assert_eq!(two.spawn_count(3), 5); // 4.5 half up
    // The difficulty does not change the count.
    for level in Difficulty::ALL {
        let e = Effective::new(DifficultyTable::PC.get(level), &c, 2).unwrap();
        assert_eq!(e.spawn_count(4), 6);
    }
    let top = CoopScaling::new(m(0), Milli::MAX_TABLE).unwrap();
    assert_eq!(
        Effective::new(n, &top, MAX_PLAYERS)
            .unwrap()
            .spawn_count(u32::MAX),
        u32::MAX
    );
}

#[test]
fn the_effective_numbers_are_the_same_every_time() {
    let c = CoopScaling::new(m(333), m(777)).unwrap();
    let serious = DifficultyTable::PC.get(Difficulty::Hard);
    let a: Vec<u32> = (0..2000)
        .map(|b| {
            Effective::new(serious, &c, 1 + (b % 64) as u8)
                .unwrap()
                .enemy_health(b)
        })
        .collect();
    let b: Vec<u32> = (0..2000)
        .map(|b| {
            Effective::new(serious, &c, 1 + (b % 64) as u8)
                .unwrap()
                .enemy_health(b)
        })
        .collect();
    assert_eq!(a, b);
    // Spot checks of the exact integer arithmetic: 1.0 * (1 + 0.333 * 3) = 1.999
    let e4 = Effective::new(DifficultyTable::PC.get(Difficulty::Normal), &c, 4).unwrap();
    assert_eq!(e4.enemy_health(1000), 1999);
    assert_eq!(e4.spawn_count(1000), 3331);
}

// ---- carry-over and locking ----------------------------------------------------------------------------------

#[test]
fn the_default_policy_changes_only_between_levels() {
    assert_eq!(ChangePolicy::default(), ChangePolicy::BetweenLevels);
    let mut c = CampaignDifficulty::new(Difficulty::Hard, ChangePolicy::default());
    assert_eq!(c.level(), Difficulty::Hard);
    assert_eq!(c.policy(), ChangePolicy::BetweenLevels);
    assert!(!c.in_level());
    assert_eq!(c.can_change(), Ok(()));
    assert_eq!(c.request_change(Difficulty::Easy), Ok(true));
    assert_eq!(c.level(), Difficulty::Easy);
    c.enter_level();
    assert!(c.in_level());
    assert_eq!(c.can_change(), Err(ChangeRefusal::LevelInProgress));
    assert_eq!(
        c.request_change(Difficulty::Serious),
        Err(DifficultyError::ChangeRefused(
            ChangeRefusal::LevelInProgress
        ))
    );
    assert_eq!(
        c.level(),
        Difficulty::Easy,
        "a refused change changes nothing"
    );
    // Asking for the level already in force is always fine.
    assert_eq!(c.request_change(Difficulty::Easy), Ok(false));
    // Entering twice is the same as once; leaving carries the level to the next one.
    c.enter_level();
    c.leave_level();
    assert!(!c.in_level());
    assert_eq!(c.level(), Difficulty::Easy);
    assert_eq!(c.request_change(Difficulty::Serious), Ok(true));
    assert_eq!(c.level(), Difficulty::Serious);
}

#[test]
fn a_locked_campaign_keeps_its_level_at_every_moment() {
    let mut c = CampaignDifficulty::new(Difficulty::Serious, ChangePolicy::Locked);
    for in_level in [false, true] {
        if in_level {
            c.enter_level();
        }
        assert_eq!(c.can_change(), Err(ChangeRefusal::Locked));
        for other in Difficulty::ALL {
            let r = c.request_change(other);
            if other == Difficulty::Serious {
                assert_eq!(r, Ok(false));
            } else {
                assert_eq!(
                    r,
                    Err(DifficultyError::ChangeRefused(ChangeRefusal::Locked))
                );
            }
            assert_eq!(c.level(), Difficulty::Serious);
        }
    }
}

#[test]
fn an_anytime_campaign_changes_in_the_middle_of_a_level() {
    let mut c = CampaignDifficulty::new(Difficulty::Normal, ChangePolicy::Anytime);
    c.enter_level();
    assert_eq!(c.can_change(), Ok(()));
    assert_eq!(c.request_change(Difficulty::Tourist), Ok(true));
    assert_eq!(c.request_change(Difficulty::Serious), Ok(true));
    assert_eq!(c.level(), Difficulty::Serious);
    assert!(c.in_level());
}

#[test]
fn the_note_names_the_level_for_the_save() {
    let c = CampaignDifficulty::new(Difficulty::Hard, ChangePolicy::Locked);
    assert_eq!(c.note(), ("difficulty", "hard"));
    for level in Difficulty::ALL {
        let c = CampaignDifficulty::new(level, ChangePolicy::Anytime);
        let (key, value) = c.note();
        assert_eq!(key, NOTE_KEY);
        let back = CampaignDifficulty::from_note(value, ChangePolicy::Anytime).unwrap();
        assert_eq!(back, c);
    }
    // A loaded campaign is between levels, with the policy the caller gives.
    let loaded = CampaignDifficulty::from_note("SERIOUS", ChangePolicy::Locked).unwrap();
    assert_eq!(loaded.level(), Difficulty::Serious);
    assert_eq!(loaded.policy(), ChangePolicy::Locked);
    assert!(!loaded.in_level());
    assert_eq!(
        CampaignDifficulty::from_note("nightmare", ChangePolicy::Locked),
        Err(DifficultyError::BadBytes("unknown difficulty name"))
    );
}

#[test]
fn the_state_round_trips_through_four_bytes() {
    for level in Difficulty::ALL {
        for policy in [
            ChangePolicy::Locked,
            ChangePolicy::BetweenLevels,
            ChangePolicy::Anytime,
        ] {
            for in_level in [false, true] {
                let mut c = CampaignDifficulty::new(level, policy);
                if in_level {
                    c.enter_level();
                }
                let bytes = c.to_bytes();
                assert_eq!(bytes.len(), CAMPAIGN_BYTES);
                assert_eq!(CampaignDifficulty::from_bytes(&bytes), Ok(c));
            }
        }
    }
    // The layout is fixed: version, level, policy, flag.
    let mut c = CampaignDifficulty::new(Difficulty::Hard, ChangePolicy::Anytime);
    c.enter_level();
    assert_eq!(c.to_bytes(), [1, 3, 2, 1]);
    assert_eq!(
        CampaignDifficulty::new(Difficulty::Tourist, ChangePolicy::Locked).to_bytes(),
        [1, 0, 0, 0]
    );
}

#[test]
fn damaged_state_bytes_are_refused() {
    let bad = |b: &[u8], why: &'static str| {
        assert_eq!(
            CampaignDifficulty::from_bytes(b),
            Err(DifficultyError::BadBytes(why)),
            "{b:?}"
        );
    };
    bad(&[], "wrong length");
    bad(&[1, 2, 1], "wrong length");
    bad(&[1, 2, 1, 0, 0], "wrong length");
    bad(&[0, 2, 1, 0], "unknown version");
    bad(&[2, 2, 1, 0], "unknown version");
    bad(&[1, 5, 1, 0], "unknown difficulty level");
    bad(&[1, 255, 1, 0], "unknown difficulty level");
    bad(&[1, 2, 3, 0], "unknown change policy");
    bad(&[1, 2, 1, 2], "bad in-level flag");
    bad(&[1, 2, 1, 255], "bad in-level flag");
    assert!(CampaignDifficulty::from_bytes(&[1, 4, 2, 1]).is_ok());
}

// ---- the real install ------------------------------------------------------------------------------------------

/// A `CGameDifficulties` object from the file as five rows of raw numbers, in the order of the original's class.
fn table_from_object(object: &ss2_format_rsc::ResourceDatabase, member: &str) -> [RawModifiers; 5] {
    let tables = object
        .root
        .find(member)
        .and_then(|v| v.as_object())
        .unwrap();
    assert_eq!(tables.class, "CGameDifficulties");
    let names = [
        "gd_dpTourist",
        "gd_dpEasy",
        "gd_dpNormal",
        "gd_dpHard",
        "gd_dpSerious",
    ];
    names.map(|name| {
        let row = tables.find(name).and_then(|v| v.as_object()).unwrap();
        assert_eq!(row.class, "CDifficultyParams");
        let f = |field: Field| row.find(field.member()).and_then(|v| v.as_f64()).unwrap();
        let i = |field: Field| row.find(field.member()).and_then(|v| v.as_i64()).unwrap();
        RawModifiers {
            enemy_health: f(Field::EnemyHealth),
            enemy_speed: f(Field::EnemySpeed),
            think_frequency: f(Field::ThinkFrequency),
            player_damage: f(Field::PlayerDamage),
            health_recharging: i(Field::HealthRecharging),
            initial_delay: f(Field::InitialDelay),
            auto_aim: f(Field::AutoAim),
        }
    })
}

#[test]
fn the_constants_match_the_difficulty_tables_of_the_real_install() {
    use ss2_vfs::Vfs;
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let path =
        ss2_vfs::VfsPath::new("Content/SeriousSam2/Databases/GlobalGameParameters.rsc").unwrap();
    let db = ss2_format_rsc::ResourceDatabase::from_bytes(&vfs.read(&path).unwrap()).unwrap();
    assert_eq!(db.kind, ss2_format_rsc::RscKind::GlobalGameParams);

    let pc = DifficultyTable::from_raw(table_from_object(&db, "ggp_gdGameDifficulties")).unwrap();
    assert_eq!(pc, DifficultyTable::PC);
    let xbox =
        DifficultyTable::from_raw(table_from_object(&db, "ggp_gdGameDifficultiesXBox")).unwrap();
    assert_eq!(xbox, DifficultyTable::XBOX);

    // Print what was compared, so a run with -- --nocapture shows the real numbers.
    for level in Difficulty::ALL {
        let x = pc.get(level);
        println!(
            "{level:8} health {} speed {} think {} damage {} recharge {} delay {} aim {}",
            x.enemy_health(),
            x.enemy_speed(),
            x.think_frequency(),
            x.player_damage(),
            x.health_recharging(),
            x.initial_delay(),
            x.auto_aim()
        );
    }
}
