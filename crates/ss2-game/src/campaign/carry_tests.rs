use super::*;

fn map(pairs: &[(&str, u32)]) -> BTreeMap<String, u32> {
    pairs.iter().map(|(k, v)| ((*k).to_string(), *v)).collect()
}

fn pc(health: u32, armor: u32) -> PlayerCarry {
    PlayerCarry {
        health,
        armor,
        ..PlayerCarry::default()
    }
}

fn rich() -> PlayerCarry {
    PlayerCarry {
        health: 73,
        armor: 12,
        preferred_weapon: Some("shotgun".into()),
        weapons: map(&[("colt", 2), ("shotgun", 1)]),
        ammo: map(&[("bullets", 120), ("shells", 14)]),
        items: map(&[("key_red", 1)]),
    }
}

fn start() -> PlayerCarry {
    PlayerCarry {
        health: 100,
        armor: 0,
        preferred_weapon: Some("colt".into()),
        weapons: map(&[("colt", 1), ("knife", 1)]),
        ammo: map(&[("bullets", 50)]),
        items: BTreeMap::new(),
    }
}

fn team() -> CarryOver {
    let mut c = CarryOver::new();
    c.set(0, Slot::Carried(rich())).unwrap();
    c.set(2, Slot::Dead).unwrap();
    c.set(5, Slot::Carried(pc(1, 0))).unwrap();
    c
}

fn reseal(bytes: &mut [u8]) {
    let n = bytes.len() - 4;
    let crc = crate::level_progress::crc32(&bytes[..n]);
    bytes[n..].copy_from_slice(&crc.to_le_bytes());
}

// ---- the merge rule ------------------------------------------------------------------------------------------

#[test]
fn carried_health_and_armor_are_raised_to_the_start_never_lowered() {
    let m = |h, a| pc(h, a).merged_with_floor(&pc(100, 10));
    assert_eq!((m(5, 0).health, m(5, 0).armor), (100, 10));
    assert_eq!((m(100, 10).health, m(100, 10).armor), (100, 10));
    assert_eq!((m(150, 40).health, m(150, 40).armor), (150, 40));
    assert_eq!(m(101, 11).health, 101);
    assert_eq!(m(99, 9).armor, 10);
}

#[test]
fn each_key_takes_the_larger_count_and_keys_from_either_side_are_kept() {
    let merged = rich().merged_with_floor(&start());
    assert_eq!(
        merged.weapons,
        map(&[("colt", 2), ("knife", 1), ("shotgun", 1)])
    );
    assert_eq!(merged.ammo, map(&[("bullets", 120), ("shells", 14)]));
    assert_eq!(merged.items, map(&[("key_red", 1)]));
    // A key that only the start has is added, a smaller carried count is raised.
    let low = PlayerCarry {
        ammo: map(&[("bullets", 10)]),
        ..PlayerCarry::default()
    };
    assert_eq!(
        low.merged_with_floor(&start()).ammo,
        map(&[("bullets", 50)])
    );
}

#[test]
fn the_preferred_weapon_stays_the_players_else_comes_from_the_start() {
    assert_eq!(
        rich()
            .merged_with_floor(&start())
            .preferred_weapon
            .as_deref(),
        Some("shotgun")
    );
    let none = PlayerCarry::default();
    assert_eq!(
        none.merged_with_floor(&start()).preferred_weapon.as_deref(),
        Some("colt")
    );
    assert_eq!(none.merged_with_floor(&none).preferred_weapon, None);
    // The result is always valid when both sides are.
    rich().merged_with_floor(&start()).validate().unwrap();
    none.merged_with_floor(&start()).validate().unwrap();
}

#[test]
fn merging_with_nothing_changes_nothing() {
    assert_eq!(rich().merged_with_floor(&PlayerCarry::default()), rich());
}

// ---- what a slot starts with ---------------------------------------------------------------------------------

