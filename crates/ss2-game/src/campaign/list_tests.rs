use super::*;

fn e(id: &str) -> LevelEntry {
    LevelEntry::new(id, "ep1", "NAME")
}

fn chain(n: usize) -> Vec<LevelEntry> {
    (0..n)
        .map(|i| {
            let en = e(&format!("l{i}"));
            if i + 1 == n {
                en.with_next(Next::End)
            } else {
                en
            }
        })
        .collect()
}

fn err(entries: Vec<LevelEntry>) -> ListError {
    LevelList::new(entries).unwrap_err()
}

fn ids(l: &LevelList) -> Vec<&str> {
    l.entries().iter().map(|x| x.id.as_str()).collect()
}

fn to(l: &LevelList, id: &str, x: u8) -> String {
    match l.destination(id, x).unwrap() {
        Destination::Level(t) => t.id.clone(),
        Destination::End => "END".to_string(),
    }
}

#[test]
fn a_linear_list_leads_from_each_level_to_the_next_and_ends_at_the_last() {
    let l = LevelList::new(chain(3)).unwrap();
    assert_eq!(l.len(), 3);
    assert!(!l.is_empty());
    assert_eq!(l.first().id, "l0");
    assert_eq!(l.main_path(), ["l0", "l1", "l2"]);
    assert_eq!(to(&l, "l0", 0), "l1");
    assert_eq!(to(&l, "l1", 0), "l2");
    assert_eq!(to(&l, "l2", 0), "END");
    assert_eq!(l.index_of("l1"), Some(1));
    assert_eq!(l.index_of("zz"), None);
    assert_eq!(l.get("l2").unwrap().id, "l2");
    assert!(l.get("zz").is_none());
}

#[test]
fn an_empty_list_is_refused() {
    assert_eq!(err(vec![]), ListError::Empty);
}

#[test]
fn one_level_is_a_campaign_when_it_ends() {
    let l = LevelList::new(chain(1)).unwrap();
    assert_eq!(l.main_path(), ["l0"]);
    assert_eq!(l.destination("l0", 0).unwrap(), Destination::End);
}

#[test]
fn one_level_that_leads_to_the_following_level_has_none() {
    assert_eq!(
        err(vec![e("only")]),
        ListError::NoFollowingLevel { id: "only".into() }
    );
}

#[test]
fn the_size_limit_is_exact() {
    let l = LevelList::new(chain(MAX_LEVELS)).unwrap();
    assert_eq!(l.len(), MAX_LEVELS);
    assert_eq!(l.main_path().len(), MAX_LEVELS);
    assert_eq!(err(chain(MAX_LEVELS + 1)), ListError::TooMany);
}

#[test]
fn ids_have_a_length_and_a_character_set() {
    let ok = |id: &str| LevelList::new(vec![e(id).with_next(Next::End)]).is_ok();
    assert!(ok("a"));
    assert!(ok("1_1_jungle"));
    assert!(ok("A-b.c_9"));
    assert!(ok(&"x".repeat(MAX_ID_LEN)));
    assert!(!ok(&"x".repeat(MAX_ID_LEN + 1)));
    assert!(!ok(""));
    assert!(!ok("a b"));
    assert!(!ok("a/b"));
    assert!(!ok("caf\u{e9}"));
    assert_eq!(
        err(vec![e("a b").with_next(Next::End)]),
        ListError::BadId { id: "a b".into() }
    );
}

#[test]
fn an_episode_id_and_a_link_target_must_be_valid_ids_too() {
    let bad_ep = LevelEntry::new("a", "ep one", "N").with_next(Next::End);
    assert_eq!(
        err(vec![bad_ep]),
        ListError::BadId {
            id: "ep one".into()
        }
    );
    let bad_next = e("a").with_next(Next::Level("no way".into()));
    assert_eq!(
        err(vec![bad_next]),
        ListError::BadId {
            id: "no way".into()
        }
    );
    let bad_alt = e("a").with_next(Next::End).with_alt_exit("");
    assert_eq!(err(vec![bad_alt]), ListError::BadId { id: String::new() });
}

