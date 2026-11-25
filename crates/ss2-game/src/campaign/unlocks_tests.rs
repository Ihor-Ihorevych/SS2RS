use super::*;
use crate::campaign::list::{LevelEntry, Next};
use crate::level_progress::{LevelProgress, LevelSetup};

fn e(id: &str) -> LevelEntry {
    LevelEntry::new(id, "ep", "N")
}

/// a -> b -> c -> end; b has a secret exit to s (a secret level that rejoins at c).
fn list() -> LevelList {
    LevelList::new(vec![
        e("a"),
        e("b").with_alt_exit("s"),
        e("c").with_next(Next::End),
        e("s")
            .with_kind(LevelKind::Secret)
            .with_next(Next::Level("c".into())),
    ])
    .unwrap()
}

fn result(level: &str, exit: u8, found: u32, total: u32) -> LevelResult {
    LevelResult {
        level: level.into(),
        exit,
        secrets_found: found,
        secrets_total: total,
    }
}

fn rule(when: Trigger, grants: Vec<Unlock>) -> UnlockRule {
    UnlockRule { when, grants }
}

fn rules() -> UnlockRules {
    UnlockRules::new(
        vec![
            rule(
                Trigger::LevelFinished { level: "a".into() },
                vec![Unlock::cheat("god")],
            ),
            rule(
                Trigger::SecretsFound {
                    level: "b".into(),
                    at_least: 2,
                },
                vec![Unlock::mode("survival")],
            ),
            rule(
                Trigger::AllSecrets { level: "b".into() },
                vec![Unlock::cheat("fly"), Unlock::cheat("god")],
            ),
            rule(Trigger::CampaignComplete, vec![Unlock::mode("serious")]),
        ],
        &list(),
    )
    .unwrap()
}

// ---- unlocks_after -------------------------------------------------------------------------------------------

#[test]
fn finishing_a_level_unlocks_where_its_exit_leads_and_nothing_with_no_rules() {
    let none = UnlockRules::none();
    let l = list();
    assert_eq!(
        unlocks_after(&none, &l, &result("a", 0, 0, 0)).unwrap(),
        [Unlock::level("b")]
    );
    assert_eq!(
        unlocks_after(&none, &l, &result("b", 0, 0, 0)).unwrap(),
        [Unlock::level("c")]
    );
    // The secret exit unlocks the secret level.
    assert_eq!(
        unlocks_after(&none, &l, &result("b", 1, 0, 0)).unwrap(),
        [Unlock::level("s")]
    );
    // The last level leads nowhere: nothing to unlock.
    assert_eq!(unlocks_after(&none, &l, &result("c", 0, 0, 0)).unwrap(), []);
}

#[test]
fn a_level_finished_rule_fires_for_that_level_only() {
    let l = list();
    let r = rules();
    let got = unlocks_after(&r, &l, &result("a", 0, 0, 0)).unwrap();
    assert_eq!(got, [Unlock::level("b"), Unlock::cheat("god")]);
    let got = unlocks_after(&r, &l, &result("c", 0, 0, 0)).unwrap();
    assert!(!got.contains(&Unlock::cheat("god")));
}

#[test]
fn a_secrets_threshold_is_met_at_the_count_and_above() {
    let l = list();
    let r = rules();
    let has = |found, total| {
        unlocks_after(&r, &l, &result("b", 0, found, total))
            .unwrap()
            .contains(&Unlock::mode("survival"))
    };
    assert!(!has(0, 5));
    assert!(!has(1, 5));
    assert!(has(2, 5));
    assert!(has(3, 5));
    // The same count on another level does nothing.
    assert!(
        !unlocks_after(&r, &l, &result("a", 0, 5, 5))
            .unwrap()
            .contains(&Unlock::mode("survival"))
    );
}

#[test]
fn all_secrets_needs_every_secret_and_at_least_one() {
    let l = list();
    let r = rules();
    let fly = |found, total| {
        unlocks_after(&r, &l, &result("b", 0, found, total))
            .unwrap()
            .contains(&Unlock::cheat("fly"))
    };
    assert!(!fly(2, 3));
    assert!(fly(3, 3));
    // A level with no secrets has all of them found, and that unlocks nothing.
    assert!(!fly(0, 0));
}