#[test]
fn keep_gives_a_living_player_the_merge_and_everyone_else_the_start() {
    let c = team();
    assert_eq!(
        c.resolve(0, &start(), CarryPolicy::Keep),
        rich().merged_with_floor(&start())
    );
    // Dead: the chapter's start inventory.
    assert_eq!(c.resolve(2, &start(), CarryPolicy::Keep), start());
    // A late joiner (no entry): the chapter's start inventory.
    assert_eq!(c.resolve(1, &start(), CarryPolicy::Keep), start());
    assert_eq!(c.resolve(63, &start(), CarryPolicy::Keep), start());
    // The weak player is raised to the start.
    let weak = c.resolve(5, &start(), CarryPolicy::Keep);
    assert_eq!(weak.health, 100);
    assert_eq!(weak.weapons, start().weapons);
}

#[test]
fn reset_gives_everyone_the_start() {
    let c = team();
    for slot in [0, 1, 2, 5] {
        assert_eq!(c.resolve(slot, &start(), CarryPolicy::Reset), start());
    }
    assert_eq!(CarryPolicy::default(), CarryPolicy::Keep);
}

// ---- building and validating ---------------------------------------------------------------------------------

#[test]
fn slots_are_below_sixty_four() {
    let mut c = CarryOver::new();
    assert!(c.set(63, Slot::Dead).is_ok());
    assert_eq!(
        c.set(64, Slot::Dead).unwrap_err(),
        CarryError::BadSlot { slot: 64 }
    );
    assert_eq!(
        c.set(255, Slot::Dead).unwrap_err(),
        CarryError::BadSlot { slot: 255 }
    );
    assert_eq!(c.len(), 1);
}

#[test]
fn set_returns_the_old_value_remove_forgets_and_get_looks() {
    let mut c = CarryOver::new();
    assert!(c.is_empty());
    assert_eq!(c.set(1, Slot::Dead).unwrap(), None);
    assert_eq!(c.set(1, Slot::Carried(pc(5, 5))).unwrap(), Some(Slot::Dead));
    assert_eq!(c.get(1), Some(&Slot::Carried(pc(5, 5))));
    assert_eq!(c.get(2), None);
    assert_eq!(c.remove(1), Some(Slot::Carried(pc(5, 5))));
    assert_eq!(c.remove(1), None);
    assert!(c.is_empty());
    let t = team();
    let slots: Vec<u8> = t.iter().map(|(s, _)| s).collect();
    assert_eq!(slots, [0, 2, 5]);
}

#[test]
fn a_refused_set_changes_nothing() {
    let mut c = team();
    let before = c.clone();
    let mut bad = rich();
    bad.preferred_weapon = Some("bazooka".into());
    assert_eq!(
        c.set(0, Slot::Carried(bad)).unwrap_err(),
        CarryError::PreferredNotOwned
    );
    assert_eq!(c, before);
}

#[test]
fn keys_have_a_length_and_no_control_characters() {
    let with_ammo = |k: &str| PlayerCarry {
        ammo: map(&[(k, 1)]),
        ..PlayerCarry::default()
    };
    assert!(with_ammo("a").validate().is_ok());
    assert!(with_ammo(&"k".repeat(MAX_KEY_LEN)).validate().is_ok());
    let bad = CarryError::BadKey { map: "ammo" };
    assert_eq!(with_ammo("").validate().unwrap_err(), bad);
    assert_eq!(
        with_ammo(&"k".repeat(MAX_KEY_LEN + 1))
            .validate()
            .unwrap_err(),
        bad
    );
    assert_eq!(with_ammo("a\nb").validate().unwrap_err(), bad);
    // Each map reports its own name.
    let weapons = PlayerCarry {
        weapons: map(&[("", 1)]),
        ..PlayerCarry::default()
    };
    assert_eq!(
        weapons.validate().unwrap_err(),
        CarryError::BadKey { map: "weapons" }
    );
    let items = PlayerCarry {
        items: map(&[("", 1)]),
        ..PlayerCarry::default()
    };
    assert_eq!(
        items.validate().unwrap_err(),
        CarryError::BadKey { map: "items" }
    );
}