#[test]
fn names_and_movies_have_a_length_and_no_control_characters() {
    let name = |n: &str| LevelList::new(vec![LevelEntry::new("a", "e", n).with_next(Next::End)]);
    assert!(name("").is_ok());
    assert!(name(&"n".repeat(MAX_TEXT_LEN)).is_ok());
    assert_eq!(
        name(&"n".repeat(MAX_TEXT_LEN + 1)).unwrap_err(),
        ListError::BadText {
            id: "a".into(),
            field: "name"
        }
    );
    assert!(name("two\nlines").is_err());
    assert!(name("tab\there").is_err());
    type With = fn(LevelEntry, &str) -> LevelEntry;
    let movie = |f: With, m: &str| LevelList::new(vec![f(e("a").with_next(Next::End), m)]);
    assert!(movie(LevelEntry::with_intro, "Movies/1.ogv").is_ok());
    assert!(movie(LevelEntry::with_intro, &"m".repeat(MAX_TEXT_LEN)).is_ok());
    let fields: [(With, &str); 3] = [
        (LevelEntry::with_intro, "intro movie"),
        (LevelEntry::with_outro, "outro movie"),
        (LevelEntry::with_episode_movie, "episode movie"),
    ];
    for (f, field) in fields {
        let expect = ListError::BadText {
            id: "a".into(),
            field,
        };
        assert_eq!(movie(f, "").unwrap_err(), expect);
        assert_eq!(movie(f, "a\nb").unwrap_err(), expect);
        assert_eq!(movie(f, &"m".repeat(MAX_TEXT_LEN + 1)).unwrap_err(), expect);
    }
}

#[test]
fn a_level_has_at_most_seven_alternative_exits() {
    let many = |n: usize| {
        let mut v = chain(2);
        for _ in 0..n {
            v[0] = v[0].clone().with_alt_exit("l1");
        }
        LevelList::new(v)
    };
    assert_eq!(
        many(MAX_ALT_EXITS).unwrap().get("l0").unwrap().exit_count(),
        8
    );
    assert_eq!(many(0).unwrap().get("l0").unwrap().exit_count(), 1);
    assert_eq!(
        many(MAX_ALT_EXITS + 1).unwrap_err(),
        ListError::TooManyExits { id: "l0".into() }
    );
}

#[test]
fn a_duplicate_id_is_refused() {
    let v = vec![e("a"), e("b"), e("a").with_next(Next::End)];
    assert_eq!(err(v), ListError::DuplicateId { id: "a".into() });
}

#[test]
fn a_link_to_a_level_that_is_not_listed_is_refused() {
    let main = vec![e("a").with_next(Next::Level("ghost".into()))];
    assert_eq!(
        err(main),
        ListError::DanglingNext {
            from: "a".into(),
            to: "ghost".into()
        }
    );
    let alt = vec![e("a").with_next(Next::End).with_alt_exit("ghost")];
    assert_eq!(
        err(alt),
        ListError::DanglingNext {
            from: "a".into(),
            to: "ghost".into()
        }
    );
}

#[test]
fn cycles_are_refused_wherever_they_are() {
    // A level that leads to itself.
    let me = vec![e("a").with_next(Next::Level("a".into()))];
    assert_eq!(err(me), ListError::Cycle { id: "a".into() });
    // a -> b -> a
    let two = vec![e("a"), e("b").with_next(Next::Level("a".into()))];
    assert_eq!(err(two), ListError::Cycle { id: "a".into() });
    // a -> b -> c -> b: the level reported is the one on the cycle that is met again.
    let tail = vec![e("a"), e("b"), e("c").with_next(Next::Level("b".into()))];
    assert_eq!(err(tail), ListError::Cycle { id: "b".into() });
    // A cycle made by an alternative exit.
    let alt = vec![e("a"), e("b").with_next(Next::End).with_alt_exit("a")];
    assert_eq!(err(alt), ListError::Cycle { id: "a".into() });
    // A long chain that closes on its first level (the search does not use the call stack).
    let mut long = chain(MAX_LEVELS);
    long[MAX_LEVELS - 1].next = Next::Level("l0".into());
    assert_eq!(err(long), ListError::Cycle { id: "l0".into() });
    // A cycle that nothing reaches is a cycle, not just an unreachable level.
    let lost = vec![
        e("a").with_next(Next::End),
        e("b").with_next(Next::Level("c".into())),
        e("c").with_next(Next::Level("b".into())),
    ];
    assert_eq!(err(lost), ListError::Cycle { id: "b".into() });
}