#[test]
fn campaign_complete_needs_a_main_level_leaving_through_an_exit_that_ends() {
    let l = list();
    let r = rules();
    let serious = |level: &str, exit| {
        unlocks_after(&r, &l, &result(level, exit, 0, 0))
            .unwrap()
            .contains(&Unlock::mode("serious"))
    };
    assert!(serious("c", 0));
    assert!(!serious("b", 0));
    assert!(!serious("a", 0));
    // A secret level that ends the campaign does not complete it.
    let ends_in_secret = LevelList::new(vec![
        e("a").with_alt_exit("s").with_next(Next::End),
        e("s").with_kind(LevelKind::Secret).with_next(Next::End),
    ])
    .unwrap();
    let rr = UnlockRules::new(
        vec![rule(
            Trigger::CampaignComplete,
            vec![Unlock::mode("serious")],
        )],
        &ends_in_secret,
    )
    .unwrap();
    let done = |level: &str, exit| {
        unlocks_after(&rr, &ends_in_secret, &result(level, exit, 0, 0))
            .unwrap()
            .contains(&Unlock::mode("serious"))
    };
    assert!(done("a", 0));
    assert!(!done("s", 0));
    // A bonus level does not complete it either.
    let bonus = LevelList::new(vec![
        e("a").with_alt_exit("x").with_next(Next::End),
        e("x").with_kind(LevelKind::Bonus).with_next(Next::End),
    ])
    .unwrap();
    let rb = UnlockRules::new(
        vec![rule(
            Trigger::CampaignComplete,
            vec![Unlock::mode("serious")],
        )],
        &bonus,
    )
    .unwrap();
    assert!(
        unlocks_after(&rb, &bonus, &result("a", 0, 0, 0))
            .unwrap()
            .contains(&Unlock::mode("serious"))
    );
    assert!(
        unlocks_after(&rb, &bonus, &result("x", 0, 0, 0))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn the_result_is_in_a_fixed_order_without_repeats() {
    let l = list();
    let r = rules();
    // Level a finished twice over would not matter; here b with every secret: exit target, then rules in table order, the repeated cheat once.
    let got = unlocks_after(&r, &l, &result("b", 0, 3, 3)).unwrap();
    assert_eq!(
        got,
        [
            Unlock::level("c"),
            Unlock::mode("survival"),
            Unlock::cheat("fly"),
            Unlock::cheat("god"),
        ]
    );
    // Deterministic.
    assert_eq!(got, unlocks_after(&r, &l, &result("b", 0, 3, 3)).unwrap());
}

#[test]
fn a_rule_that_grants_the_exit_target_does_not_repeat_it() {
    let l = list();
    let r = UnlockRules::new(
        vec![rule(
            Trigger::LevelFinished { level: "a".into() },
            vec![Unlock::level("b"), Unlock::cheat("x")],
        )],
        &l,
    )
    .unwrap();
    assert_eq!(
        unlocks_after(&r, &l, &result("a", 0, 0, 0)).unwrap(),
        [Unlock::level("b"), Unlock::cheat("x")]
    );
}

#[test]
fn an_impossible_result_or_an_unknown_level_or_exit_is_refused() {
    let l = list();
    let r = UnlockRules::none();
    assert_eq!(
        unlocks_after(&r, &l, &result("a", 0, 4, 3)).unwrap_err(),
        UnlockError::BadResult
    );
    assert!(unlocks_after(&r, &l, &result("a", 0, 3, 3)).is_ok());
    assert_eq!(
        unlocks_after(&r, &l, &result("zz", 0, 0, 0)).unwrap_err(),
        UnlockError::List(ListError::UnknownLevel { id: "zz".into() })
    );
    assert_eq!(
        unlocks_after(&r, &l, &result("a", 1, 0, 0)).unwrap_err(),
        UnlockError::List(ListError::NoSuchExit {
            id: "a".into(),
            exit: 1
        })
    );
}

#[test]
fn a_result_is_built_from_a_level_summary() {
    let mut p = LevelProgress::new(LevelSetup {
        secrets: 3,
        ..LevelSetup::default()
    })
    .unwrap();
    p.find_secret(0, Some(0)).unwrap();
    p.find_secret(2, None).unwrap();
    let s = p.summary(30).unwrap();
    assert_eq!(LevelResult::from_summary("b", 1, &s), result("b", 1, 2, 3));
}

// ---- the rule table ------------------------------------------------------------------------------------------

#[test]
fn a_rule_table_is_checked_against_the_list() {
    let l = list();
    let ok = |r: UnlockRule| UnlockRules::new(vec![r], &l);
    let g = vec![Unlock::cheat("x")];
    assert!(ok(rule(Trigger::CampaignComplete, g.clone())).is_ok());
    assert_eq!(
        ok(rule(
            Trigger::LevelFinished { level: "zz".into() },
            g.clone()
        ))
        .unwrap_err(),
        UnlockError::UnknownLevel { id: "zz".into() }
    );
    assert_eq!(
        ok(rule(Trigger::AllSecrets { level: "zz".into() }, g.clone())).unwrap_err(),
        UnlockError::UnknownLevel { id: "zz".into() }
    );
    assert_eq!(
        ok(rule(
            Trigger::SecretsFound {
                level: "zz".into(),
                at_least: 1
            },
            g.clone()
        ))
        .unwrap_err(),
        UnlockError::UnknownLevel { id: "zz".into() }
    );
    assert_eq!(
        ok(rule(
            Trigger::SecretsFound {
                level: "a".into(),
                at_least: 0
            },
            g.clone()
        ))
        .unwrap_err(),
        UnlockError::ZeroThreshold
    );
    assert_eq!(
        ok(rule(
            Trigger::LevelFinished {
                level: "bad id".into()
            },
            g.clone()
        ))
        .unwrap_err(),
        UnlockError::BadId {
            id: "bad id".into()
        }
    );
    assert_eq!(
        ok(rule(Trigger::CampaignComplete, vec![])).unwrap_err(),
        UnlockError::NoGrants
    );
    assert_eq!(
        ok(rule(Trigger::CampaignComplete, vec![Unlock::level("zz")])).unwrap_err(),
        UnlockError::UnknownLevel { id: "zz".into() }
    );
    assert_eq!(
        ok(rule(Trigger::CampaignComplete, vec![Unlock::cheat("a b")])).unwrap_err(),
        UnlockError::BadId { id: "a b".into() }
    );
    // A cheat or a mode may have any valid id, even one that is not a level.
    assert!(ok(rule(Trigger::CampaignComplete, vec![Unlock::mode("zz")])).is_ok());
}

#[test]
fn the_rule_and_grant_limits_are_exact() {
    let l = list();
    let one = |n: usize| {
        (0..n)
            .map(|i| Unlock::cheat(&format!("c{i}")))
            .collect::<Vec<_>>()
    };
    let r = |g: Vec<Unlock>| rule(Trigger::CampaignComplete, g);
    assert!(UnlockRules::new(vec![r(one(MAX_GRANTS))], &l).is_ok());
    assert_eq!(
        UnlockRules::new(vec![r(one(MAX_GRANTS + 1))], &l).unwrap_err(),
        UnlockError::TooManyGrants
    );
    let many = |n: usize| (0..n).map(|_| r(one(1))).collect::<Vec<_>>();
    let ok = UnlockRules::new(many(MAX_RULES), &l).unwrap();
    assert_eq!(ok.rules().len(), MAX_RULES);
    assert_eq!(
        UnlockRules::new(many(MAX_RULES + 1), &l).unwrap_err(),
        UnlockError::TooManyRules
    );
    assert!(UnlockRules::none().rules().is_empty());
}

// ---- the set -------------------------------------------------------------------------------------------------

#[test]
fn a_new_campaign_has_its_first_level_unlocked() {
    let s = UnlockSet::initial(&list());
    assert!(s.level_unlocked("a"));
    assert!(!s.level_unlocked("b"));
    assert_eq!(s.len(), 1);
    assert!(!s.is_empty());
    assert!(UnlockSet::new().is_empty());
    assert!(s.contains(&Unlock::level("a")));
    assert!(!s.contains(&Unlock::cheat("a")));
}

#[test]
fn granting_returns_only_what_is_new() {
    let mut s = UnlockSet::new();
    let first = s
        .grant([Unlock::level("a"), Unlock::cheat("god"), Unlock::level("a")])
        .unwrap();
    assert_eq!(first, [Unlock::level("a"), Unlock::cheat("god")]);
    let again = s.grant([Unlock::cheat("god"), Unlock::mode("m")]).unwrap();
    assert_eq!(again, [Unlock::mode("m")]);
    assert_eq!(s.len(), 3);
    assert_eq!(s.grant([]).unwrap(), []);
    // Iterated in order: kind, then id.
    let all: Vec<_> = s.iter().cloned().collect();
    assert_eq!(
        all,
        [Unlock::level("a"), Unlock::cheat("god"), Unlock::mode("m")]
    );
}

#[test]
fn the_set_has_a_limit_and_a_refused_grant_changes_nothing() {
    let mut s = UnlockSet::new();
    let items = |from: usize, n: usize| (from..from + n).map(|i| Unlock::cheat(&format!("c{i}")));
    s.grant(items(0, MAX_UNLOCKS)).unwrap();
    assert_eq!(s.len(), MAX_UNLOCKS);
    // Nothing new at the limit is fine; one new is refused.
    assert_eq!(s.grant(items(0, 3)).unwrap(), []);
    let before = s.clone();
    assert_eq!(
        s.grant(items(MAX_UNLOCKS, 1)).unwrap_err(),
        UnlockError::TooManyUnlocks
    );
    assert_eq!(s, before);
    // A bad id in the middle changes nothing.
    let mut t = UnlockSet::new();
    assert_eq!(
        t.grant([Unlock::cheat("ok"), Unlock::cheat("not ok")])
            .unwrap_err(),
        UnlockError::BadId {
            id: "not ok".into()
        }
    );
    assert!(t.is_empty());
}

#[test]
fn errors_say_what_is_wrong() {
    let all = [
        (UnlockError::UnknownLevel { id: "q".into() }, "`q`"),
        (UnlockError::BadId { id: "q q".into() }, "`q q`"),
        (UnlockError::NoGrants, "grants nothing"),
        (UnlockError::TooManyGrants, "16"),
        (UnlockError::TooManyRules, "256"),
        (UnlockError::ZeroThreshold, "zero"),
        (UnlockError::TooManyUnlocks, "4096"),
        (UnlockError::BadResult, "secrets"),
        (UnlockError::List(ListError::Empty), "empty"),
    ];
    for (e, needle) in all {
        assert!(e.to_string().contains(needle), "{e}");
    }
}

// ---- bytes ---------------------------------------------------------------------------------------------------

fn busy() -> UnlockSet {
    let mut s = UnlockSet::initial(&list());
    s.grant([
        Unlock::level("b"),
        Unlock::cheat("god"),
        Unlock::cheat("fly"),
        Unlock::mode("survival"),
    ])
    .unwrap();
    s
}

fn reseal(bytes: &mut [u8]) {
    let n = bytes.len() - 4;
    let crc = crate::level_progress::crc32(&bytes[..n]);
    bytes[n..].copy_from_slice(&crc.to_le_bytes());
}

#[test]
fn a_set_round_trips_and_gives_the_same_bytes_whatever_the_order_of_granting() {
    let s = busy();
    let b = s.to_bytes();
    assert_eq!(UnlockSet::from_bytes(&b).unwrap(), s);
    let mut t = UnlockSet::new();
    t.grant([
        Unlock::mode("survival"),
        Unlock::cheat("fly"),
        Unlock::level("b"),
        Unlock::cheat("god"),
        Unlock::level("a"),
    ])
    .unwrap();
    assert_eq!(t.to_bytes(), b);
    assert_eq!(&b[..4], b"S2UL");
    let empty = UnlockSet::new();
    assert_eq!(UnlockSet::from_bytes(&empty.to_bytes()).unwrap(), empty);
    // The largest set.
    let mut big = UnlockSet::new();
    big.grant((0..MAX_UNLOCKS).map(|i| Unlock::mode(&format!("m{i}"))))
        .unwrap();
    assert_eq!(UnlockSet::from_bytes(&big.to_bytes()).unwrap(), big);
}

#[test]
fn every_prefix_and_every_flipped_byte_is_refused() {
    let b = busy().to_bytes();
    for n in 0..b.len() {
        assert!(UnlockSet::from_bytes(&b[..n]).is_err(), "prefix {n}");
    }
    for i in 0..b.len() {
        for bit in [0x01u8, 0x40] {
            let mut d = b.clone();
            d[i] ^= bit;
            assert!(UnlockSet::from_bytes(&d).is_err(), "byte {i}");
        }
    }
}

#[test]
fn damage_under_a_valid_checksum_is_refused_with_the_right_error() {
    let good = busy().to_bytes();
    let fix = |f: &dyn Fn(&mut Vec<u8>)| {
        let mut d = good.clone();
        f(&mut d);
        reseal(&mut d);
        UnlockSet::from_bytes(&d)
    };
    assert_eq!(fix(&|d| d[1] = b'X').unwrap_err(), BytesError::WrongMagic);
    assert_eq!(
        fix(&|d| d[4] = 9).unwrap_err(),
        BytesError::UnknownVersion { found: 9 }
    );
    // 6 header, count u32 (6..10), first unlock: kind (10), length u16 (11..13), id.
    assert_eq!(
        fix(&|d| d[10] = 7).unwrap_err(),
        BytesError::Invalid("unknown unlock kind")
    );
    assert_eq!(
        fix(&|d| d[6..10].copy_from_slice(&(MAX_UNLOCKS as u32 + 1).to_le_bytes())).unwrap_err(),
        BytesError::Invalid("a count is over its limit")
    );
    assert_eq!(
        fix(&|d| d[6..10].copy_from_slice(&1000u32.to_le_bytes())).unwrap_err(),
        BytesError::Truncated
    );
    assert_eq!(
        fix(&|d| {
            let n = d.len() - 4;
            d.insert(n, 0);
        })
        .unwrap_err(),
        BytesError::Trailing
    );
    assert_eq!(
        fix(&|d| d[13] = b' ').unwrap_err(),
        BytesError::Invalid("an unlock id is not valid")
    );
}

#[test]
fn unlocks_out_of_order_or_repeated_are_refused() {
    let build = |a: Unlock, b: Unlock| {
        let mut w = Writer::new(MAGIC, VERSION);
        w.u32(2);
        for u in [a, b] {
            w.u8(u.kind.tag());
            w.str(&u.id);
        }
        UnlockSet::from_bytes(&w.finish())
    };
    assert!(build(Unlock::level("a"), Unlock::level("b")).is_ok());
    assert!(build(Unlock::level("a"), Unlock::cheat("a")).is_ok());
    let order = BytesError::Invalid("unlocks out of order or repeated");
    assert_eq!(
        build(Unlock::level("b"), Unlock::level("a")).unwrap_err(),
        order
    );
    assert_eq!(
        build(Unlock::level("a"), Unlock::level("a")).unwrap_err(),
        order
    );
    assert_eq!(
        build(Unlock::cheat("a"), Unlock::level("z")).unwrap_err(),
        order
    );
}

#[test]
fn a_text_over_the_id_limit_is_refused() {
    let mut w = Writer::new(MAGIC, VERSION);
    w.u32(1);
    w.u8(0);
    w.u16(65);
    w.raw(&[b'a'; 65]);
    assert_eq!(
        UnlockSet::from_bytes(&w.finish()).unwrap_err(),
        BytesError::Invalid("a text is too long")
    );
}

#[test]
fn pseudo_random_garbage_never_panics() {
    let good = busy().to_bytes();
    let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
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
        let _ = UnlockSet::from_bytes(&d);
    }
}