#[test]
fn a_map_has_at_most_256_entries_and_no_zero_counts() {
    let many = |n: usize, zero: bool| -> BTreeMap<String, u32> {
        (0..n)
            .map(|i| (format!("k{i:03}"), u32::from(!zero)))
            .collect()
    };
    let mk = |m| PlayerCarry {
        items: m,
        ..PlayerCarry::default()
    };
    assert!(mk(many(MAX_ENTRIES, false)).validate().is_ok());
    assert_eq!(
        mk(many(MAX_ENTRIES + 1, false)).validate().unwrap_err(),
        CarryError::TooManyEntries { map: "items" }
    );
    assert_eq!(
        mk(many(1, true)).validate().unwrap_err(),
        CarryError::ZeroCount { map: "items" }
    );
    let w = PlayerCarry {
        weapons: map(&[("colt", 0)]),
        ..PlayerCarry::default()
    };
    assert_eq!(
        w.validate().unwrap_err(),
        CarryError::ZeroCount { map: "weapons" }
    );
    let a = PlayerCarry {
        ammo: many(MAX_ENTRIES + 1, false),
        ..PlayerCarry::default()
    };
    assert_eq!(
        a.validate().unwrap_err(),
        CarryError::TooManyEntries { map: "ammo" }
    );
    let wt = PlayerCarry {
        weapons: many(MAX_ENTRIES + 1, false),
        ..PlayerCarry::default()
    };
    assert_eq!(
        wt.validate().unwrap_err(),
        CarryError::TooManyEntries { map: "weapons" }
    );
}

#[test]
fn the_preferred_weapon_must_be_a_valid_key_that_the_player_owns() {
    let mut p = rich();
    p.preferred_weapon = Some(String::new());
    assert_eq!(
        p.validate().unwrap_err(),
        CarryError::BadKey {
            map: "preferred weapon"
        }
    );
    p.preferred_weapon = Some("ghost".into());
    assert_eq!(p.validate().unwrap_err(), CarryError::PreferredNotOwned);
    p.preferred_weapon = Some("colt".into());
    assert!(p.validate().is_ok());
    p.preferred_weapon = None;
    assert!(p.validate().is_ok());
}

#[test]
fn errors_say_what_is_wrong() {
    let all = [
        (CarryError::BadSlot { slot: 70 }, "70"),
        (CarryError::BadKey { map: "ammo" }, "ammo"),
        (CarryError::TooManyEntries { map: "items" }, "items"),
        (CarryError::ZeroCount { map: "weapons" }, "weapons"),
        (CarryError::PreferredNotOwned, "preferred"),
    ];
    for (e, needle) in all {
        assert!(e.to_string().contains(needle), "{e}");
    }
}

// ---- bytes ---------------------------------------------------------------------------------------------------

#[test]
fn a_team_round_trips_and_the_same_team_gives_the_same_bytes() {
    let c = team();
    let b = c.to_bytes();
    assert_eq!(CarryOver::from_bytes(&b).unwrap(), c);
    assert_eq!(team().to_bytes(), b);
    // Building it in another order gives the same bytes.
    let mut d = CarryOver::new();
    d.set(5, Slot::Carried(pc(1, 0))).unwrap();
    d.set(2, Slot::Dead).unwrap();
    d.set(0, Slot::Carried(rich())).unwrap();
    assert_eq!(d.to_bytes(), b);
    assert_eq!(&b[..4], b"S2CO");
    assert_eq!(u16::from_le_bytes([b[4], b[5]]), 1);
}