#[test]
fn two_exits_that_meet_again_are_not_a_cycle() {
    // a -> b or c; b -> d; c -> d; d ends.
    let l = LevelList::new(vec![
        e("a").with_alt_exit("c"),
        e("b").with_next(Next::Level("d".into())),
        e("c").with_next(Next::Level("d".into())),
        e("d").with_next(Next::End),
    ])
    .unwrap();
    assert_eq!(to(&l, "a", 0), "b");
    assert_eq!(to(&l, "a", 1), "c");
    assert_eq!(to(&l, "b", 0), "d");
    assert_eq!(to(&l, "c", 0), "d");
    assert_eq!(to(&l, "d", 0), "END");
    assert_eq!(l.main_path(), ["a", "b", "d"]);
}

#[test]
fn a_level_that_nothing_leads_to_is_refused_and_the_first_one_is_named() {
    let v = vec![
        e("a").with_next(Next::End),
        e("b").with_next(Next::End),
        e("c").with_next(Next::End),
    ];
    assert_eq!(err(v), ListError::Unreachable { id: "b".into() });
    // A level reached only by an alternative exit is reachable.
    let ok = vec![
        e("a").with_next(Next::End).with_alt_exit("b"),
        e("b").with_kind(LevelKind::Secret).with_next(Next::End),
    ];
    assert!(LevelList::new(ok).is_ok());
}

#[test]
fn a_secret_level_is_entered_by_an_alternative_exit_and_rejoins_the_main_path() {
    let l = LevelList::new(vec![
        e("a").with_alt_exit("s"),
        e("b").with_next(Next::End),
        e("s")
            .with_kind(LevelKind::Secret)
            .with_next(Next::Level("b".into())),
    ])
    .unwrap();
    assert_eq!(l.main_path(), ["a", "b"]);
    assert_eq!(l.get("s").unwrap().kind, LevelKind::Secret);
    assert_eq!(to(&l, "a", 1), "s");
    assert_eq!(to(&l, "s", 0), "b");
}

#[test]
fn the_main_exit_may_skip_ahead() {
    let l = LevelList::new(vec![
        e("a").with_next(Next::Level("c".into())).with_alt_exit("b"),
        e("b"),
        e("c").with_next(Next::End),
    ])
    .unwrap();
    assert_eq!(l.main_path(), ["a", "c"]);
}

#[test]
fn asking_about_a_level_or_exit_that_is_not_there_is_a_typed_error() {
    let l = LevelList::new(chain(2)).unwrap();
    assert_eq!(
        l.destination("zz", 0).unwrap_err(),
        ListError::UnknownLevel { id: "zz".into() }
    );
    assert_eq!(
        l.destination("l0", 1).unwrap_err(),
        ListError::NoSuchExit {
            id: "l0".into(),
            exit: 1
        }
    );
    assert_eq!(
        l.destination("l0", 255).unwrap_err(),
        ListError::NoSuchExit {
            id: "l0".into(),
            exit: 255
        }
    );
}

#[test]
fn destination_index_answers_for_a_position() {
    let l = LevelList::new(chain(2)).unwrap();
    assert_eq!(l.destination_index(0, 0), Some(Some(1)));
    assert_eq!(l.destination_index(1, 0), Some(None));
    assert_eq!(l.destination_index(1, 1), None);
    assert_eq!(l.destination_index(9, 0), None);
}