#[test]
fn an_empty_team_and_a_full_one_round_trip() {
    let e = CarryOver::new();
    assert_eq!(CarryOver::from_bytes(&e.to_bytes()).unwrap(), e);
    let mut full = CarryOver::new();
    for s in 0..MAX_PLAYERS {
        let mut p = rich();
        p.health = u32::from(s);
        full.set(s, Slot::Carried(p)).unwrap();
    }
    assert_eq!(CarryOver::from_bytes(&full.to_bytes()).unwrap(), full);
    // The largest counts survive.
    let mut big = CarryOver::new();
    big.set(
        0,
        Slot::Carried(PlayerCarry {
            health: u32::MAX,
            armor: u32::MAX,
            ammo: map(&[("x", u32::MAX)]),
            ..PlayerCarry::default()
        }),
    )
    .unwrap();
    assert_eq!(CarryOver::from_bytes(&big.to_bytes()).unwrap(), big);
}

#[test]
fn every_prefix_of_the_bytes_is_refused() {
    let b = team().to_bytes();
    for n in 0..b.len() {
        assert!(
            CarryOver::from_bytes(&b[..n]).is_err(),
            "prefix of {n} bytes was accepted"
        );
    }
}

#[test]
fn every_flipped_byte_is_refused() {
    let b = team().to_bytes();
    for i in 0..b.len() {
        for bit in [0x01u8, 0x80] {
            let mut d = b.clone();
            d[i] ^= bit;
            assert!(
                CarryOver::from_bytes(&d).is_err(),
                "byte {i} ^ {bit:#x} was accepted"
            );
        }
    }
}

#[test]
fn damage_that_keeps_the_checksum_valid_is_still_refused_with_the_right_error() {
    let good = team().to_bytes();
    let fix = |mutate: &dyn Fn(&mut Vec<u8>)| {
        let mut d = good.clone();
        mutate(&mut d);
        reseal(&mut d);
        CarryOver::from_bytes(&d)
    };
    assert_eq!(fix(&|d| d[0] = b'X').unwrap_err(), BytesError::WrongMagic);
    assert_eq!(
        fix(&|d| d[4] = 2).unwrap_err(),
        BytesError::UnknownVersion { found: 2 }
    );
    // Layout after the 6 header bytes: count u32, then slot 0: slot u8, tag u8, health u32 ...
    assert_eq!(
        fix(&|d| d[6..10].copy_from_slice(&65u32.to_le_bytes())).unwrap_err(),
        BytesError::Invalid("a count is over its limit")
    );
    // A count the bytes cannot hold (one dead slot is two bytes).
    let mut one = CarryOver::new();
    one.set(0, Slot::Dead).unwrap();
    let mut d = one.to_bytes();
    d[6..10].copy_from_slice(&30u32.to_le_bytes());
    reseal(&mut d);
    assert_eq!(
        CarryOver::from_bytes(&d).unwrap_err(),
        BytesError::Truncated
    );
    assert_eq!(
        fix(&|d| d[10] = 64).unwrap_err(),
        BytesError::Invalid("slots out of order or out of range")
    );
    assert_eq!(
        fix(&|d| d[11] = 9).unwrap_err(),
        BytesError::Invalid("unknown slot tag")
    );
    // The preferred-weapon flag is a bool: byte 10 slot, 11 tag, 12..16 health, 16..20 armor, 20 flag.
    assert_eq!(
        fix(&|d| d[20] = 2).unwrap_err(),
        BytesError::Invalid("a flag is not 0 or 1")
    );
    // Bytes left after the value: insert one before the checksum.
    assert_eq!(
        fix(&|d| {
            let n = d.len() - 4;
            d.insert(n, 0);
        })
        .unwrap_err(),
        BytesError::Trailing
    );
}

#[test]
fn slots_out_of_order_or_repeated_are_refused() {
    let mut a = CarryOver::new();
    a.set(1, Slot::Dead).unwrap();
    a.set(2, Slot::Dead).unwrap();
    let mut b = a.to_bytes();
    // Layout: 6 header, count u32 (6..10), slot 1 at 10, tag 11, slot 2 at 12, tag 13.
    b[10] = 2; // 2, 2: repeated
    reseal(&mut b);
    assert_eq!(
        CarryOver::from_bytes(&b).unwrap_err(),
        BytesError::Invalid("slots out of order or out of range")
    );
    let mut c = a.to_bytes();
    c[10] = 3; // 3, 2: descending
    reseal(&mut c);
    assert!(CarryOver::from_bytes(&c).is_err());
}

#[test]
fn a_state_the_validator_would_refuse_is_refused_on_reading() {
    // Write a player with a preferred weapon that is not owned, by hand.
    let mut w = Writer::new(MAGIC, VERSION);
    w.u32(1);
    w.u8(0);
    w.u8(1);
    w.u32(1);
    w.u32(1);
    w.u8(1);
    w.str("ghost");
    for _ in 0..3 {
        w.u32(0);
    }
    assert_eq!(
        CarryOver::from_bytes(&w.finish()).unwrap_err(),
        BytesError::Invalid("a player's state is out of range")
    );
    // A zero count.
    let mut w = Writer::new(MAGIC, VERSION);
    w.u32(1);
    w.u8(0);
    w.u8(1);
    w.u32(1);
    w.u32(1);
    w.u8(0);
    w.u32(0); // weapons
    w.u32(1); // ammo: one entry
    w.str("bullets");
    w.u32(0);
    w.u32(0); // items
    assert_eq!(
        CarryOver::from_bytes(&w.finish()).unwrap_err(),
        BytesError::Invalid("a player's state is out of range")
    );
}

#[test]
fn map_keys_out_of_order_or_repeated_are_refused() {
    let build = |first: &str, second: &str| {
        let mut w = Writer::new(MAGIC, VERSION);
        w.u32(1);
        w.u8(0);
        w.u8(1);
        w.u32(1);
        w.u32(1);
        w.u8(0);
        w.u32(2); // weapons
        w.str(first);
        w.u32(1);
        w.str(second);
        w.u32(1);
        w.u32(0);
        w.u32(0);
        CarryOver::from_bytes(&w.finish())
    };
    assert!(build("a", "b").is_ok());
    let order = BytesError::Invalid("keys out of order or repeated");
    assert_eq!(build("b", "a").unwrap_err(), order);
    assert_eq!(build("a", "a").unwrap_err(), order);
}

#[test]
fn a_hostile_count_does_not_allocate_or_loop() {
    // 64 slots claimed in a body that holds none.
    let mut w = Writer::new(MAGIC, VERSION);
    w.u32(64);
    assert_eq!(
        CarryOver::from_bytes(&w.finish()).unwrap_err(),
        BytesError::Truncated
    );
    // A text longer than the limit.
    let mut w = Writer::new(MAGIC, VERSION);
    w.u32(1);
    w.u8(0);
    w.u8(1);
    w.u32(1);
    w.u32(1);
    w.u8(1);
    w.u16(500);
    w.raw(&[b'a'; 500]);
    assert_eq!(
        CarryOver::from_bytes(&w.finish()).unwrap_err(),
        BytesError::Invalid("a text is too long")
    );
    // A text that is not UTF-8.
    let mut w = Writer::new(MAGIC, VERSION);
    w.u32(1);
    w.u8(0);
    w.u8(1);
    w.u32(1);
    w.u32(1);
    w.u8(1);
    w.u16(2);
    w.raw(&[0xFF, 0xFE]);
    assert_eq!(
        CarryOver::from_bytes(&w.finish()).unwrap_err(),
        BytesError::Invalid("a text is not UTF-8")
    );
}

#[test]
fn pseudo_random_garbage_never_panics() {
    let good = team().to_bytes();
    let mut x: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    for _ in 0..2_000 {
        let mut d = good.clone();
        for _ in 0..(next() % 4 + 1) {
            let i = (next() % d.len() as u64) as usize;
            d[i] = next() as u8;
        }
        if next() % 2 == 0 {
            reseal(&mut d);
        }
        let _ = CarryOver::from_bytes(&d);
        let n = (next() % 40) as usize;
        let junk: Vec<u8> = (0..n).map(|_| next() as u8).collect();
        let _ = CarryOver::from_bytes(&junk);
    }
}