#[test]
fn the_fingerprint_follows_ids_order_and_exits_but_not_names() {
    let base = LevelList::new(chain(3)).unwrap();
    assert_eq!(
        base.fingerprint(),
        LevelList::new(chain(3)).unwrap().fingerprint()
    );
    // Names, movies and kinds do not change it.
    let mut v = chain(3);
    v[1] = LevelEntry::new("l1", "other", "A NEW NAME")
        .with_intro("x.ogv")
        .with_kind(LevelKind::Bonus);
    assert_eq!(LevelList::new(v).unwrap().fingerprint(), base.fingerprint());
    // An id does.
    let mut v = chain(3);
    v[1].id = "m1".into();
    assert_ne!(LevelList::new(v).unwrap().fingerprint(), base.fingerprint());
    // The order does.
    let mut v = chain(3);
    v.swap(0, 1);
    let swapped = LevelList::new(v).unwrap();
    assert_eq!(ids(&swapped), ["l1", "l0", "l2"]);
    assert_ne!(swapped.fingerprint(), base.fingerprint());
    // An exit does.
    let mut v = chain(3);
    v[0] = v[0].clone().with_alt_exit("l2");
    assert_ne!(LevelList::new(v).unwrap().fingerprint(), base.fingerprint());
    // Which exit leads where does: the same targets in the other order.
    let swap = |main: &str, alt: &str| {
        let mut v = chain(3);
        v[0] = v[0]
            .clone()
            .with_next(Next::Level(main.into()))
            .with_alt_exit(alt);
        LevelList::new(v).unwrap().fingerprint()
    };
    assert_ne!(swap("l1", "l2"), swap("l2", "l1"));
    // Two ids that would run together do not collide ("ab","c" against "a","bc").
    let one = LevelList::new(vec![e("ab"), e("c").with_next(Next::End)]).unwrap();
    let two = LevelList::new(vec![e("a"), e("bc").with_next(Next::End)]).unwrap();
    assert_ne!(one.fingerprint(), two.fingerprint());
}

#[test]
fn the_fingerprint_of_a_known_list_is_pinned() {
    // Pinned so that the format of the fingerprint cannot change unnoticed: a saved note depends on it.
    let l = LevelList::new(chain(2)).unwrap();
    // "l0" 0x00 1 exit -> entry 1; "l1" 0x00 1 exit -> End.
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"l0\0\x01\x01\x00");
    bytes.extend_from_slice(b"l1\0\x01\xff\xff");
    assert_eq!(l.fingerprint(), crate::level_progress::crc32(&bytes));
}

#[test]
fn defaults_and_builders() {
    let en = e("a");
    assert_eq!(en.kind, LevelKind::Main);
    assert_eq!(LevelKind::default(), LevelKind::Main);
    assert_eq!(en.next, Next::Following);
    assert_eq!(Next::default(), Next::Following);
    assert!(en.alt_exits.is_empty());
    assert!(en.intro_movie.is_none() && en.outro_movie.is_none() && en.episode_movie.is_none());
    assert_eq!(en.exit_count(), 1);
    let full = e("a")
        .with_intro("i")
        .with_outro("o")
        .with_episode_movie("p")
        .with_alt_exit("b");
    assert_eq!(full.intro_movie.as_deref(), Some("i"));
    assert_eq!(full.outro_movie.as_deref(), Some("o"));
    assert_eq!(full.episode_movie.as_deref(), Some("p"));
    assert_eq!(full.exit_count(), 2);
}

#[test]
fn every_error_says_what_is_wrong() {
    let all = [
        (ListError::Empty, "empty"),
        (ListError::TooMany, "more than"),
        (ListError::BadId { id: "x".into() }, "`x`"),
        (
            ListError::BadText {
                id: "a".into(),
                field: "name",
            },
            "name",
        ),
        (ListError::TooManyExits { id: "a".into() }, "too many exits"),
        (ListError::DuplicateId { id: "a".into() }, "twice"),
        (
            ListError::DanglingNext {
                from: "a".into(),
                to: "b".into(),
            },
            "`b`",
        ),
        (ListError::NoFollowingLevel { id: "a".into() }, "last"),
        (ListError::Cycle { id: "a".into() }, "cycle"),
        (ListError::Unreachable { id: "a".into() }, "reached"),
        (ListError::UnknownLevel { id: "a".into() }, "no level"),
        (
            ListError::NoSuchExit {
                id: "a".into(),
                exit: 3,
            },
            "exit 3",
        ),
    ];
    for (e, needle) in all {
        let text = e.to_string();
        assert!(text.contains(needle), "{text:?} should contain {needle:?}");
    }
}
